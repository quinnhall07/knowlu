FAIL

# v0.4.1 pricing: design note and plan review, round 1

Reviewer (spec/plan mode), 2026-11-19. Branch `docs/v041-pricing` at `ce80e90e`.
Documents: `docs/specs/2026-11-19-v041-pricing-design.md` ("the note") and
`docs/plans/2026-11-19-v041-pricing-plan.md` ("the plan").

**Verdict: FAIL.** One Critical and seven Important findings. None of them needs a redesign. The money
model itself is right: the cents (1299 and 8437; 13 x 6.49 = 84.37), mapping the plan from the price id,
keeping the old keys for P10, keeping the pause on `monthly` only, refusing old builds with a 409, and
publishing the terms in the same Pages deploy as `latest.json`. The problems are in the production half
of the rollout, in three readings of signed text that only Quinn can confirm, and in Managed Payments
parameters that could be dropped without anyone deciding to drop them.

I checked these code claims and found them accurate:
- `billing-checkout/handler.ts:14-16, 51-72, 95-127` and `index.ts:16-20`;
- `stripe-webhook/handler.ts:64-67, 97-125`;
- `billing-jobs/handler.ts:69-75, 126-137, 86, 101`;
- `_shared/entitlement.ts:22, 79-81`;
- `_shared/stripe.ts:82`, and the seven importing functions;
- the `entitlements` columns (`20260910000100_accounts.sql:27`, `20260910000200_billing_jobs.sql:13`);
- `wizard.js:34, 642`, `settings.js:32, 184`, `carried/account.js:137`;
- `account.rs:24-25, 1292-1301, 1611-1621`;
- the pins in `ui_wizard.rs`, `ui_settings.rs`, `bundle_config.rs` and `ui_account_privacy.rs`;
- `release.yml:294-298`. The site and `latest.json` go out in one Pages deploy, so the terms and the
  updater go live together.

The amendment is still not on `main` (`5dac15c7` is only on `docs/waitlist-spec`); P1 covers this.

The contract list is respected. Only `app/src/account.rs` is touched, by `contract-engineer` in T5 (and
T5b). `engine/src/entitle.rs` and the other files on the list are untouched. Every task has an agent
label. No task carries pre-written code, and the parallel tasks own disjoint files.

## Critical

**C1. The production rollout assumes production parity, and nothing checks for it.** Note §4 steps
7-10; plan §3 P3, §8 C steps 7 and 10.
- *What it conflicts with:*
  - `HANDOFF.md:598` records production at "C1's seven migrations and eight functions of 2026-09-14,
    live billing, and nothing later". The L9 parity lane (`HANDOFF.md:172`) is still an open row.
  - That row names "L8's `<stamp>_testers.sql` and the billing-checkout and entitlement redeploys,
    before any comp account exists on prod".
  - Today's `billing-checkout/index.ts:35-42` reads `effective_entitlements`, which is defined in
    `20261020000500_testers.sql:76`.
- *Consequence as written:*
  - Step 10 deploys `billing-checkout` from the merge SHA to a project that may not have the view.
  - `isTester` runs before the new D3 refusal (plan T1's refusal order), so every production
    checkout, from any build, answers 5xx.
  - Step 10's own check (POST `monthly`, expect 409) then fails halfway through the cutover, with the
    old function already replaced.
  - P3's third query (`public.comp_grants`) and Q2's comp grant for Quinn's account fail for the same
    reason.
  - The note's claims about the account function (`account/handler.ts:203`, the "never records a later
    terms version" rule D11 relies on) describe the repository, not what is deployed on production.
- *Fix:*
  - T0 reads production's `migration list` and `functions list` (read-only, on Quinn's word).
  - The migrations the three functions and Q2 depend on, at least `20261020000500_testers.sql` and
    `20261020000600_comp_guards.sql`, become a named precondition (L9 steps 1-2 for those stamps),
    with Quinn's go.
  - Step 7 confirms the same before go.

## Important

**I1. A2 ends when billing moves, so "A2 stays" (D9) contradicts signed P9.** Note D9 last bullet,
§5 "A2"; plan §11 item 3.
- *Conflicts with:*
  - A2's own text (`docs/specs/2026-11-17-v0.4.0-new-ui-design.md:24-27`): billing moves "in a later
    cloud lane … **until then** the wizard shows no price figure, trial length or plan price". This
    lane is that later lane.
  - Amendment P9 (`5dac15c7`, cloud design `:1083-1087`): "The app's subscribe step and Stripe
    Checkout state the every-4-weeks billing …, the billed amount …, on yearly the money-back terms …
    all before the card is entered."
