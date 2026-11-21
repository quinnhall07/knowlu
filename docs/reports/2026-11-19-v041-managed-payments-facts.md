# v0.4.1: Managed Payments and billing facts (T0r)

**Summary**
1. **Neither reading of D7 holds as written.** An account-level setting can make a session Managed Payments without the session asking, so (i)'s premise is wrong. But Managed Payments refuses `automatic_tax`, so (ii)'s "Managed Payments accepted `automatic_tax`" is also wrong. The 2026-09-24 session, which sent `automatic_tax`, was therefore probably an ordinary Stripe Tax session; the live Dashboard settles it. A setting in the API call overrides the account setting, so T1 should send `managed_payments[enabled]=true` every time. That is reading (i)'s action, and it is correct whichever reading turns out true.
2. **None of the seven load-bearing checkout keys is refused under Managed Payments.** Of the four keys that may be dropped, Stripe refuses two: `automatic_tax[enabled]` and `customer_update[address]`. It does not refuse `billing_address_collection` or `allow_promotion_codes`, so under checkpoint A's rule those two stay.
3. **Five findings change the note or S1:**
   - Stripe sends no trial reminder emails in a sandbox, so S1 cannot record Link's reminder.
   - When `trial_will_end` fires is a Dashboard setting (default 3 days), not a fixed 3 days.
   - Link sends a 12-month anniversary email 15 days before by default. That races a yearly `move` whose notice comes fewer than 15 days before renewal.
   - Metadata writes merge (documented).
   - In live mode, turning Managed Payments back on after turning it off may be impossible.

Everything below was read from Stripe's pages on 2026-11-21. No Stripe API call was made and nothing was signed into. Code was read from `origin/main`. Two page caveats:
- My fetch tool cut the support.stripe.com quotes to 125 characters, and one is truncated where marked.
- The GitHub PR I cite is a third party, not Stripe, and is labelled where it is used.

---

## Q1. Every row of the note's §3 (`docs/specs/2026-11-19-v041-pricing-design.md:710-720`)

| Row | Verdict | Stripe's words (fetched 2026-11-21) |
|---|---|---|
| Account setting | **Partly contradicted.** The record stands, but Stripe documents an "auto-enablement" setting that selects Managed Payments by default, and an explicit API value wins. See Q2. | support.stripe.com/questions/change-your-managed-payments-auto-enablement-setting: "An explicit API setting takes precedence over the account auto-enablement setting." |
| Checkout Session | **Confirmed, and broader.** The parameter is `managed_payments[enabled]`. Refused keys include `automatic_tax` and `customer_update[address]`, and also `customer_update[name]`, `tax_id_collection`, `payment_method_types`, `payment_method_configuration`, `adaptive_pricing`, `shipping_*`, `subscription_data.invoice_settings`, `subscription_data.default_tax_rates`, `invoice_creation` and the Connect keys. The row's "asked for per session" is true, but auto-enablement can also select it. | docs.stripe.com/payments/managed-payments/update-checkout: "some parameters aren't available when using Managed Payments, and you must remove them when creating Managed Payments Checkout Sessions." |
| Tax | **Confirmed.** Stripe is merchant of record; the customer sees Link. | docs.stripe.com/payments/managed-payments/how-it-works: "It calculates and collects tax based on product classification and customer location, then files and remits payments to the relevant tax authorities." |
| Receipts | **Confirmed.** In a sandbox they are not sent on their own. | how-it-works: "These emails are sent from Link." / "The customer sees Link as the merchant of record, and sees purchases as *Sold through Link*." update-checkout: "In sandbox mode, you won't receive receipt emails automatically after purchase" |
| Trial reminders | **Confirmed, and sharper.** There are two emails: a trial-start email, and a trial-ending reminder for trials over 7 days, which covers the 14-day trial. The Dashboard toggle does not stop them. None are sent in a sandbox. | how-it-works: "A trial ending reminder. Stripe sends this in addition to the trial start email for all trials longer than 7 days. The Dashboard setting for trial ending emails doesn't apply to these emails." docs.stripe.com/billing/subscriptions/trials/manage-trial-compliance: "We don't send trial reminder emails in a sandbox." |
| Cancelling | **Confirmed.** Deleting the Link account also cancels the subscription. | how-it-works: "Cancel or update subscriptions" (Link website). stripe.com/checkout/terms (Sold Through Link Terms, last updated 2026-09-30): "any ongoing subscriptions that you purchased through an Order will be cancelled upon deletion of your Link Account." |
| Refunds | **Confirmed, with softer wording.** The 48-hour refund is a "might", and it applies when Stripe has asked you for input. | how-it-works: "Stripe can issue refunds within 60 days of the original transaction in certain cases." / "If you don't respond within 48 hours, Stripe might issue a refund without your approval." |
| Fee | **Confirmed: 3.5%.** | stripe.com/pricing: "per successful Managed Payments transaction in addition to Payments fees." |
| Subscriptions and webhooks | **Partly documented.** Subscriptions are created in your account and you can update them. `customer.subscription.*` delivery under Managed Payments is **not documented** (see Q5). | how-it-works: "You can issue refunds and update subscriptions in the Dashboard or with the API." |

