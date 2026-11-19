# v0.4.1 pricing: design note

Planner, 2026-11-19. **Size: Contract**: it moves money, it moves `TOS_VERSION` in `app/src/account.rs`
(contract list), it publishes new terms, and, if Managed Payments ships, it changes the privacy page.
Status: r1, for review (one review and one re-review, reports in `docs/reports/`), then Quinn. Plan:
`docs/plans/2026-11-19-v041-pricing-plan.md`.

**Authority.** Quinn's ruling of 2026-11-19: pricing is fixed in v0.4.1, straight after the v0.4.0 tag.
The prices are the cloud design's Amendment 2026-11-10 (signed 2026-11-11), as Quinn's rulings of
2026-11-17 amend it (quoted as R2-R4, U5, U6, U10-U12 in `docs/specs/2026-11-17-legal-pages-rewrite.md`
§0, "the legal spec"):
- **$12.99 every 4 weeks** (13 charges a year), or **yearly at $84.37 once a year**;
- **14 days free**, card taken first, **a reminder 3 days before the first charge** (P14);
- **no summer pause**; **no Pell price** (U6); the 30-day money-back on the **yearly plan only** (ruling 8, P12);
- prices are **tax-exclusive** (U12); **Managed Payments, with Link as seller of record** (R2, U5, U12).

The gift year (ruling 9, P13) has its own spec and lane (legal spec C24) and is not built here. The v0.4.0
note (A2, §13) and the legal spec (§5) place the billing move in v0.5.0. Quinn's 2026-11-19 ruling is
later, so where they disagree about *when* billing moves, this note wins.

**Limits of what I checked.**
- I had no shell and no web tools.
- **The amendment is not on this branch.** This worktree's `docs/specs/2026-09-09-knowlu-cloud-design.md`
  has no "Amendment 2026-11-10". I read the `waitlist-spec` worktree's copy (`:993-1232`), which the legal
  spec's review r1 matched to `5dac15c7`. Whether `5dac15c7` has reached `main` is `unverified`.
  Precondition P1 (§8) covers both cases.
