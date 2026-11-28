PASS

# v0.4.1 pricing, cloud tasks T1a, T1, T2, T3: task review

Reviewer, 2026-11-21. Branch `v041/t01-cloud`, base `8c78e1f8`, eight commits `bb8235c0..aa96fb8b`
(tags `v041-red/t1a` through `v041-green/t03`). Read against the brief, the engineer's report, the review
package, the design note (`docs/specs/2026-11-19-v041-pricing-design.md`; section 1, D1-D14, sections 5 and
6) and the Stripe facts report (Q3-Q11). I read the final files whole, the red commits' scaffolding, and
the one test changed in a green commit. I did not re-run the suite; the report carries the runs.

**Result: no Critical or Important findings.** There are seven Minor findings: five against the code
and two that are gaps in the note, listed under "For Quinn".

## What I checked and found holding

- **Scope.** Only the twelve files the brief assigns changed, all under `cloud/supabase/functions/`.
  Nothing changed under `engine/` or `app/`, nothing on the contract list, and `_shared/stripe.ts` is
  untouched. All twelve files are LF (`git ls-files --eol`). No telemetry is added. Test data uses
  `example.invalid`, invented ids and a documentation IP range.
- **One source.** `1299` and `8437` appear only in `_shared/price_catalog.ts` among non-test files. No
  handler test imports `SELLING`, `KEPT` or `CATALOG`: each builds its own table through
  `resolveCatalog`. Only `the_catalog_sells_two_plans_yearly_first` reads the real selling amounts.
  `the_variable_names_are_derived`, `the_formatter_…` and `the_yearly_amount_…` write 1299 and 8437 as
  literals, because the task text requires it, but they never read `SELLING`, so a price change breaks
  none of them. The source scan applies the five rules the task names to all four folders. It checks
  its own rules on text it supplies first, and it has no bare-cents rule.
- **Red first.** Each red commit holds the tests plus the scaffolding the report names (a types-only
  stub for T1a; optional `Deps` fields and stubs for T1-T3). `index.ts` is untouched at each red tag.
  The T2 and T3 green commits change no test. The T1 green commit changes one assertion (the `fail(NNN,`
  exclusion in the `index.ts` text-read), and the report and the commit message both give the reason,
  which is the note's own r5 m3 reasoning.
- **T1 order.** The order is method, user, parse, D3 (old names 409, then unknown 400, then the missing
  or wrong-typed fields 400, then the equality 409), account, 18+, tester, D5, D4, customer, consent,
  session (`billing-checkout/handler.ts:159-257`). Every refusal comes before the customer, the consent
  row and the session. The D3 refusals come before any read, and the tests pin `seen.calls` to `[]`.
  D4 ties the interval and count to the key's cadence (`priceProblems`, `:146-156`), so the two
  cross-cadence cases fail on `interval`. The log names the failing fields only.
- **T1 form.** `managed_payments[enabled]=true` is added. Exactly `automatic_tax[enabled]` and
  `customer_update[address]` left, and the test computes that against v0.4.0's key list. Every D7
  load-bearing key is present. The trial key is sent only when no earlier subscription id exists.
  `payment_method_collection` is `always` in both cases. No version key is added. D8's four sentences
  are word for word, and the tax and money-back words come from the catalog.
- **T2.** The plan comes from `lookupPrice` across the selling and kept entries. An unset variable gives
  `priceId: null`, which matches nothing, and nothing can throw on it. `trial_will_end` runs after the
  signature, duplicate, account-id and account-exists checks. It never reaches the stale guard or
  `writeEntitlement`. A send failure answers 502 and records nothing. The log line is
  `stripe-webhook: <account id>: …` and never carries the error text. `chargeDate` shifts by −10 h
  before taking the UTC date, which is correct at the 10:00 UTC edge (the test pins 09:59:59 and
  10:00:00). `index.ts` resolves through `Deno.env.get`, never through `env()`.
- **T3.** The pause code and its tests are gone, and the reply is `{ reminded, moved, failed }`. The move
  goes notice, then mark (form key `metadata[knowlu_price_notice]` only), then change (`items[0][id]`,
  `items[0][price]`, `proration_behavior: none`). The window `N×24h ≤ end − now < (N+6)×24h` is applied
  to Stripe's own period end (`:84-87, 156-157`). The view margin is `[(N−1)d, (N+7)d)` (`:91-98`). On a
  raise a trial is skipped, and on a cut it is moved, decided by comparing cents for the same key
  (`:162`). A failed notice throws before either write. A matching mark skips the notice and the mark,
  and a mark for another renewal does not. Unknown prices and failed reads count in `failed` and are
  never mailed. The move runs before the reminder, so the reminder states the new amount.
- **Tests that would pass with wrong code.** I looked for them and found none that matter:
  - Reminding from the plan column instead of the subscription read would pass
    `the_annual_reminder_states_the_subscribers_own_price` alone. `a_kept_price_is_reminded_at_its_own_amount`
    and `an_unknown_price_is_never_mailed` catch it.
  - A "skip every trial" rule passes the raise test, and `a_cut_moves_a_trial` catches it.
  - A list of allowed pairs in D4 is caught by the cross-cadence cases.
  - Taking the reminder's amount from the event's `unit_amount` is caught by the 1399 case.