## Q2. The account setting
- **Question as the plan words it:** what the account-level Managed Payments setting is, and whether it applies to a session that does not ask for it (D7's (i) or (ii)).
- **Answer.**
  - *What the setting is:* Managed Payments auto-enablement. It exists for accounts that joined Stripe after 10 July 2026 and turned Managed Payments on during onboarding. For eligible new Checkout Sessions and Payment Links it selects Managed Payments by default.
  - *Scope and override:* it is set separately for live mode and for the sandbox, and a value sent in the API call wins.
  - *So:* it can apply to a session that does not ask, which is (ii)'s premise. But (ii)'s claim that Managed Payments "accepted `automatic_tax`" contradicts Stripe's refusal list (Q4).
  - *What this means for 2026-09-24:* that session sent `automatic_tax[enabled]` and `customer_update[address]` (`billing-checkout/handler.ts:65-66`) and completed. So it was most likely not a Managed Payments session.
  - *Why it might have been left out:* Stripe does not say what makes a session "eligible". Possible causes are that the account predates the setting, that the product's tax code is not an eligible one, or that the session itself was ineligible.
  - *If it had been auto-enabled:* Stripe does not say whether such a session would have failed with an error or fallen back to an ordinary session.
- **Quotes** (support.stripe.com/questions/change-your-managed-payments-auto-enablement-setting, fetched 2026-11-21):
  - "Auto-enablement determines whether Managed Payments is selected by default for eligible new Checkout Sessions…" (cut by the tool at 125 characters)
  - "If you've joined Stripe after 10 July 2026, and you enabled Managed Payments during onboarding,"
  - "An explicit API setting takes precedence over the account auto-enablement setting."
  - Summarised, not quoted: in live mode the turn-on option "may be unavailable if you've already integrated" with relevant APIs.
- **Third party, not Stripe.** github.com/TryGhost/Ghost/pull/29715, merged 2026-08-03: "Stripe has been provisioning some new accounts with Managed Payments", "enabled by default for all Checkout Sessions". Their fix was `managed_payments: {enabled: false}` per session.
- **Confidence:**
  - Documented plainly: the setting exists, it is per environment, and an explicit API value wins.
  - Inferred: what happened on 2026-09-24.
  - Not documented: what happens to an auto-enabled session that carries refused keys.
- **Before S1 (Quinn, in the Dashboard):**
  - Settings › Managed Payments, live and sandbox: is auto-enablement on in each?
  - Does the 2026-09-24 payment show Managed Payments, "Sold through Link" and a `LINK.COM*` descriptor?
  - Is the live product's tax code labelled "Eligible for Managed Payments"?
- **S1 must still confirm:** that Managed Payments is activated in the sandbox, so an explicit `true` is accepted there.

## Q3. The session parameter
- **Question as the plan words it:** the exact per-session parameter, and whether it needs a preview API version.
- **Answer.**
  - *The parameter:* `managed_payments[enabled]=true` on `POST /v1/checkout/sessions`. Setting it to `false` turns Managed Payments off for that one session.
  - *Version:* today's guide asks for `2025-03-31.basil` or later. That is exactly the pin in `cloud/supabase/functions/_shared/stripe.ts:82`, so on the guide's words no change of version is needed.
  - *The tension:* the API changelog lists the parameter as "Added" under `2026-02-25.preview` and again under `2026-04-22.dahlia` (general availability).
- **Quotes:**
  - docs.stripe.com/payments/managed-payments/update-checkout (2026-11-21): "Use an API version of `2025-03-31.basil` or later." / "Set the `managed_payments[enabled]` parameter by updating your server's call to the Checkout Session API."
  - docs.stripe.com/payments/managed-payments/set-up: "To disable Managed Payments for a specific Checkout Session, set managed_payments[enabled] to `false`."
  - docs.stripe.com/changelog/clover/2026-02-25/managed-payments: "update your API requests to include `Stripe-Version: 2026-02-25.preview`"
  - docs.stripe.com/changelog/dahlia/2026-04-22/managed-payments: "When you set managed_payments.enabled to `true`, Stripe handles indirect tax compliance". It also says Stripe manages certain parameters, and "You can't manually specify those parameters."
  - Third party (Ghost PR): the parameter is "accepted under Ghost's pinned `2020-08-27` API version".
  - A search snippet claimed the guide says to "use a Preview API version". The page fetched today does not say that.
- **Confidence:**
  - Documented plainly: the parameter name.
  - Inferred: the version. The guide and the changelog disagree, and a third party reports it works on an older version.
- **S1 must still confirm:** a session created at the basil pin with `managed_payments[enabled]=true` gets no 400 for an unknown parameter. D7's "only the session call carries it" stays as the contingency.

## Q4. What Managed Payments refuses (`billing-checkout/handler.ts:51-72`)
- **Question as the plan words it:** which form keys it refuses, load-bearing ones first.
- **Answer.**
  - **Load-bearing keys: none refused.**
    - `consent_collection[terms_of_service]` (`:69`), `payment_method_collection` (`:60`) and `subscription_data[trial_period_days]` (`:59`) are not on the list.
    - Stripe also documents trials under Managed Payments, and documents `payment_method_collection=if_required` as allowed with it.
    - `customer` (`:55`) is not on the list, and Stripe describes passing one.
    - `subscription_data[metadata][account_id]` (`:57`), `client_reference_id` (`:56`) and `custom_text` (not in today's form; D8 adds it) are not on the list.
  - **Droppable keys:**
    - `automatic_tax[enabled]` (`:65`): **refused.**
    - `customer_update[address]` (`:66`): **refused.**
    - `billing_address_collection` (`:67`): **not refused, stays.**
    - `allow_promotion_codes` (`:63`): **not refused, stays.**
- **Quotes** (update-checkout, 2026-11-21):
  - Table rows: "`automatic_tax` | Managed Payments handles tax calculation and withholding for you." / "`customer_update[address]`".
  - On `customer`: "If you provide the ID of an existing customer_account or customer when you create the Checkout Session, that object is updated with any changes to the customer's name or billing address."
  - docs.stripe.com/payments/managed-payments/changelog, 2026-02-09: "You can set the payment_method_collection parameter to `if_required`…"
  - Same changelog, 2026-04-22: "Customers who purchase a Managed Payments subscription that starts with a trial period now receive a confirmation email after checkout".
- **Confidence:** the two refusals are documented plainly. "Not refused" is inferred from the keys being absent from Stripe's own list of refusals.
- **S1 must still confirm:**
  - The session is created with `managed_payments[enabled]=true`, without the two refused keys, and with `billing_address_collection=required`, `allow_promotion_codes=true` and `consent_collection[terms_of_service]=required`.
  - The terms checkbox and `custom_text` show on the page.

## Q5. Subscriptions and events
- **Question as the plan words it:** whether its subscriptions live in Knowlu's account, keep their metadata and send `customer.subscription.*` events to the endpoint.
- **Answer.**
  - *Account:* they live in your account. Stripe shows them in your Dashboard and lets you update them by API.
  - *Metadata:* `subscription_data[metadata]` is not refused (inferred kept).
  - *Events:* Stripe's Managed Payments pages list only the `checkout.session.*` events. Whether `customer.subscription.*` events reach your endpoint is not stated for Managed Payments.
  - *Risk:* if a customer asks Link to delete their data, Stripe cancels the subscription and deletes their data from Subscription objects in your account. The webhook finds the account by `metadata.account_id` (`stripe-webhook/handler.ts:93-94`), and if that data is gone a cancel event could arrive without it.
- **Quotes:**
  - how-it-works: "You can issue refunds and update subscriptions in the Dashboard or with the API."
  - Same page: Stripe "Deletes their data from any data objects (including objects in your Stripe account) … Subscription, and Charge objects."
- **Confidence:**
  - Documented: the subscriptions live in your account.
  - Inferred: metadata is kept.
  - Not documented: event delivery under Managed Payments.
- **S1 must still confirm:** (a) as written, and that `customer.subscription.created` arrives with `metadata.account_id`.

## Q6. The trial event
- **Question as the plan words it:** when `trial_will_end` fires, and whether moving a trial end to within 3 days fires it at once, in test mode and in live mode.
- **Answer.**
  - *When:* 3 days before the trial ends by default. The trials guide says this follows a Dashboard "Trial ending events" setting.
  - *Short or ended trials:* it fires immediately if the trial is shorter than that interval, if the trial is shortened to under 3 days, or if the trial is ended early.
  - *Test vs live:* not documented as different.
  - *Managed Payments:* nothing says it changes this event.
- **Quotes:**
  - docs.stripe.com/api/events/types: "Occurs three days before a subscription's trial period is scheduled to end, or immediately when a trial is ended early … If a trial is shortened so that fewer than three days remain, this event can fire immediately, including during the same transaction that collects payment."
  - docs.stripe.com/billing/subscriptions/trials: "Sent according to the **Trial ending events** setting in the Dashboard, which defaults to 3 days before the trial ends. If the trial is shorter than the configured interval, it triggers immediately."
- **Contradiction with D6** (`spec:216-218`):
  - D6 says: "Stripe sends that event three days before a trial ends".
  - Stripe says: the timing is a setting that defaults to 3 days.
  - D6 never writes "in 3 days" (`:234`), and its trialing / future-`trial_end` guard (`:219-221`) covers the early-end firing.
  - D6's citation URL now leads to the Trial Offers page; the older trial-period page is at `/billing/subscriptions/trials/free-trials`.
- **Confidence:**
  - Documented: the timing, and the immediate firing on an early end.
  - Not documented: whether test and live mode behave differently.
- **S1 must still confirm:**
  - The test-mode "Trial ending events" setting.
  - That moving the trial end to under 3 days fires the event, and the reminder arrives (S1 (c)).
  - The live setting gets the same Dashboard check.

## Q7. Display, portal and Link
- **Week × 4 on Checkout: not documented.** The Price object supports it: "`interval=month` and `interval_count=3` bills every 3 months" (docs.stripe.com/api/prices/object). S1 takes a screenshot.
- **Customer Portal: documented.** how-it-works: "You can also offer additional subscription management from your own website using the Customer Portal or your own solution." You cannot set `default_payment_method` by API on these subscriptions; the portal can.
- **What Link sends, and when.** The plain receipts, trial-reminder and data-deletion facts are in Q1's table and Q5. Additions:
  - Receipts, invoices and refund notices, with PDFs attached.
  - A trial-start email.
  - A trial-ending reminder for trials over 7 days. Its timing under Managed Payments is not stated; Stripe's general reminder goes "7 days before a trial ends" (manage-trial-compliance).
  - An upcoming-renewal email before every renewal, if "Upcoming renewals" is on.
  - Whether that setting is on or off, an anniversary email: "Before the subscription's 12-month anniversary for customers in all other countries … By default, Stripe sends anniversary emails 15 days before the anniversary date … must be sent at least 7 days before" (how-it-works).
- **Do refunds return the tax and the fee?**
  - Tax: "the refund includes any sales tax the customer paid … in these jurisdictions, your account balance will be reduced by the amount that corresponds to the original sales tax amount" (how-it-works).
  - Processing fee: "Stripe's processing fees from the original transaction aren't returned" (docs.stripe.com/refunds).
  - The 3.5% Managed Payments fee on refund: not documented.
- **Is `tax_behavior: exclusive` accepted?** Yes, documented: "Managed Payments uses the tax behavior specified on your price … If you don't specify the price's tax behavior, Managed Payments adds tax on top" (update-checkout).
- **Also documented:** Adaptive Pricing is always on, so a customer outside the US may pay in local currency.
- **S1 must still confirm:**
  - Screenshots of the week × 4 display.
  - The portal opens for a Managed Payments subscription.
  - The tax and fee lines in the balance after a refund.

## Q8. Moving Stripe to an LLC (Q4)
- **Answer: not documented for a direct US account.**
  - Stripe documents that business details can be edited.
  - Its only documented case needing a new account is a verified entity in Brazil or Japan, and that is written for connected accounts.
  - A change of business type can trigger a request for a new W-9.
  - Managed Payments access rests on a review that "considers factors such as business type".
- **Quotes:**
  - support.stripe.com/questions/change-in-circumstance-and-legal-documentation: "Relevant changes may include your name, address, tax ID, and business type."
  - support.stripe.com/questions/how-do-i-update-the-business-type-for-my-payee-on-their-tax-form: "If your legal entity has already been verified, you'll need to create a new account with the correct business type." (Brazil and Japan.)
  - docs.stripe.com/payments/managed-payments/eligibility: "Stripe determines access to Managed Payments based on an eligibility review that considers factors such as business type and geography."
- **Confidence:** inferred, or not documented.
- **S1:** nothing to confirm. Stripe Support answers this.

## Q9. Changing a subscription's price (D14 `move`)
- **Answer.**
  - *Proration `none`:* nothing is billed or credited now. The next invoice bills the full new price.
  - *Billing date:* the same interval and count keep the billing dates.
  - *Trialing subscriptions:* updates during a trial are allowed, and are non-proration if trial status is unchanged. That the trial's first charge then bills the new price is inferred, not stated.
  - *Under Managed Payments:* updating subscriptions is allowed. Price changes are not addressed specifically.
  - *Link's own notice:* Link's terms impose no price-change notice and say nothing on one. But Link's anniversary email, at 15 days by default, goes out whatever the settings.
- **Quotes:**
  - docs.stripe.com/billing/subscriptions/prorations: "When prorations are disabled, customers are billed the full amount at the new price when the next invoice is generated." / "Debit from updating a free trial period without changing its trial status | ❌ false" (not a proration).
  - docs.stripe.com/billing/subscriptions/change-price: "If both prices have the same billing periods (combination of `interval` and `interval_count`), the subscription retains the same billing dates."
  - docs.stripe.com/billing/subscriptions/trials/free-trials: "You can update subscriptions in a trial normally."
  - Sold Through Link Terms: no price-change clause, only "you may be charged different amounts in Local Currency on each subscription payment."
- **Risk for D14** (inferred): for a yearly `move` with N under 15, Link's anniversary email could state the old price before Knowlu moves it.
- **No stop condition.** A no-proration change does not bill or credit the current period, so T3's move step does not stop.
- **Confidence:**
  - Documented: outside a trial.
  - Inferred: on a trial, and under Managed Payments.
  - Not documented: what Link's emails say about price.
- **S2 must still confirm:**
  - The current invoice is unchanged after the move.
  - The renewal bills the new price on the same date, for an active subscription and a trialing one, both under Managed Payments.
  - What Link's renewal and anniversary emails say. That needs live mode.

## Q10. An archived price (D14 `stay`)
- **Answer.**
  - *Renewal:* subscriptions on an archived price keep going.
  - *Re-selling:* an archived price can be unarchived. It is the same Price object, so the same id.
  - *Listing:* the subscriptions list filters by `price` and accepts `status=all`.
  - *Portal:* whether it shows an archived price is not documented.
  - *Trap:* the list leaves out test-clock subscriptions unless `test_clock` or `customer` is passed. That matters for S2 if it uses test clocks.
- **Quotes:**
  - docs.stripe.com/products-prices/manage-prices: "If you archive a price, any existing subscriptions that use the price remain active until they're canceled" / "To use the API to unarchive a price … change the active parameter to `true`."
  - docs.stripe.com/api/subscriptions/list:
    - "`price` … Filter for subscriptions that contain this recurring price ID."
    - "Passing in a value of `all` will return subscriptions of all statuses. If no value is supplied, all subscriptions that have not been canceled are returned."
    - On `test_clock`: "The response will not include subscriptions with test clocks if this and the customer parameter is not set."
- **Confidence:** documented plainly, except the portal display (not documented).
- **S2 must still confirm:** a renewal on an archived price, and what the portal shows.

## Q11. Subscription metadata (D14)
- **Answer.**
  - *Merge:* updating metadata merges keys. Writing one key leaves `metadata[account_id]` alone, and an empty value deletes only its own key. So no read-back and resend is needed.
  - *Event:* that a metadata-only write fires `customer.subscription.updated` is inferred from "whenever a subscription changes". The webhook would rewrite the same entitlement (`stripe-webhook/handler.ts:97-101, 165`) and answer 200.
  - *Under Managed Payments:* that the metadata is kept is not documented.
- **Quotes:**
  - docs.stripe.com/metadata: "This parameter uses a merge mechanism, which allows you to add new key-value pairs to an object in an update call without affecting any existing metadata."
  - docs.stripe.com/api/events/types: "Occurs whenever a subscription changes (e.g., switching from one plan to another, or changing the status from trial to active)."
- **Confidence:**
  - Documented: the merge.
  - Inferred: the event.
  - Not documented: Managed Payments keeping the metadata.
- **S2 must still confirm:** the event arrives after the mark is written, the webhook answers 2xx, and `account_id` is unchanged.

---

## Status table

| Item | exists / partial / missing | Evidence | Size |
|---|---|---|---|
| Q1 §3 rows | partial: 7 confirmed, 2 partly contradicted | Q1 table | S, 1 file (the note), no |
| Q2 account setting | exists (auto-enablement); its effect on 09-24 is missing | support article; Ghost PR (third party) | Dashboard read; no code |
| Q3 parameter and version | exists; version partial | update-checkout vs changelog | S, `billing-checkout/handler.ts` + `handler_test.ts`, no |
| Q4 refusals | exists: 2 droppable refused, 0 load-bearing | update-checkout table | S, same 2 files, no |
| Q5 subscriptions and events | partial | how-it-works | S1 only |
| Q6 trial event | exists; test/live missing | events types; trials page | S, D6 wording, no |
| Q7 display / portal / Link / refund / tax | partial: week × 4 and the Managed Payments fee refund missing | how-it-works, refunds, pricing | S1 only |
| Q8 LLC | missing for a direct US account | support articles | L if a new account: secrets, prices, webhook, two projects; no |
| Q9 price change | exists; trial and Managed Payments partial; Link notice missing | prorations, change-price | T3 as planned, plus a yearly N floor in D14: S, no |
| Q10 archived price | exists; portal missing | manage-prices, list API | none |
| Q11 metadata | exists (merge); event inferred | metadata guide | none; T3 writes its own key only |

## Questions for Quinn

1. **The Dashboard check before T1 merges.** In Settings › Managed Payments, live and sandbox: is auto-enablement on? Does the 2026-09-24 payment show "Sold through Link"? Is the product's tax code marked eligible? *Recommendation:* do it, but don't let it block T1. T1 sends `managed_payments[enabled]=true` explicitly and drops `automatic_tax[enabled]` and `customer_update[address]`.
2. **The Q10 fallback.** Turning the account setting off in live mode may not be reversible. *Recommendation:* if a gate fails, fall back with `managed_payments[enabled]=false` per session, which overrides the account setting and needs no change to it. Never use the account switch.
3. **Link emails S1 cannot see.** The sandbox sends no trial reminders or receipts. *Recommendation:* move "record Link's notice" from S1 to the live step 10b.
4. **Yearly `move` vs Link's 15-day anniversary email.** *Recommendation:* in D14, require N ≥ 16 for a yearly `move` entry, and keep "Upcoming renewals" off.
5. **Q4 (LLC).** *Recommendation:* ask Stripe Support whether a US direct account with Managed Payments active can change business type in place, before planning a new account.

Pages read (all fetched 2026-11-21):
- [update-checkout](https://docs.stripe.com/payments/managed-payments/update-checkout)
- [how-it-works](https://docs.stripe.com/payments/managed-payments/how-it-works)
- [set-up](https://docs.stripe.com/payments/managed-payments/set-up)
- [eligibility](https://docs.stripe.com/payments/managed-payments/eligibility)
- [Managed Payments changelog](https://docs.stripe.com/payments/managed-payments/changelog)
- [2026-02-25 changelog](https://docs.stripe.com/changelog/clover/2026-02-25/managed-payments)
- [2026-04-22 changelog](https://docs.stripe.com/changelog/dahlia/2026-04-22/managed-payments)
- [auto-enablement](https://support.stripe.com/questions/change-your-managed-payments-auto-enablement-setting)
- [Checkout Session create](https://docs.stripe.com/api/checkout/sessions/create)
- [event types](https://docs.stripe.com/api/events/types)
- [trials](https://docs.stripe.com/billing/subscriptions/trials)
- [free-trials](https://docs.stripe.com/billing/subscriptions/trials/free-trials)
- [trial compliance](https://docs.stripe.com/billing/subscriptions/trials/manage-trial-compliance)
- [change-price](https://docs.stripe.com/billing/subscriptions/change-price)
- [prorations](https://docs.stripe.com/billing/subscriptions/prorations)
- [manage-prices](https://docs.stripe.com/products-prices/manage-prices)
- [Price object](https://docs.stripe.com/api/prices/object)
- [subscriptions list](https://docs.stripe.com/api/subscriptions/list)
- [subscription update](https://docs.stripe.com/api/subscriptions/update)
- [metadata guide](https://docs.stripe.com/metadata)
- [metadata API](https://docs.stripe.com/api/metadata)
- [refunds](https://docs.stripe.com/refunds)
- [pricing](https://stripe.com/pricing)
- [Consumer / Sold Through Link Terms](https://stripe.com/checkout/terms)
- [change in circumstance](https://support.stripe.com/questions/change-in-circumstance-and-legal-documentation)
- [business type](https://support.stripe.com/questions/how-do-i-update-the-business-type-for-my-payee-on-their-tax-form)
- [Ghost PR #29715](https://github.com/TryGhost/Ghost/pull/29715) (third party)

---

Filed 2026-11-21 by the main session from the `researcher` run of T0r (read-only, web sources only; no Stripe API call, nothing signed in to). Checkpoint A reads this report before T1's brief is written.