- *Consequence:* v0.4.1 sells $84.37 and $12.99 behind two bare buttons. That rests on a ruling that
  stops applying with this lane. The tests T4 extends (the banned-word lists) would then lock in a
  breach of a signed amendment.
- *For Quinn, with a recommendation:* show P9's two lines on the subscribe step, yearly first, in
  ruling (2)'s words ("$6.49 every 4 weeks, billed $84.37 once a year. Renews yearly." beside "$12.99
  every 4 weeks"), plus "plus sales tax where it applies". T4's pins then flip from banning these
  figures to requiring them. If Quinn wants the app to stay price-free, record that as an extension of
  A2 in P1's 11-17 amendment, so that P9 is formally narrowed and not just read around.

**I2. The note ignores the repository's own record that Managed Payments is already on.** Note §3 (the
"Subscriptions and webhooks: Nothing on record" row), the §4 checklist ("Turn on Managed Payments"), D7.
- *Conflicts with:*
  - `HANDOFF.md:80` and `:629`: on 2026-09-14 Quinn "kept Managed Payments on (P3)" in live mode.
  - `HANDOFF.md:63`: a live checkout on 2026-09-24 completed and the webhook wrote the entitlement,
    with today's form, which includes `automatic_tax[enabled]`.
  - The amendment `:1131` and `:1157`: "Managed Payments is already on".
- *Consequence:*
  - Either the 09-14 setting is not the per-session Managed Payments the note describes, or a live
    Managed Payments session already accepted `automatic_tax` and the webhook path works. In the
    second case D7's premise that Managed Payments refuses `automatic_tax` is wrong.
  - Executed as written, T0r and S1 start from "nothing on record", and Quinn is told to turn on
    something that may already be on.
- *Fix:*
  - Add to T0r's questions: what the account-level setting recorded on 09-14 is, and whether it
    applies to a session that does not ask for it.
  - S1 (a) uses the 09-24 live result as prior evidence.
  - Reword the checklist item as "confirm Managed Payments is on".