- **The engineer's rulings,** checked against the note. Fourteen agree with it or fill room it leaves:
  - checkout: `TRIAL_DAYS` as a handler constant, the offer answered while a selling variable is unset,
    and the price check logging field names only;
  - the catalog: the words held in `CatalogDef`, "every year" for an unknown year × 1 price, and
    `unknownPriceText` returning `null`;
  - the webhook: deciding from the event's own object, a missing address recorded and answered 200,
    and `PLAN_WORDS` as cadence words, which D13 allows;
  - billing-jobs: the mark carrying the target price id, the status filter (`active`, and `trialing` on
    a cut), which agrees with the `billing_subscribers` view's own filter (`20260910000200_billing_jobs.sql:33`),
    the encoded path, the `moved` count, and the reminder-due trick in `the_window_and_the_date_are_stripes`.

  Two contradict the note's text, findings 1 and 2.

## Findings

### 1. Minor, T1: a wrong-typed offer field answers 400 where the note says 409

- **Where:** `billing-checkout/handler.ts:174` and `:178`. This is the engineer's third T1 ruling.
- **The note:** D3 (`spec:148-150`) says `amount_cents` and `terms_version` are "required (400 if either
  is absent) and must equal the catalog's … **Anything else is refused with 409**".
- **The code:** a present but wrong-typed value (`"1299"`, `12.5`, `20260910`) gets 400 "amount_cents
  must be a whole number of cents" or "terms_version must be a string".
- **Failure scenario:** a build, or a later app change, that serialises the offer's cents as a string
  sends `"1299"`. The student sees a 400 sentence about cents instead of D3's "prices or terms have
  changed", and the app does not re-ask for the offer as D9 prescribes for the 409. No money moves,
  and the refusal still names a reason.
- **Smallest fix:** drop the two type checks, so that a present value that is not `=== entry.cents` or
  `=== termsVersion` falls through to the 409. Move the two type cases in
  `an_offer_with_a_missing_field_is_400` to `a_changed_offer_is_refused`. Or Quinn accepts the ruling,
  and the note's D3 says so.

### 2. Minor, T1: an unreadable Stripe price answers 502, where D4 says every failure is 500

- **Where:** `billing-checkout/handler.ts:213-217`. This is the engineer's fourth T1 ruling.
- **The note:** D4 (`spec:189`) says "Any failure answers 500 'the function is not configured'".
- **The code:** `getPrice` returning `null` (any non-2xx from Stripe, including a 404 for a price id
  from the other Stripe mode or a deleted price) answers 502 "the payment provider refused the request".
- **Failure scenario:** after a price change, Quinn sets the new variable to a test-mode id on the live
  project. Every purchase of that plan answers 502, which reads as a passing Stripe outage, not a
  configuration fault. The app's Try again loops on a cause no retry fixes. Nothing is written, so no
  money is at risk.
- **Smallest fix:** answer `fail(500, NOT_CONFIGURED)` on `null`. The log already names the plan. Or
  keep 502 and record the ruling in D4.

### 3. Minor, T2: a reminder can go twice when recording the event fails after a successful send

- **Where:** `stripe-webhook/handler.ts:288-296`.
- **Failure scenario:** `sendEmail` succeeds, then `recordEvent` (a PostgREST upsert) throws on a
  transient database error. `asResponse` answers 500, Stripe retries, `seenEvent` is false, and the
  student gets a second "Your Knowlu free trial is ending". Two deliveries of the same event arriving
  together have the same effect, since both pass `seenEvent` before either records. Nothing is
  charged, but the brief's "goes once" fails.
- **Smallest fix:** after a successful send, wrap `recordEvent` in `try/catch`. On failure, log
  `stripe-webhook: <account id>: the reminder was sent but not recorded` and still answer 200, so
  Stripe does not retry. Pin it with one test in which `recordEvent` rejects after a send: 200, and one
  mail. The concurrent case stays as a known limit unless the email provider takes an idempotency key
  (cannot verify).
- **The same class in T3, no fix asked:** if the notice is sent and the mark write
  (`billing-jobs/handler.ts:171`) fails, the next day sends the notice again. That is the safe
  direction under "notice first", and D14 allows it.

### 4. Minor, T3: an unset selling variable fails a trialing subscriber that the raise rule should skip

- **Where:** `billing-jobs/handler.ts:158-162`.
- **Failure scenario:** a future catalog has a `KEPT` `move` entry on a raise, and the key's new selling
  variable is not yet set on the project. A `trialing` subscriber on the old price, inside the window,
  reaches `:159`, which throws "no selling price" before `:162` can skip the trial. The subscriber is
  logged and counted in `failed` every day of the window, although D14 says the move leaves them alone
  until they are active. No mail goes and nothing is written, so no money is affected. The run's
  `failed` count misreports, and that is what Quinn watches after a price change.