- **I opened nothing on Stripe's site.** Every Stripe statement in this note is marked `unverified
  (Stripe)` and names the page it is said to be on. Task T0 (a researcher with web access) and the
  test-mode spike S1 turn those statements into facts before any code depends on them.
- Code facts are cited `path:line` against this worktree, and I opened each one.

## 1. What changes for the student

- **The wizard's Subscribe step** offers two buttons, **Yearly** first and **Every 4 weeks** second. It
  still shows no price, no trial and no trial length (rule A2). The upgrade sheet keeps one Subscribe
  button, which buys the 4-week plan.
- **Stripe's checkout page** shows the plan's price:
  - $84.37 once a year, or $12.99 every 4 weeks;
  - 14 days free on the account's first subscription;
  - sales tax added on top;
  - one renewal sentence, which on the yearly plan includes the money-back guarantee.

  Under Managed Payments, the page says "Sold through Link".
- **Three days before the first charge**, Knowlu emails the student the plan, the amount, the date and how
  to cancel.
- **Settings → Membership** reads, for example, "Active, Yearly" or "Active, Every 4 weeks".
- **knowlu.com** carries a new version of the terms that states these prices. The page after checkout says
  14 days.
- **A build older than v0.4.1** that tries to subscribe is told to update Knowlu first.
- Nothing changes for a tester on a comp grant, and nothing changes in the engine.

## 2. Decisions, each with its reason

**D1. Rename the plan keys.** New subscriptions get the keys `four_weekly` and `yearly`. The keys `monthly`
and `academic_year` stay only as the names of subscriptions started before the switch, and nothing sells
them again.
- **The pause job would catch a reused key.** `billing-jobs` pauses every `monthly` subscription from June
  to August (`billing-jobs/handler.ts:126-137`). If the 4-week plan reused `monthly`, it would be paused,
  and ruling (1) says it has no pause.
- **P10 and the amendment ask for it.** P10 keeps an old subscription's price and pause "under its old plan
  name". The amendment's consequences say the 4-week plan "does not take the name `monthly`".
- **No migration is needed.** `entitlements.plan` is free text with no check
  (`20260910000100_accounts.sql:27`).
- **Testers and the gate are unaffected.** `tester` comes from the view, not from the key
  (`20261020000500_testers.sql:76-87`), and the gate reads only the status
  (`_shared/entitlement.ts:22, 79-81`).
- **Settings already knows `yearly`** (`app/static/js/pages/settings.js:32`).

The amendment also says "existing `academic_year` rows map to the yearly plan". I read that line together
with P10 and U10. An `academic_year` row keeps its name and its $69.99 price, and it counts as a yearly
plan for the money-back guarantee (U10). No row is rewritten.

**D2. The plan comes from the price id, not the interval.** The webhook looks up `items.data[0].price.id`
against four price ids held in the project's environment:
- `STRIPE_PRICE_FOUR_WEEKLY` and `STRIPE_PRICE_YEARLY`, which are new;
- `STRIPE_PRICE_MONTHLY` and `STRIPE_PRICE_YEAR`, which already exist (`billing-checkout/index.ts:16-19`).

Any other id gives `plan: null`. `billing-jobs` already reports a null plan as a failure and never mails a
guessed price for it (`billing-jobs/handler.ts:69-75, 119-123`).

*Reasons:*
- The interval cannot tell the new yearly price from the old academic-year price, because both are `year`.
- Today a `week` interval maps to `null` (`stripe-webhook/handler.ts:66-67`).
- Price ids are public values, set per project as function environment variables
  (`docs/plans/2026-09-09-c1-accounts-plan.md:2207-2209`). Function secrets are project-wide (several
  functions read the one `STRIPE_SECRET_KEY`), so the new ids are two values set once per project.

**D3. Older builds cannot buy the new prices.** `billing-checkout` accepts only two plans, `four_weekly` and
`yearly`. It also requires a `terms_version` no older than the terms that state these prices, a constant in
the function. Any other request is refused with 409 and the sentence "This version of Knowlu can't
subscribe any more. Update Knowlu, then try again." That covers a request for `monthly` or `academic_year`
and a request with an older terms version. The refusal comes before the Stripe customer, the consent row
and the session, in the same position as the existing refusals (`billing-checkout/handler.ts:95-107`).
Any other plan name stays a 400. The app shows the server's sentence as written
(`app/src/account.rs:210-217`).

*Reasons:*
- **Aliasing would falsify the consent record.** The consent row records the version the client sends
  (`handler.ts:120-127`), and every earlier build sends `TOS_VERSION` 2026-09-10 (`account.rs:24, 1300`).
  If `monthly` were aliased to the 4-week plan, Knowlu would record consent to the $9.99 terms for a $12.99
  charge.
- **Almost nobody is affected.** Nobody but Quinn has the app (v0.4.0 note, A1), and the updater offers
  0.4.1.
- **Older builds degrade quietly.** Settings in v0.4.0 shows the status without a plan word when it does
  not know the key (`settings.js:184`).

**D4. Checkout checks the price before it sells it.** `index.ts` maps each plan to its price id and its
cents: `four_weekly` is 1299 and `yearly` is 8437 (today's map is `index.ts:16-20`). Before it writes the
consent row, the function reads that price from Stripe and checks four things: the price is active, the
currency is `usd`, the amount equals the cents, and the interval is `week` with count 4, or `year` with
count 1. If any check fails, the function answers 500 "the function is not configured" and logs which
field failed.

*Reason:* the consent row takes `price_cents` from the map, not from Stripe. A price entered wrongly in the
Dashboard, such as $12.99 monthly, would otherwise be sold silently under terms that say something else.
The check costs one Stripe GET per checkout.

**D5. 14 days free, once per account.** The session asks for `subscription_data[trial_period_days]` = 14,
but only when the account has no `entitlements` row with a `stripe_subscription_id`
(`20260910000200_billing_jobs.sql:13`). The card is still taken up front: `payment_method_collection:
always` stays (`handler.ts:60`), for the reason the test gives (`handler_test.ts:22-25`).

*Reason:* ruling (7), and P3's one trial per account, which U6 left standing. The terms say "Each account
gets one free trial." Known limit: a deleted account's address can take a second trial on a new account,
because the tombstone is not read here.

**D6. The reminder is Knowlu's own email, sent by the webhook.** It is triggered by Stripe's
`customer.subscription.trial_will_end` event, which Stripe sends three days before a trial ends
(`unverified (Stripe)`: docs.stripe.com/billing/subscriptions/trials and docs.stripe.com/api/events/types).
- **When it is sent.** Only while the subscription is `trialing` and not set to cancel at the end of the
  period.
- **What it says.**
  - The plan's words, such as "Yearly".
  - The amount from the price, such as "$84.37 once a year, plus sales tax where it applies".
  - The date of the first charge. The date is taken in UTC−10, so the date named is never later than the
    student's own date anywhere in the fifty states. The email also says "in 3 days".
  - How to cancel: Settings, then Membership, then Manage subscription, or a reply to the email.
- **If the email fails.** The webhook answers 502 and records nothing, so Stripe retries the event.

*Reasons:*
- P14 is a promise made in Knowlu's name.
- The event arrives once per trial, the existing `webhook_events` record removes duplicates, and no
  migration is needed. A daily check in `billing-jobs` would need a new column and a change to the view.

If Link or Stripe sends a notice of its own (S1 finds out), Knowlu's still goes. A second notice is
harmless. A missing one would make the first charge refundable under the terms' case (c). This event
writes no entitlement, so the webhook stays the only writer of `entitlements` (`stripe-webhook/handler.ts:2-4`).

**D7. Managed Payments in checkout (legal spec C5).** The function creates the session as a Managed Payments
session and drops the two parameters Managed Payments refuses: `automatic_tax[enabled]` and
`customer_update[address]` (`handler.ts:65-66`; `unverified (Stripe)`,
docs.stripe.com/payments/managed-payments/update-checkout). The rest of the form (`handler.ts:51-72`) stays
unless T0 or S1 shows that Managed Payments refuses it.

If Managed Payments needs a different API version, only the session call carries it. The pin in
`_shared/stripe.ts:82` stays, for two reasons. The webhook parses events at the pinned version. And seven
functions bundle that file: `account`, `billing-checkout`, `billing-jobs`, `billing-portal`,
`mail-canary`, `mail-process` and `stripe-webhook`.

**S1's gate.** In test mode on staging, S1 must show five things:

| | S1 must show |
|---|---|
| (a) | A Managed Payments checkout writes the entitlement through Knowlu's webhook, with the right plan and trial end. |
| (b) | The card is taken, with 14 days free. |
| (c) | `trial_will_end` is delivered. |
| (d) | Manage subscription cancels the subscription in one click. |
| (e) | Deleting the account cancels it (`cloud/supabase/functions/account/index.ts:77-98`). |

S1 also records what Link sends and where refunds happen. If any of (a) to (e) fails, Quinn chooses at
checkpoint B. The recommended choice is to ship v0.4.1 without Managed Payments: keep today's Stripe Tax
form, have the terms say "Stripe", and make no privacy change. Managed Payments then moves to v0.5.0. The
alternative is to hold the release.

**D8. Stripe's page states the renewal terms.** The session sets `custom_text[submit][message]` from a pure
function of the plan and of whether a trial applies:
- **Every 4 weeks:** "Billed every 4 weeks[ after 14 days free], plus sales tax where it applies, until you
  cancel. Cancel any time in Knowlu: Settings, Membership, Manage subscription."
- **Yearly:** "Billed once a year[ after 14 days free], plus sales tax where it applies, until you cancel.
  30-day money-back guarantee on each yearly charge (knowlu.com/terms)."

*Reason:* P9 requires the billing terms, and on the yearly plan the money-back terms, to be stated before
the card is entered. A2 keeps them out of the app, so they go on Stripe's page. If Managed Payments refuses
`custom_text` (S1 finds out), Quinn rules whether the subscribe step may carry these sentences without the
trial clause, which would bend A2.

**D9. The app's labels.**
- **The wizard** (`app/static/js/wizard.js:34`). The buttons are "Yearly", first and red, and "Every 4
  weeks". They send `yearly` and `four_weekly` (`:642`). The yearly button comes first because ruling (2)
  shows the yearly plan first. The element ids stay as they are: no student sees them, and renaming them
  would only churn the DOM checks.
- **Settings.** Its table gains `four_weekly: "Every 4 weeks"`. It keeps `monthly: "Monthly"` and
  `academic_year: "Academic year"` for old subscriptions, because those really are billed that way, and
  Settings shows no price.
- **The upgrade sheet** sends `four_weekly` (`app/static/js/carried/account.js:137`). This follows Quinn's
  2026-09-14 ruling that the sheet sells one plan (`HANDOFF.md:80`).
- **The Customer Portal** offers no plan switching in v0.4.1, because the terms describe no switch.
- **A2 stays.** Its pins gain the new figures and "14 days" (`app/tests/ui_wizard.rs:584-602`,
  `app/tests/ui_settings.rs:55-67`). A2 is Quinn's ruling of 2026-11-17, later than P9's "the app's
  subscribe step" of 2026-11-11, and D8 meets P9 on Stripe's page.

**D10. The billing emails.**
- **`priceOf`** gives "$12.99 every 4 weeks, plus sales tax where it applies" and "$84.37 once a year, plus
  sales tax where it applies". The two old plans keep their old prices (P10).
- **The pause** stays `monthly`-only (`billing-jobs/handler.ts:130`). After D1 that means it runs only for
  P10's old subscriptions, as P10 says.
- **The annual reminder** goes to every plan.
- **The cancel path.** "click the gear" (`:86`, `:101`) becomes "go to Settings, then Membership".
  "Manage subscription" stays. It is pinned in `billing-jobs/handler_test.ts:153` and in
  `app/tests/ui_settings.rs:132`.

**D11. The terms publish with this lane, as one new version.** The text starts from the legal spec's
interim terms (§4, "Interim terms with bump A", which Quinn approved as U10). Its sections *The
subscription*, *How to cancel* and *Refunds* are replaced by the full text from §4, without the paragraphs
whose code this release does not build:
- the subscription confirmation email (C8);
- the yearly renewal notice (C10);
- the clause saying a disputed charge is not a paid period (C12);
- the "We" and shutdown paragraphs (C13);
- *A year for someone else* (the gift lane).

The text gains *Emails about your subscription*, with two bullets: the trial reminder (D6) and the annual
reminder (D10). "Sold through Link" appears only if D7 ships. The paragraph for old subscribers ("If you
subscribed before …", in the interim's summer-pause wording) appears only if production still holds an
old-price subscription at go (§4, step 7).

**These terms replace bump A's interim terms.** Whenever bump A ships, it carries the privacy page only.
`TOS_VERSION` moves in one commit with the page and with checkout's terms floor from D3
(`account.rs:21-24`). A new test that ties the three sits beside the privacy one
(`app/tests/bundle_config.rs:134-157`). Until that commit, the floor equals today's `TOS_VERSION`,
2026-09-10. `site/subscribed.html:13` changes to name
14 days, say they apply to a first subscription, and give the Settings → Membership path.

*Existing accounts.* The server never records a later terms version for an account that already has one
(`cloud/supabase/functions/account/handler.ts:203`), and C1 is not built.
- **A paying student** accepts the new version at checkout, because the auto-renew row carries it (D3).
- **Everyone else** comes under the legal spec's gate (N3). The main session runs one read-only production
  count. If Quinn's is the only account, Quinn's acceptance is recorded by hand, as the legal spec §4
  describes. If any other account exists, C1 joins this lane before the terms publish.

**D12. The privacy page names Link (only if D7 ships).** Under Managed Payments, Link receives the card, the
billing address and the email address. The page says its list of companies is closed ("Nobody else gets
anything", `site/privacy.html:86`), so it has to name Link. The texts come from the legal spec's B variants
in §3: *Your subscription*, the *Link* bullet, and *Delete*. The stale count "Seven companies"
(`site/privacy.html:75`) becomes the legal spec's D3 wording. A build carries one privacy bump, because it
has one `PRIVACY_CHANGES` key (`app/tests/ui_account_privacy.rs:226-233`).
- If bump A merges first and rides v0.4.1, these paragraphs join bump A.
- If not, this lane makes a Link-only bump, and bump A takes the next tag.

Quinn reads the page.

## 3. What Managed Payments changes (every row to be confirmed by T0 and S1)

Sources: the legal-pages audit (`docs/reports/2026-11-17-legal-pages-audit.md`, gap 1 and its sources
list) cites the two Stripe pages below. The 2026-11-17 cost research read Stripe's pricing pages; its notes
are session notes, not in the repository. **None of these pages was opened for this note.**

| Area | What the sources say | Page | What this lane does |
|---|---|---|---|
| Checkout Session | A Managed Payments session is created with `managed_payments[enabled]`. Managed Payments refuses `automatic_tax` and `customer_update[address]`. | `docs.stripe.com/payments/managed-payments/update-checkout` | D7. T0 confirms the parameter, any other parameter it refuses (`billing_address_collection`, `consent_collection`, `allow_promotion_codes`, `custom_text`, `customer`) and whether a preview API version is needed. |
| Tax | Link, as seller of record, calculates, collects and remits sales tax. | `docs.stripe.com/payments/managed-payments/how-it-works` | Prices are created tax-exclusive (U12). Whether this settles Amendment 2026-10-16b's Kentucky registration item goes on the lawyer's list. |
| Receipts | Link sends them, and the customer sees "Sold through Link". | same | The terms say so (D11). Stripe's own receipt setting no longer matters. |
| Trial reminders | Link sends trial reminders. | same (audit) | Knowlu's reminder goes anyway (D6). S1 records when Link's arrives and what it says. |
| Cancelling | The customer can cancel at link.com. | same (audit) | The in-app path stays Manage subscription. S1 (d) checks that the portal still cancels a Managed Payments subscription. |
| Refunds | Link may refund within 60 days whatever Knowlu's policy says, at Knowlu's cost, and refunds on its own when an escalation goes unanswered for 48 hours. Knowlu can still refund from the Dashboard. | the audit; the 2026-11-17 cost research | The money-back guarantee and the four refund cases are honoured by hand in the Dashboard (the webhook does not learn of refunds until C12). S1 records whether a refund returns the tax and the 3.5% fee. |
| Fee | 3.5% on top of the card and Billing fees. | the 2026-11-17 cost research | Not an input to this design. The amendment's margin check already assumes a higher fee. |
| Subscriptions and webhooks | Nothing on record. | (none) | S1 (a) is the hard gate: the subscription, its metadata and its events must reach Knowlu's endpoint as they do today. |

## 4. Order of rollout, and the Stripe steps only Quinn can take

**Staging, in test mode.**
1. Quinn takes the test-mode steps in the checklist below.
2. The main session sets the two price ids on staging (`supabase secrets set …`; they are public values).
3. The main session deploys `billing-checkout`, `stripe-webhook` and `billing-jobs` to staging from the
   lane branch.
4. **S1.** The main session runs it, with Quinn at the test-mode Dashboard. It completes test-mode
   checkouts with Stripe's test card. No money moves.
5. **Checkpoint B.** Quinn rules on D7 if S1 failed.
6. The live proof (the plan, §8).

**Production, in live mode, each step on Quinn's word.**

7. **Read-only checks.** These must be confirmed before go:
   - the account count (N3);
   - `entitlements` by plan and status;
   - open comp grants.

   Quinn reads, in the live Dashboard, any subscription on an old price. These checks first run at the
   plan's T0, because they decide D11's terms text. They run again here to confirm that nothing has
   changed.
8. **The pull request merges**, with CI green and every review passed.
9. Quinn takes the live-mode steps in the checklist.
10. The main session sets the production price ids and deploys the three functions. From this moment no
    older build can subscribe, and no build can reach the new prices until the tag.
11. **The `v0.4.1` tag.** CI builds and signs the release, and `release.yml:294-298` deploys `site/`, so the
    terms and the app go live together.
12. If D11's count allows it, Quinn's acceptance of the new terms is recorded by hand, after the page is
    live.
13. **After the tag.** The main session checks that knowlu.com/terms.html carries the new version. Then
    Quinn's own account opens a live checkout page, and the main session reads it. Nothing is paid.

*Why the functions go before the tag.* In the opposite order, the published terms would state $12.99 while
older builds could still buy at $9.99 from the old function, and v0.4.1 would get a 400 "unknown plan".

**Quinn's checklist.** Do each step in test mode first, then in live mode at step 9.
- [ ] On the existing Knowlu product, create a price of **USD 12.99, recurring every 4 weeks** (interval
      week, count 4), with **tax behaviour exclusive**. Send the id: it is public.
- [ ] On the same product, create a price of **USD 84.37, recurring yearly**, tax behaviour exclusive. Send
      the id.
- [ ] Do not archive the old prices while any subscription uses them (P10).
- [ ] Turn on **Managed Payments**, and check that the product's tax code is one Managed Payments accepts.
- [ ] On the webhook endpoint, add the event **`customer.subscription.trial_will_end`**. Today's list is in
      `docs/plans/2026-09-09-c1-accounts-plan.md:2198`. Adding it before the new function is deployed is
      safe: today's webhook answers 200 "ignored" to any event type it does not handle
      (`stripe-webhook/handler.ts:118-125`).
- [ ] In the **Customer Portal**, keep cancellation on with no survey, and turn **plan switching off**.
- [ ] Note whether **Stripe's own trial-reminder email** is on. If S1 shows Link sends a reminder, turn
      Stripe's off, so that the student gets two emails, not three.
- [ ] Review the **promotion codes**: P10 keeps them for comp grants only. A code on the Knowlu product also
      works on the new prices.
- [ ] Check that the **terms URL** in the public business details is still knowlu.com/terms.html.
      `consent_collection` needs it.
- [ ] No new secrets. `STRIPE_SECRET_KEY` and `STRIPE_WEBHOOK_SECRET` stay as they are.

## 5. What must not break

- **The two overriding rules.**
  - *No single-user assumption.* Price ids come from each project's environment. No account, person or
    address is named in code, tests or docs. The proof uses scratch profiles, and the reminder's date rule
    assumes no student's time zone.
  - *No frozen reference is regenerated.* Nothing in `engine/` changes.
- **The gate.** Entitlement depends on status alone, so `trialing` counts as entitled. Testers come from
  `effective_entitlements`, and a lapsed account still gets the 402.
- **Checkout's order.**
  - Refusals come before any Stripe customer, consent row or session. That covers the 18+ refusal
    (`handler.ts:103`), the tester refusal (`:107`), the new D3 refusal and D4's price check.
  - The consent row is still written before the redirect (`handler.ts:6-8`).
- **The webhook.** It verifies the signature before it parses (`stripe-webhook/handler.ts:105-115`), then
  removes duplicates, then applies the stale guard, and it ignores a deleted account. It stays the only
  writer of `entitlements`. The Stripe API pin stays as it is (`_shared/stripe.ts:82`).
- **P10.** Old subscriptions keep their price, their name and their summer pause. The pause job runs for
  them alone.
- **`billing-jobs`.** One subscriber's failure never stops the run, and it never mails a guessed price
  (`billing-jobs/handler.ts:14-16, 69-75`).
- **A2.** No price, trial or trial length appears in the wizard, the picker or Settings. "Manage
  subscription" keeps its exact words.
- **The wording rule.** Every price Knowlu writes (emails, terms, Stripe's custom text, the site) says
  "every 4 weeks" or "once a year". None says "a month", "/mo" or "monthly" beside a price.
- **The legal pages.**
  - The version ties hold: the privacy tie (`bundle_config.rs:134-157`) and the new terms tie.
  - A build carries one `PRIVACY_CHANGES` key.
  - Every existing pin in `engine/tests/site.rs` still passes.
- **Releases.** Releases are built only by CI. Deploys to production and the tag wait for Quinn's word.

## 6. Tests that prove it (each written first, and failing)

**Cloud, `deno test`.**
- **`billing-checkout/handler_test.ts`**
  - `four_weekly` and `yearly` reach Stripe with their price and cents.
  - `monthly`, `academic_year` or an older `terms_version` answers 409 with the update sentence, and
    nothing is called first. An unknown plan answers 400.
  - The trial is "14" on a first subscription and absent on a second.
  - The Managed Payments form has exactly the keys T0 confirms.
  - `custom_text` is pinned for every plan and trial combination.
  - Each price-check failure (amount, currency, interval, count, inactive) answers 500 before the consent
    row.
  - The 18+ and tester tests stay unchanged.
  - A text-read of `index.ts`, following the existing precedent (`handler_test.ts:331-343`), pins the
    environment names, the cents and the terms floor.
- **`stripe-webhook/handler_test.ts`**
  - The plan comes from the price id, for all four ids. An unknown id gives `null`, and the interval no
    longer decides anything.
  - `trial_will_end` sends one email. It names the plan's words, "$84.37 once a year" or "$12.99 every 4
    weeks", the UTC−10 date, "in 3 days" and Manage subscription.
  - No email is sent when the subscription will cancel at period end or is not `trialing`.
  - When the email fails, the webhook answers 502 and records nothing.
  - A duplicate event sends no second email, and the event writes no entitlement.
- **`billing-jobs/handler_test.ts`**
  - `four_weekly` and `yearly` are never paused in June to August, and both get the annual reminder with
    their new wording.
  - Neither new plan's mail says "per month".
  - The old plans' mails are unchanged: the "$9.99" pin at `:149` stays for the old resume notice.
  - "Manage subscription" stays, and "click the gear" is gone.

**App, `cargo test`.**
- **`ui_wizard`**
  - `the_subscribe_buttons_send_todays_plans_by_name` becomes `…send_the_signed_plans`. It checks the
    labels, the order, the values, and the line in `billing-checkout` that lists the plans (today
    `:614-615`).
  - The banned list in `the_wizard_states_no_price_or_trial` gains "12.99", "84.37", "6.49", "14-day",
    "14 days", "fourteen" and "/mo".
- **`ui_settings`**
  - The table carries the four plan words.
  - The banned list in `membership_shows_no_price` gains "84.37", "6.49" and "14 days".
- **`ui_plans` (new)**
  - The sheet sends `four_weekly`.
  - Every plan the app sends is in billing's list of plans.
  - Every key the webhook can write has a word in Settings.
- **`bundle_config`**: a new test, `the_terms_version_constant_is_the_published_terms_date`. It also checks
  that the terms floor in `billing-checkout` equals `TOS_VERSION`, so that the page, the constant and the
  floor move together.
- **`engine/tests/site.rs`, the terms**
  - The terms contain "every 4 weeks", "$12.99", "$84.37", "once a year", "14 days", "Sales tax is added at
    checkout where it applies", "30-day money-back guarantee on the yearly plan", "Each account gets one
    free trial" and "There is no summer pause".
  - The terms contain none of "a month", "per month", "/mo", "$9.99", "$69.99", "Pell" or "Alabama".
  - `subscribed.html` says "14 days" and not "seven-day".
- **If D12 ships, the privacy page.** New pins cover the Link texts. The panel test that names Gmail and
  Amazon moves to name this bump's subject.

**S1 and the live proof** (the main session; the plan's §8). These cover what no unit test can:
- Stripe's page really shows $84.37, $12.99 every 4 weeks, 14 days and the tax line (and "Sold through Link"
  if D7 ships).
- The webhook writes `yearly` or `four_weekly`, with a trial end 14 days out.
- The reminder arrives.
- One click cancels the subscription.
- The wizard shows the new labels with no price.
- An older build gets the 409.

## 7. Not in this lane

These stay in the v0.5.0 billing lane or later:
- **C1**, terms re-acceptance, unless the D11 count brings it in.
- **C2**, emailing every account on a material policy change.
- **C7's record**, the consent record keeping the plan and the text shown. That needs columns; `price_cents`
  already tells the two plans apart.
- **C8**, the acknowledgment email.
- **C10**, the yearly renewal notice. It must land before the first yearly renewal, which comes no sooner
  than one year and 14 days after the first yearly checkout.
- **C11**, the earlier pause-resume notice.
- **C12**, refunds and disputes in the webhook.
- **C13**, closing an account and the shutdown.
- **C15** and **C16**.
- **The gift year** (C24).
- **Pell.**
- **Plan switching.**
- **Any price on the homepage** (Q5).

Outside paid sign-ups still wait on Amendment 2026-10-16b's gate (the lawyer's read, the tax question, a
signed installer). v0.4.1 makes billing correct. It does not open sales.

## 8. For Quinn: decisions, each with a recommendation

- **P1. Precondition: put the signed rulings on `main` before T2 starts.**
  - Merge Amendment 2026-11-10 to `main`.
  - Record the 2026-11-17 rulings (U6 Pell dropped, Managed Payments, tax-exclusive prices) as an amendment
    to it (legal spec C26).
  - Write VISION's "One edition" line and CLAUDE.md's Direction line in your own words.

  *Recommend:* yes, because otherwise this release's terms disagree with VISION.
- **Q1. Managed Payments in v0.4.1, with the Link privacy bump (D7, D12)?** *Recommend:* yes, gated by S1.
  If S1 fails, ship the prices without Managed Payments and move it to v0.5.0.
- **Q2. Production data and old subscriptions.**
  - Run the read-only checks at step 7.
  - Cancel your own old-price subscriptions in both modes, and put your own account on a comp grant.

  *Recommend:* yes. Then no old-price subscription is left, and the terms drop the old-subscriber paragraph.
- **Q3. The mailing address for the terms' Notices clause (legal spec U9).** *Recommend:* supply it before
  the tag. The alternative is that Notices names only support@knowlu.com, which goes on the lawyer's list.
- **Q4. The Stripe account in KNOWLU LLC's name.** The legal spec's Q1 recommends moving Stripe first,
  before the billing lane, and this is that lane. *Recommend:* move it before the live-mode checklist.
- **Q5. The homepage keeps "Pricing will be announced"?** *Recommend:* yes, until outside sales open.