**I3. D7 lets an implementer drop parameters that carry money and legal weight.** Note D7 ("The rest
of the form stays unless T0 or S1 shows that Managed Payments refuses it"); plan T1
`the_form_is_a_managed_payments_session`, checkpoint A.
- *Conflicts with:*
  - `billing-checkout/handler.ts:6-8, 68-69`: the consent row is written on the premise of Stripe's
    terms checkbox.
  - `handler_test.ts:22-25`: `payment_method_collection=always` is what makes the terms' "card taken
    first" true.
  - Ruling (7): 14 days free.
  - `billing-portal` and account deletion (`account/index.ts:82-96`) depend on the stored
    `stripe_customer_id`.
- *Consequence:* if T0r reports that Managed Payments refuses any of these, checkpoint A writes the
  keys into T1's brief and the implementer drops them:
  - without `consent_collection[terms_of_service]`, Knowlu records auto-renew consent nobody ticked;
  - without `payment_method_collection`, no card is taken;
  - without `trial_period_days`, there is no trial;
  - with `customer` refused, Manage subscription opens a portal that has no subscription in it.
- *Fix:* name these five as load-bearing: `consent_collection`, `payment_method_collection`,
  `trial_period_days`, `customer` and `custom_text`. A refusal of any of them is a checkpoint B failure
  for Quinn, with D7's fallback, never a silent removal. Only `automatic_tax`, `customer_update` and
  `billing_address_collection` may be dropped at checkpoint A.

**I4. Q2's recommendation breaks step 13, and step 10's check would pass for the wrong reason.** Note
§8 Q2 and §4 step 13; plan §8 C steps 10 and 13.
- *Conflicts with:*
  - `billing-checkout/handler.ts:107`: a tester account gets 409 "this account has free tester
    access".
  - `20261020000500_testers.sql:76-86`: a canceled Stripe row plus an open grant reads `tester`.
  - D5: an account that has had a subscription gets no trial.
- *Consequence:*
  - Once Q2 is done, Quinn's account cannot open a live checkout at all.
  - Even before Q2, Quinn's account (which already had a live subscription, `HANDOFF.md:63`) would
    show no "14 days".
  - So the only live check of D4's price validation, of the trial and of the live page never happens.
  - Step 10's POST `monthly` from that account answers 409 with the tester sentence. That satisfies
    "409" while proving nothing about D3.
- *Fix:*
  - Step 10 asserts D3's exact sentence.
  - Step 13 opens the live page before the comp grant is applied, or with a fresh production scratch
    account that is deleted afterwards. A fresh account shows the trial, and it must be counted in
    step 7's numbers.
  - Q2 says to cancel immediately, not at period end. Otherwise step 7 still shows an `active` monthly
    row and the old-subscriber paragraph comes back.

**I5. The live reminder is never proven, and a missing reminder is a refund the terms promise.** Note
§4 steps 9-13, D6; plan §8 C.
- *Conflicts with:*
  - Terms case (c) (legal spec §4, *Refunds*).
  - `HANDOFF.md:594, 597`: "which Resend key is live" is still open, and the production Resend key
    roll is pending.
- *Consequence:*
  - The `trial_will_end` event on the live endpoint is a checklist tick that nobody verifies.
  - The production `EMAIL_*` path is never exercised, because the proof sends mail only on staging.
  - If either is wrong, the first live trial gets no reminder, and Stripe's retries of a 502 run for
    days. That raises endpoint alarms that also cover the entitlement events.
- *Fix:* step 10 or 13 records two things:
  - Quinn's reading of the live endpoint's enabled events, `trial_will_end` included;
  - one delivered production email through the same `EMAIL_*` secrets: a `billing-jobs` or
    `mail-canary` send to Quinn's own address, recorded as "delivered" with no address in the record.

**I6. Q4's recommendation has a consequence the plan does not handle.** Note §8 Q4; plan §3 P6, the
§4 checklist's "No new secrets".
- *Conflicts with:* `billing-checkout/handler.ts:109-118`, which reuses `accounts.stripe_customer_id`,
  and `account/index.ts:82-96`, which cancels by the stored subscription id.
- *Consequence, if "move to KNOWLU LLC" means a new Stripe account:*
  - every stored customer id refers to the old account, so checkout answers Stripe's "No such
    customer" as a 502;
  - the portal and deletion fail the same way;
  - `STRIPE_SECRET_KEY` and `STRIPE_WEBHOOK_SECRET` change, which contradicts the checklist;
  - the live webhook endpoint, product, prices and tax code have to be rebuilt.

  If it means updating the business details of the same account, none of this happens.
- *Fix:*
  - T0r answers which of the two it is.
  - Q4's text states both branches.
  - The new-account branch adds clearing the stored customer ids, a production write on Quinn's word,
    plus new secrets and a new endpoint to step 9.

**I7. Three readings of signed text are missing from Quinn's list.** Note D1 (last paragraph), D9 (the
upgrade sheet), D11 (the gift year), §8; plan §11 items 1, 5 and 10.
- **"Existing `academic_year` rows map to the yearly plan"** (amendment `:1196-1197`). The note keeps
  `academic_year` instead. That is the money-safe reading: mapping the rows to `yearly` would mail
  "$84.37" to a $69.99 subscriber through `priceOf`. But it changes a signed sentence.
  *Recommend:* record it in P1's 11-17 amendment as Quinn's ruling.
- **R3, "Keep the gift year if the amendment has it"** (legal spec `:108-112`). The amendment has it
  (ruling 9), and D11 drops the paragraph. *Recommend:* omit it until the gift lane ships, because
  the terms must not promise an unbuilt checkout. Quinn confirms.
- **The upgrade sheet sells only the 4-week plan.** This rests on a 2026-09-14 ruling. The amendment
  prices $12.99 "to make the year the obvious buy" (`:1111-1112`), and ruling (2) shows the yearly
  plan first. A lapsed student gets only the plan the price is designed to steer them away from.
  *Recommend:* two buttons, Yearly first, mirroring the wizard. That is one more value in T4. If Quinn
  does not want that, Quinn confirms the 4-week-only sheet.

## Minor

- **M1. D4 should also check `tax_behavior === "exclusive"`.** Note D4; plan T1
  `the_price_is_checked_before_it_is_sold`. U12 makes prices tax-exclusive. A price created as
  `inclusive`, or as `unspecified` under the fallback form where the account default applies, sells
  $12.99 including tax under terms that say tax is added. Add it as a sixth case.
- **M2. D6's "in 3 days" can be false, and unknown prices are unspecified.** Note D6; plan T2.
  - A 502 is retried by Stripe for up to days, and S1 (c) moves the trial end to two days out, so an
    email saying "in 3 days" can arrive late or after the charge.
  - *Fix:* skip when `trial_end <= now`, and name the date only, or compute the days remaining.
  - The behavior for an old or unknown price id is also unspecified. Skipping it means no reminder,
    which is refund case (c). *Recommend:* take the amount from the event's own price
    (`unit_amount`, `recurring`), or from the old `priceOf` wording, and pin it with a test.