- **Smallest fix:** take `selling = sellingFor(...)` first. If `selling` exists, the subscription is
  trialing and `selling.cents > kept.cents`, return `null`. Only then throw on a missing `priceId`.

### 5. Minor, T3: billing-jobs now bundles the webhook's handler

- **Where:** `billing-jobs/handler.ts:31`, `import { chargeDate } from "../stripe-webhook/handler.ts"`.
  This is the engineer's fifth T3 ruling.
- **Failure scenario:** a later change to `stripe-webhook/handler.ts` is deployed to `stripe-webhook`
  only. `billing-jobs` keeps running the older copy, including any change to the UTC−10 rule. The
  two then state different dates for the same renewal (the trial reminder versus the price notice).
  `mail/` already sets the precedent of importing across folders, so this is a deploy-list risk, not
  a code defect.
- **Smallest fix:** add a line to the lane's deploy steps (and to `cloud/supabase/README.md` when
  docs-keeper next edits it): "a change to `stripe-webhook/handler.ts` redeploys `billing-jobs` too".
  Move `chargeDate` to `_shared/` in a later small change; that file is not T3's to edit.

## For Quinn (gaps in the note; the code does what the note says)

### 6. Minor, T1 / note D5: the one-live-subscription check cannot stop two checkouts opened together

- **Where:** `billing-checkout/handler.ts:201-206`, against the note's reason at `spec:210`.
- **The gap:** the note says D5 stops "two checkouts in parallel". But the `entitlements` row gets a
  subscription id only when the webhook sees a completed checkout.
- **Failure scenario:** a student double-clicks Subscribe, or opens checkout twice, before completing
  either. Both POSTs read no row, and both sessions carry 14 days free. If both are completed, the
  account has two trialing subscriptions. The webhook keeps only the later id. Account deletion
  cancels only that one, so the other bills a deleted account after its trial. The trial reminder
  goes for both.
- **Recommendation:** before creating a session, list the customer's open Checkout Sessions and expire
  them (one GET and one POST per open session). This is a small change inside T1's files, and it needs
  a test. Otherwise, write the window into D5's "Known limit".

### 7. Minor, T2 / note D6: the reminder states the list price even when a promotion code discounts it

- **Where:** `stripe-webhook/handler.ts:102-112`, with `allow_promotion_codes=true` kept at
  `billing-checkout/handler.ts:106`.
- **Failure scenario:** a student subscribes with a promotion code that takes 50% off the first
  payment. Their reminder says "$84.37 once a year", but Stripe charges about half that. A 100% code
  (the note's checklist deactivates the live one, `spec:867`) would be told a charge that never comes.
  The statement errs high, never low, so no student is charged more than told. But D6's "the amount"
  is not what is charged.
- **Recommendation:** either state the discount when `subscription.discounts` is non-empty (or omit the
  figure and point to Manage subscription), or drop `allow_promotion_codes`. D7 lets checkpoint A drop
  it, and P10 makes comp grants the comp mechanism. The smaller decision is to drop the key.

## Cannot verify

- **The runs.** The suite results, and the claim that at each `v041-red` tag only that task's tests
  fail. I did not re-run, per the dispatch. The red scaffolding I read is consistent with the claim.
- **Stripe behaviour (S1, S2):**
  - that `managed_payments[enabled]=true` is accepted at the `2025-03-31.basil` pin (facts Q3: the
    guide and the changelog disagree);
  - that `customer.subscription.*` and `trial_will_end` events, carrying `metadata.account_id`, reach the
    endpoint under Managed Payments (Q5, Q6);
  - that a no-proration price change on a trialing subscription bills the new price at the trial's end
    (Q9);
  - that a metadata-only write keeps `account_id` under Managed Payments (Q11).
- **The Stripe endpoint's event list** must gain `customer.subscription.trial_will_end` in each mode, as
  the report says. Otherwise the reminder never goes, and no test can see that.
- **The deploy order for the webhook.** Every function resolves the catalog once per cold start. If the
  new webhook deploys before `STRIPE_PRICE_FOUR_WEEKLY_1299` and `STRIPE_PRICE_YEARLY_8437` are set,
  every subscription event in between writes `plan: null`. The gate is unaffected, because it reads
  status, but Settings shows no plan until the next event after a cold start. Whether setting a secret
  recycles warm workers is not verifiable from the repository. Set the variables before deploying any
  of the three functions, not only before checkout.
- **No subscription in either mode still carries `pause_collection`.** The resume path is gone, so
  such a subscription would never be charged again. Ruling 6 and §4 say none remains. The repository
  cannot confirm it.
- **Tags.** `v041-red/t10` and `v041-green/t10` exist in the shared repository and are not this run's,
  as the report says.

## Verdicts

T1a (price catalog): PASS
T1 (checkout): PASS WITH FIXES
T2 (webhook and reminder): PASS WITH FIXES
T3 (billing jobs): PASS WITH FIXES
Whole: PASS WITH FIXES