- **M3. The terms floor accepts any string that sorts higher.** D3 compares "no older than", so
  "9999-99-99" or "zzz" passes and is written into `consents.version`. Use the account function's
  precedent (`account/handler.ts:199-201`): a YYYY-MM-DD date, no earlier than the floor and no later
  than tomorrow.
- **M4. Two checkouts in parallel give two trials and two subscriptions.** Pre-existing, but D5 now
  reads the row anyway. Account deletion cancels only the one stored id (`account/index.ts:89-96`), so
  the second subscription bills a deleted account (refund case (a)). *Optional:* answer 409 when the
  row is `active` or `trialing` with a subscription id. The app already asks first, so this is the
  server-side backstop.
- **M5. S1 leaves gaps.**
  - Only `four_weekly` is completed, so the webhook's `yearly` mapping is never seen live. Complete
    yearly on account B before its S1 (e) deletion.
  - "No trial on a second subscription" is never seen. Open a checkout on account A after its cancel
    and read the page.
  - S1 (a) expects "the new terms version" in the consent row, but S1 runs before T5 creates that
    version. Assert the version sent; the proof checks the new one after T5.
- **M6. A missing old price id would take the whole webhook down.** The new `stripe-webhook/index.ts`
  will read four `STRIPE_PRICE_*` names on every request, so a missing old id answers 500 to every
  event. Step 10 checks only checkout. Add a check of the webhook's delivery log after the deploy, or
  make the two old ids optional (an absent id simply matches nothing).
- **M7. P1's blocking point is inconsistent.** The note §8 says "before T2 starts", while the plan §3
  and T0 say it blocks T1. In lean mode, the amendment merge gates T1 (authority), and the VISION and
  CLAUDE.md wording gates the tag, not the code.
- **M8. The live 100% promotion code works on the new prices.** It is the code Quinn used
  (`HANDOFF.md:63`). The checklist says only "review the promotion codes". *Recommend:* deactivate
  or restrict it once Q2's comp grant exists, because comp grants are now the comp mechanism (P10).
- **M9. The S1 report goes into a public repository.** `docs/reports/2026-11-2x-v041-s1-managed-payments.md`
  should follow the ledger's rule explicitly: no address, customer or subscription id, and no receipt
  text that carries one.
- **M10. The docs will not reach `main` with the v0.4.0 merge.** Plan §2 says they will, but they sit
  on `docs/v041-pricing`, a separate branch, so they need their own merge or docs push before T0.

## Scope

Nothing to cut. Each piece serves "charge the signed prices correctly, with matching terms":
- the reminder (P14, and terms case (c));
- `custom_text` (P9);
- the old-build 409 (the consent record's integrity);
- the price check, one GET that guards against a wrong Dashboard price.

Managed Payments with D12 is the largest item, and Q1 already gates it on S1 with a fallback. T5b
(C1) is conditional and correctly so. `ui_plans.rs` is small and closes a real drift.

## Quinn's list (§8): review of each item

- **P1** is Quinn's. Narrow what it blocks per M7, and add I7's three rulings to it.
- **Q1** is Quinn's, and the recommendation is sound once I2 is reconciled.
- **Q2** is Quinn's. The recommendation needs I4's ordering and "cancel immediately", and it depends
  on C1 (the comp grant table on production).
- **Q3** is Quinn's, and the recommendation is sound.
- **Q4** is Quinn's. The recommendation is incomplete without I6's two branches.
- **Q5** is Quinn's, and the recommendation is sound.
- **Missing:**
  - I1 (A2 against P9);
  - I7's three readings;
  - L9 parity as a precondition (C1);
  - optionally M8 (the 100% code).

## VISION, the two rules, privacy

- **VISION.** The lane makes the price honest, and it opens no sales (10-16b's gate stands). VISION's
  price line is stale, and P1 covers that.
- **Rule 1.** Price ids come from each project's environment. Test data uses `example.invalid`. The
  steps that name Quinn's account are one-off production operations, not code. No violation.
- **Rule 2.** Nothing under `engine/src` or the fixtures changes.
- **Privacy.**
  - D12 covers Link, and `PRIVACY_VERSION` moves only in T5's commit with its page, published by the
    tag's single deploy.
  - The reminder uses the existing email processor and the account's own address. It adds no data
    class and no telemetry.
  - T2 should pin that the reminder's failure log names the account id only, as `billing-jobs`
    does at `:173-175`.
