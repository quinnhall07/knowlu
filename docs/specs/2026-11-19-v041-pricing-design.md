# v0.4.1 pricing: design note

Planner, 2026-11-19; revised 2026-11-20. **Size: Contract**: it moves money, it changes
`app/src/account.rs` (contract list: `TOS_VERSION` and the checkout command), it publishes new terms, it
adds a publishing workflow (T10), and, if Managed Payments ships, it changes the privacy page.
**Status: r5.** r2 answered review r1 (`docs/reports/2026-11-19-v041-pricing-spec-review-r1.md`). r3
applied review r2's fixes (`docs/reports/2026-11-19-v041-pricing-spec-review-r2.md`, PASS with changes);
§9 maps each finding to its fix. **r4 revised r3 for Quinn's ruling of 2026-11-20** (zero customers; prices
that can change later). §10 lists every decision and task r4 changed. **r5 applies the r4 delta review**
(`docs/reports/2026-11-20-v041-pricing-r4-review.md`, PASS with changes) and the signing of Amendment
2026-11-17; §10's *Revision r5* maps each finding to its fix. Plan: `docs/plans/2026-11-19-v041-pricing-plan.md`.

**Authority.** Quinn's ruling of 2026-11-19: pricing is fixed in v0.4.1, straight after the v0.4.0 tag.
The prices are the cloud design's Amendment 2026-11-10 (signed 2026-11-11), as Quinn's rulings of
2026-11-17 amend it (quoted as R2-R4, U5, U6, U10-U12 in `docs/specs/2026-11-17-legal-pages-rewrite.md`
§0, "the legal spec"):
- **$12.99 every 4 weeks** (13 charges a year), or **yearly at $84.37 once a year**, shown first as "$6.49
  every 4 weeks, billed $84.37 once a year. Renews yearly." (ruling 2);
- **14 days free**, card taken first, **a reminder 3 days before the first charge** (ruling 7, P14);
- **no summer pause**; **no Pell price** (U6); the 30-day money-back on the **yearly plan only** (ruling 8,
  P12);
- prices are **tax-exclusive** and shown "plus tax where it applies" (U12, P9);
- **Managed Payments, with Link as seller of record** (R2, U5, U12);
- **P9:** "The app's subscribe step and Stripe Checkout state the every-4-weeks billing …, the billed amount
  …, on yearly the money-back terms …, all before the card is entered."

**Quinn's ruling of 2026-11-20** (verbatim): "Again, we have zero customers right now. Just use the new
prices. Make the changes that are necessary, and make sure that if we change prices in the future (lower
or raise), we can make the proper adjustments". Amendment 2026-11-17, **signed by Quinn on 2026-11-20**
(PR #83), records it as rulings (6) and (7) (`docs/specs/2026-09-09-knowlu-cloud-design.md:1368-1379`, read
in the `price-amend` worktree at `:1234-1550`):
- **(6) The old plans are retired.** No paying customer exists, so nothing is kept for an old subscription.
  `monthly` ($9.99) and `academic_year` ($69.99) are not sold again and not carried forward.
- **(7) A price can change later, up or down** (Quinn's words, `:1378-1379`). The amendment's *Reading (7)*
  (`:1395-1404`) reads them as: one written procedure; each plan's amount in one place on the server, read
  by every surface the code builds; each later change its own amendment, which says what happens to
  students already subscribed. Whether a change also needs an app release or a terms release is left to
  this lane's design and runbook (`:1400-1402`); D15 and D16 decide it. *The promises every later price
  change keeps* (`:1406-1415`) bind D14.

The same day Quinn answered §8 item 1 yes, and the quote answers items 3 and 9. The amendment's (8) to (10)
(`:1444-1462`) settle §8 items 5, 6 and 7 as this note recommended.

**When rule A2 ends.** The v0.4.0 note's A2 (`docs/specs/2026-11-17-v0.4.0-new-ui-design.md:24-27`) kept
prices out of the wizard "until" billing moved to the signed prices in a later lane. This is that lane, so
A2's wizard clause ends here and P9 governs the subscribe step. D9 shows P9's lines. The amendment's (8)
settles it (§8 item 5). A2's homepage clause ("to be announced") is a separate ruling, and it stands.

The gift year (ruling 9, P13) has its own spec and lane (legal spec C24) and is not built here. The v0.4.0
note (§13) and the legal spec (§5) place the billing move in v0.5.0. Quinn's 2026-11-19 ruling is later,
so on *when* billing moves, this note wins.

**Limits of what I checked.**
- I had no shell and no web tools.
- **The amendments are not on `main` yet.** Amendment 2026-11-10 was read from the `docs/waitlist-spec`
  worktree (`docs/specs/2026-09-09-knowlu-cloud-design.md:993-1232` there); Quinn ruled on 2026-11-20 that it
  goes to `main` (§8 item 1). Amendment 2026-11-17 is signed (2026-11-20) and goes to `main` by PR #83. T1a
  waits until both are on `main`.
- **I opened nothing on Stripe's site.** Every Stripe statement here is marked `unverified (Stripe)` and
  names its page. Plan task T0r confirms each one, and the test-mode spike S1 tests it.
- **Production's state is not known from the repository.**
  - `HANDOFF.md:598` says production holds C1's migrations of 2026-09-14 "and nothing later".
  - `20261020000600_comp_guards.sql:2-3` says the testers migration "is applied on staging and prod".
  - `HANDOFF.md:70` records sixteen functions deployed to production on 2026-11-10.

  T0 reads production's migration and function lists before anything depends on them (§4, step 0).
- Code facts are cited `path:line` against this worktree, and I opened each one. r4's new citations were
  read on 2026-11-20.

## 1. What changes for the student

- **The wizard's Subscribe step** shows the two plans, Yearly first. Each plan is a button with its price,
  as the server sends it (D9, D13). Today the server sends:
  - **Yearly:** "$6.49 every 4 weeks, billed $84.37 once a year. Renews yearly.", then "30-day money-back
    guarantee on each yearly charge."
  - **Every 4 weeks:** "$12.99 every 4 weeks."
  - **Below both:** "Plus tax where it applies."

  The amendment's (8) settles this (D9). The step does not mention the trial, because only
  the server knows whether this account still has one (D5); Stripe's page states it.
- **Without a connection, or if the server refuses,** the step shows no price, the buttons wait, the step
  says which of the two it is (D9), and "Try again" asks again. The app never shows a price it did not just
  get from the server.
- **The upgrade sheet** offers the same two plans, with the same lines (D9; the amendment's (9)).
- **Stripe's checkout page** shows:
  - the plan's price;
  - 14 days free, on the account's first subscription;
  - tax added on top;
  - one renewal sentence, which on the yearly plan names the money-back guarantee (D8). *Changed
    2026-11-27:* Stripe refuses that sentence under Managed Payments, so the page shows Stripe's own
    renewal lines instead (the paragraph under D8).

  Under Managed Payments it also says "Sold through Link".
- **Before the first charge**, Knowlu emails the student the plan, the amount, the date and how to cancel.
- **Settings → Membership** reads, for example, "Active, Yearly" or "Active, Every 4 weeks", with no price.
- **knowlu.com** carries a new terms version that states these prices. The page after checkout says 14 days.
- **A build older than v0.4.1** that tries to subscribe is told to update Knowlu.
- **A later price change** reaches every build from v0.4.1 on with no update (D13, D15). A student already
  subscribed either keeps their price or is emailed 7 to 30 days before it changes at their next renewal, as
  that change's amendment rules (D14).
- Nothing changes for a tester on a comp grant, and nothing changes in the engine.

## 2. Decisions, each with its reason

**D1. Two plan keys, stable across every price change** (revised r4). Subscriptions are `four_weekly` or
`yearly`. The old keys `monthly` and `academic_year` are retired (ruling 6): checkout never sells them
(D3), the webhook never writes them (D2 maps no old price), and they leave `billing-jobs` (D10) and
Settings (D9).
- **A key outlives its price.** A price change is a new Stripe Price and a catalog entry under the same key
  (D13), never a new key. The key fixes the cadence, so the app's plan words, Settings and the webhook's
  output stay right across every change.
- **Why not reuse `monthly`.** The amendment's consequences say the 4-week plan "does not take the name
  `monthly`", and it is not billed monthly. (r3's other reason, the summer pause, is gone with the pause,
  D10.)
- **No migration is needed.** `entitlements.plan` is free text with no check
  (`20260910000100_accounts.sql:27`).
- **Testers and the gate are unaffected.** `tester` comes from the view (`20261020000500_testers.sql:76-87`),
  and the gate reads status only (`_shared/entitlement.ts:22, 79-81`).

*Removed in r4 (zero customers):* r3's reading that an `academic_year` row keeps its name and $69.99 and
counts as yearly for the money-back (U10), and §8 item 3 that asked Quinn to confirm it. Ruling (6)
answers it: no such subscription is kept.

**D2. The plan comes from the price id, through the catalog** (revised r4). The webhook looks up
`items.data[0].price.id` among the catalog's selling and kept entries (D13) and writes that entry's key.
- **At v0.4.1 it reads two values**, `STRIPE_PRICE_FOUR_WEEKLY_1299` and `STRIPE_PRICE_YEARLY_8437`, whose
  names the catalog derives (D13). It reads neither old value (`STRIPE_PRICE_MONTHLY`, `STRIPE_PRICE_YEAR`,
  `billing-checkout/index.ts:16-19`): no customer pays those prices (ruling 6), and §4 ends Quinn's own
  before the new webhook deploys. They are unset after the tag.
- Any other id gives `plan: null`, which `billing-jobs` counts as a failure without mailing a guessed price
  (`billing-jobs/handler.ts:69-75, 119-123`).
- **Every value is optional in the webhook.** A missing one matches nothing and is logged once per cold
  start, never answered 500, so one unset value cannot take down every event. Checkout requires its
  selling values (D4).

*Reasons:*
- After a price change two prices share one key and one cadence (D14's `stay`), so only the id tells them
  apart. The interval never could: today a `week` interval maps to `null` (`stripe-webhook/handler.ts:66-67`).
- Price ids are public, set per project (`docs/plans/2026-09-09-c1-accounts-plan.md:2207-2209`).

*Removed in r4:* the two old values, which existed only to name old-price subscriptions.

**D3. Checkout sells only the offer the app showed** (revised r4). `billing-checkout` sells the catalog's
selling plans (D13). A purchase request carries `plan`, `amount_cents` and `terms_version`, the last two
copied from the offer the app rendered (D9). Validation runs in this order:

1. **Old plan names.** A request for `monthly` or `academic_year` is refused with 409 and the sentence "This
   version of Knowlu can't subscribe any more. Update Knowlu, then try again."
2. **Unknown plan names** stay a 400.
3. **The offer.** `amount_cents` and `terms_version` are required (400 if either is absent) and must equal
   the catalog's cents for that plan and its `TERMS_VERSION`. Anything else is refused with 409: "Knowlu's
   prices or terms have changed. Look them over again, then subscribe." The app shows the sentence and asks
   for the offer again (D9).

These checks read only the request, so they come first: after the session check and the parse
(`billing-checkout/handler.ts:90-93`), and before the account read, the 18+ check and the tester check
(`:98-107`). The app shows the server's sentence as written (`app/src/account.rs:210-217`). The consent row
then records the catalog's cents and `TERMS_VERSION` (`handler.ts:120-127`), which equal what the app showed.

*Reasons:*
- **Kept: the old names' 409, though it protects no sale.** The rename alone already stops an old build
  buying: without this step `monthly` is an unknown name and gets 400. It stays for three reasons. It gives
  a pre-v0.4.1 build a sentence a student can act on. Its exact body is how §4 step 10b tells the new
  function from the tester refusal. And with stable keys (D1) the list never grows. Only Quinn's installs
  meet it (v0.4.0 note, A1).
- **Consent integrity, by match rather than floor.** Every earlier build sends 2026-09-10
  (`account.rs:24, 1300`), and the old names are refused before anything else. From v0.4.1 the student
  consents to the amount and terms version the server named and the app showed. If the price changed
  between showing and clicking, the 409 shows the new one before Stripe's page (P9).
- **Only the catalog's own version can be written to `consents.version`**, so a malformed or future value
  never is, with no pattern and no date rule.
- **No build is refused for its age**, so a terms bump needs no release (D15).

*Removed in r4:* r3's floor (`CHECKOUT_TERMS_VERSION`, tied to `TOS_VERSION`), its consequence that every
later terms bump refused every older build, the shape check and its "2026-11-19x" case.

**D4. Checkout checks the price before selling it** (r4: against the catalog). The catalog gives each selling
plan its price variable, its cents and its cadence (D13): `four_weekly` is 1299, week × 4, and `yearly` is
8437, year × 1. Today's map in `index.ts:16-20` goes. Before the consent row is written, the function reads
the price from Stripe and requires six things:
1. the price is `active`;
2. the currency is `usd`;
3. `unit_amount` equals the plan's cents;
4. `recurring.interval` is the plan's own interval: `week` for `four_weekly`, `year` for `yearly`;
5. `recurring.interval_count` is the plan's own count: 4 for `four_weekly`, 1 for `yearly`;
6. `tax_behavior` is `exclusive`.

The pair is tied to the plan, not just to a list of allowed pairs. Otherwise a $12.99 price created as
yearly by mistake would pass under `four_weekly`.

Any failure answers 500 "the function is not configured" and logs which field failed, by name only.

*Reason:* `price_cents` comes from the catalog, not from Stripe. Without the check, a price entered wrongly
in the Dashboard would be sold silently under terms that say something else. That covers a wrong interval,
an inclusive price under terms that say tax is added (U12), and, at a later price change, a variable set to
the old price's id. A selling variable that is unset answers the same 500. The check costs one Stripe GET.

**D5. 14 days free, once per account, and no second live subscription.** The checkout reads the account's
`entitlements` row (`stripe_subscription_id`, `status`; `20260910000200_billing_jobs.sql:13`):
- **A row with a subscription id whose status is `active`, `trialing`, `past_due` or `unpaid`** is refused
  with 409: "This account already has a subscription. Manage it in Settings, then Membership." The refusal
  comes before the customer and the consent row. A student whose card failed is sent to Manage
  subscription to fix it, not to a second subscription while the first is still dunning.
- **No row with a subscription id:** the session asks for `subscription_data[trial_period_days]` = 14.
- **Any other row with a subscription id** (`canceled`, `incomplete_expired`, and so on): no trial.

**Open pages are expired first (added 2026-11-27, on the founder's yes).** The row above is written only
when a checkout completes, so two pages opened before either is paid would both pass these checks. Once the
customer is known, and before the consent row, the checkout lists that customer's open Checkout Sessions and
expires each one. A list shorter than 100 ends it; a full list is read again, skipping ids already expired;
five rounds, or a full round with nothing new, is a refusal. Any failure answers 502, writes no consent row
and creates no session. A customer created in this request has no pages and is not listed.

`payment_method_collection: always` stays (`handler.ts:60`), for the reason its test gives
(`handler_test.ts:22-25`).

*Reasons:*
- Ruling (7), and P3's one trial per account.
- Two checkouts in parallel would otherwise give two trials and two subscriptions. Account deletion sets
  only the one stored id to cancel at period end (`account/handler.ts:88-94`), so the second would bill a
  deleted account. The app asks before opening checkout; this is the server's backstop.

*Known limits:* a deleted account's address can trial again on a new account. Two requests in the same
instant, or a second checkout opened in the seconds between paying and the webhook's write, can still give
two subscriptions; closing that needs a guard in the webhook, which is not built. A checkout page still open when an account is deleted is
not expired by the delete; a payment completed on it afterwards needs a refund by hand (added 2026-11-27).

**D6. The reminder is Knowlu's own email, sent by the webhook** on Stripe's
`customer.subscription.trial_will_end` event. Stripe sends that event three days before a trial ends
(`unverified (Stripe)`: docs.stripe.com/billing/subscriptions/trials).
- **When it is sent.** Only while the subscription is `trialing`, is not set to cancel at period end, and
  has a `trial_end` still in the future. A retry that arrives after the trial ended sends nothing and is
  recorded.
- **What it says.**
  - The plan's words and the amount, from the catalog entry the event's price id matches (D13), through the
    catalog's one formatter: "$84.37 once a year" or "$12.99 every 4 weeks" (r4).
  - For an id the catalog lacks: no plan words, and the amount formatted by the same formatter from the
    event's own price (`unit_amount`, `currency`, `recurring.interval`, `recurring.interval_count`). Any
    other interval is written as "every N weeks", "every N months" or "every N years", with N = 1 written
    without a number.
  - "plus tax where it applies".
  - The date of the first charge, written in UTC−10. That is never later than the student's own date
    anywhere in the fifty states.
  - How to cancel: Settings, then Membership, then Manage subscription, or a reply to the email.

  It never says "in 3 days". An unknown price id still gets the email, without plan words: skipping it
  would leave the first charge refundable under the terms' case (c).
- **If the email fails.** The webhook answers 502 and records nothing, so Stripe retries. The failure log
  names the account id only, as `billing-jobs` does (`billing-jobs/handler.ts:173-175`).

*Reasons:*
- P14 is Knowlu's promise.
- The event arrives once per trial, `webhook_events` removes duplicates, and no migration is needed.
- The amendment's consequences list the reminder under `billing-jobs` (`:1198-1201` of the amendment). That
  list is marked "not done by this amendment" and leaves mechanisms to the plan. A daily scan there would
  need a new column and a change to the view, for the same email.
- The event writes no entitlement, so the webhook stays the only writer of `entitlements`
  (`stripe-webhook/handler.ts:2-4`).

If Link or Stripe also sends a notice (S1 records it), Knowlu's still goes.

**D7. Managed Payments in checkout (legal spec C5), starting from what the record already says.**
- **In live mode it is already on.** On 2026-09-14 Quinn "kept Managed Payments on (P3)" (`HANDOFF.md:80,
  629`), and the amendment repeats it (`:1131, :1157` there).
- **A live checkout has already worked.** On 2026-09-24 one completed with today's form, which includes
  `automatic_tax[enabled]`. The webhook wrote the entitlement about 4 seconds later (`HANDOFF.md:63`). It
  was paid at $0 with the 100% code, so no tax or fee was charged (amendment `:1159`).

That record allows two readings, and T0r settles which is true:

| Reading | What happened on 2026-09-24 | What D7 does |
|---|---|---|
| **(i)** The account setting does not make a session Managed Payments unless the session asks for it. | That checkout was an ordinary Stripe Tax session. | The session adds the Managed Payments parameter and drops what Managed Payments refuses. |
| **(ii)** The setting applies to every session, and Managed Payments accepted `automatic_tax`. | That checkout already proves, in live mode, the webhook path S1 (a) tests. | The form needs no change for Managed Payments. **Link is already the seller of every live session**, so D11's Link line and D12 ship regardless. |

Quinn can tell the two apart from the live Dashboard: does the 2026-09-24 payment show Link as the seller?

**"Managed Payments is in effect"** means reading (ii), or reading (i) with the per-session change. D11's
Link line, D12 and the step-10b check of "Sold through Link" all follow that condition, not whether the
form changed.

**Which parameters may go.** Only these may be dropped at checkpoint A, and only if T0r quotes Stripe
refusing them under Managed Payments:
- `automatic_tax[enabled]`, `customer_update[address]` and `billing_address_collection`: Link collects
  the tax and the address;
- `allow_promotion_codes`: P10 makes comp grants the comp mechanism, so no promise rests on it.

**Which parameters are load-bearing, and never dropped by an implementer:**
- `consent_collection[terms_of_service]`, which the consent row presumes (`handler.ts:6-8, 68-69`);
- `payment_method_collection` ("card taken first", `handler_test.ts:22-25`);
- `subscription_data[trial_period_days]` (ruling 7);
- `customer`, which the portal and account deletion depend on (`handler.ts:109-118`, `account/index.ts:82-96`,
  `account/handler.ts:88-94`);
- `subscription_data[metadata][account_id]` and `client_reference_id`, by which the webhook finds the account
  (`stripe-webhook/handler.ts:92-94`);
- `custom_text` (D8, P9). *Changed 2026-11-27:* Stripe refuses it in every slot under Managed Payments,
  so the session no longer sends it; the founder ruled so the same day (the paragraph under D8).

If Managed Payments refuses any of these, that is a checkpoint B failure for Quinn, with the fallback below.

If Managed Payments needs another API version, only the session call carries it. The pin in
`_shared/stripe.ts:82` stays, because the webhook parses events at that version and seven functions bundle
the file.

**S1's gate (test mode, on staging).** Live mode is on record. Test mode is not, so S1 must show five things:

| | S1 must show |
|---|---|
| (a) | A checkout writes the entitlement through Knowlu's webhook, with the right plan and trial end, for both plans. |
| (b) | The card is taken, with 14 days free on a first subscription and none on a second. |
| (c) | `trial_will_end` is delivered, and the reminder arrives. |
| (d) | Manage subscription cancels in one click. |
| (e) | Deleting the account sets its subscription to cancel at period end, with no invoice (`account/handler.ts:88-94`). |

**Fallback, if any gate fails.** It depends on the reading:
- **Under reading (i), recommended:** ship v0.4.1 without the per-session change and without D12. The
  form stays today's Stripe Tax form, the terms say "Stripe", and the per-session change moves to v0.5.0.
- **Under reading (ii)** there is no form change to skip. The only fallback is Quinn turning the account
  setting off in live mode before step 10, and step 10b then reads that the page does **not** say "Sold
  through Link". That reverses a signed choice (R2, U5), so it is not recommended. Recommended instead:
  ship the Link texts.
- **Either way,** the alternative is to hold the release.

**D8. Stripe's page states the renewal terms.** The session sets `custom_text[submit][message]` from a pure
function of the plan and of whether a trial applies:
- **Every 4 weeks:** "Billed every 4 weeks[ after 14 days free], plus tax where it applies, until you
  cancel. Cancel any time in Knowlu: Settings, Membership, Manage subscription."
- **Yearly:** "Billed once a year[ after 14 days free], plus tax where it applies, until you cancel. 30-day
  money-back guarantee on each yearly charge (knowlu.com/terms)."

*Reason:* P9 names Stripe Checkout as well as the app.

The cadence words are the key's, which never change. The tax phrase and the money-back sentence come from
the catalog's words (D13), so the two cannot drift from the app's lines (r5: m2).

**Refused by Stripe under Managed Payments (found 2026-11-27 on staging; ruled by the founder the same day: Managed
Payments stays, and the sentence is not sent).**
Stripe answers "You cannot use custom_text with Managed Payments." for every `custom_text` slot, so the session no
longer sets one. What Stripe's own page states without it, as read on the sandbox: "14 days free" on a first
subscription; "Then $12.99 every 4 weeks starting" or "Then $84.37 per year starting" with the first charge's date;
the tax line; "Total due today $0.00"; a required checkbox agreeing to the seller's Terms of Service and Privacy
Policy; and "By subscribing, you agree to Link's Terms and Privacy Policy and authorize Link to charge you for the
described subscription until you cancel." What it does not state: how to cancel inside Knowlu, and the yearly plan's
money-back guarantee. Both are on the app's Subscribe step before the page opens ("Cancel any time in Settings." and
the yearly line's guarantee sentence) and in the terms.

**D9. The app's subscribe step and labels: the app shows what the server sends** (revised r4).
- **The offer, from the server.** `GET /billing-checkout`, with the session, answers the catalog's selling
  plans in order (Yearly first), each with its key, its `amount_cents` and its P9 lines, plus the catalog's
  `terms_version` and the line "Plus tax where it applies." It reads no account. A new command,
  `checkout_offer`, in `app/src/account.rs` beside `open_checkout`, reads it with the pending session, as
  `entitlement_now` does (`account.rs:1191-1206`), and hands the answer to the page unchanged. It joins
  `account::open_checkout` in both `generate_handler!` lists (`app/src/main.rs:181, 270`).
  - *Why `billing-checkout`'s GET:* the function that sells reads the same catalog in the same deploy, so
    what is shown and what is charged come from one bundle; no new function is deployed or listed; it
    checks the session itself, as every function here does (`cloud/supabase/config.toml:101-109`). Not the
    entitlement reply: its four keys are the C2 contract the app caches (`account.rs:879-888`), and every
    slot calls it.
- **The purchase.** `open_checkout` takes the chosen plan's `amount_cents` and the offer's `terms_version`,
  and `checkout_url_at` sends them with the plan (`account.rs:1292-1301`, which today sends `TOS_VERSION`).
  The sign-up consent retry in front of it is unchanged (D15).
- **The wizard** (`app/static/js/wizard.js:32-34`).
  - "Yearly" comes first, red (`ibtn`), and sends `yearly`. "Every 4 weeks" comes second (`wbtn`) and
    sends `four_weekly` (`:641-646`).
  - On entering the step, after the entitlement check (`:384-393`) answers not entitled, it asks for the
    offer. Each button's lines, and the line below both, are the offer's, set as text, never as markup.
  - The lede drops "and shows the price".
  - The element ids stay the same: no student sees them, and renaming them would churn the DOM checks.
  - *Reason:* P9 governs once A2's wizard clause ends. Ruling (2) gives the order and the yearly wording,
    and P11 sets the figure ($6.49 × 13 = $84.37), with the billed amount beside it. The catalog writes
    those lines (D13).
- **No offer, no figure** (r5: m1). Until an offer has arrived the plan buttons are disabled and carry no
  figure. If none can be had, the step says why, beside a Try again button:
  - the server could not be reached: "Prices could not be loaded. Check your connection, then try again."
    (an error that starts with `UNREACHABLE`'s stable clause, `account.rs:175-178`, which exists so the page
    can tell this case apart);
  - the server refused (a 401 from an expired pending session, a 5xx): the command's own `error` sentence,
    which `provider_error` takes from the reply (`account.rs:210-217`). Every refusal names its reason, and
    Try again does not loop on a cause a retry cannot fix silently;
  - an answer that lacks either plan: "Prices could not be loaded. Update Knowlu, then try again." (the
    keys are compiled into the app; see D16 on plan changes).

  Nothing is cached: no file, no storage, and each entry to the step asks again. A checkout refusal (D3, D5)
  shows the server's sentence and asks for the offer again. *Reason:* never a stale or invented price, and a
  checkout needs the network anyway.
- **The upgrade sheet** (`app/static/js/carried/account.js:83, 129, 134-137`) gets the same two buttons and
  lines in place of its one Subscribe button, and asks for the offer where today it shows that button
  (`:129`). *Reason:* the amendment prices the 4-week plan "to make the year the obvious buy" (`:1111-1112`
  there). A sheet that sells only that plan sends a lapsed student to the one the price steers away from.
  The amendment's (9) replaces the 2026-09-14 one-plan ruling (`HANDOFF.md:80`) with this (§8 item 6,
  settled).
  - Only the two buttons start a checkout. Today's handler starts `subscribe()` from any click inside
    `#up-subscribe` (`carried/account.js:214`), so a click on the price lines would open a checkout with no
    plan.
- **Settings** (`settings.js:32`) words `four_weekly` "Every 4 weeks" and `yearly` "Yearly". `monthly` and
  `academic_year` go: no row will carry them (ruling 6), and a plan without a word reads as none
  (`settings.js:184`). It still shows no price.
- **The app holds no price.** No amount, figure or price line is written anywhere in `app/static/js`; it
  holds the two plan keys and their words. So a price change reaches every build from v0.4.1 on with no
  update (ruling 7).
- **The Customer Portal** offers no plan switching, because the terms describe none.
- *Settled by the amendment's (8):* the step shows the lines. r4's price-free alternative is not built.

*Removed in r4:* the price lines built into the app from the cents map, and the test that tied the two.

**D10. The billing emails, and the summer pause removed** (revised r4).
- **The pause and its resume notice are removed** from `billing-jobs`: `shouldBePaused`, `pauseDecision`,
  `resumeNotice` and the `monthly` branch (`billing-jobs/handler.ts:46-58, 92-105, 126-166`).
  - *Reasons:* ruling (1) of 11-10 says no summer pause; under ruling (6) no subscription carries the one
    key it ran on (`:130`); and it moves money (`pause_collection`, `:141-143`) with nothing left to act on.
    The amendment leaves the removal to this lane (`:1480-1482`).
  - The `paused` column and the webhook's `paused` field stay: dropping them needs a migration and buys
    nothing.
- **The annual reminder states the subscriber's own price.** It still goes a year after the last one
  (`reminderDue`, `:63-67`, unchanged). For each subscriber it is due to, `billing-jobs` reads the
  subscription from Stripe (`stripeGetFrom`, `_shared/stripe.ts:105-117`), looks its price id up in the
  catalog, and writes the catalog's words: "Knowlu, $12.99 every 4 weeks, plus tax where it applies,
  renewing automatically until you cancel."
  - An id the catalog lacks is not mailed, is logged with the account id only, and is counted in `failed`,
    as an unknown plan is today (`:119-123`).
  - *Why the subscription, not the plan column:* after a `stay` change two amounts share one key (D14). The
    cost is one Stripe GET per subscriber a year.
- **D14's move step** runs in the same loop, under the same one-failure-never-stops rule (`:117-178`).
- `priceOf` (`:69-75`) is gone; the catalog replaces it.
- "click the gear" (`:86`) becomes "go to Settings, then Membership". "Manage subscription" stays
  (`billing-jobs/handler_test.ts:328`, `app/tests/ui_settings.rs:132`). The second "gear" line (`:101-102`)
  goes with the resume notice.

*Removed in r4:* the old plans' texts (P10), and the pause kept for them.

**D11. The terms publish with this lane, as one new version.**

*What the text is made of.* It starts from the legal spec's interim terms (§4, "Interim terms with bump A",
U10). Their *The subscription*, *How to cancel* and *Refunds* sections are replaced by §4's full text. These
paragraphs are left out, because this release builds none of their code:
- the confirmation email (C8);
- the yearly renewal notice (C10's yearly half; D14 builds its price-change half);
- "a disputed charge is not a paid period" (C12);
- the "We" and shutdown paragraphs (C13);
- *A year for someone else*. The amendment has the gift year (ruling 9), and R3 says to keep it if the
  amendment has it. These terms still leave it out until the gift lane ships, because terms must not
  promise a checkout that does not exist. Amendment 2026-11-17's (10) settles this (§8 item 7).

*What it adds.*
- A new section, *Emails about your subscription*, naming the trial reminder (D6) and the annual reminder
  (D10).
- "Sold through Link", only if Managed Payments is in effect (D7).
- **The price-change paragraph** (r4), as the legal spec words it (`:625`): "If we ever change a price, we
  email you before the change takes effect, between 7 and 30 days ahead, with the new price, the date and
  how to cancel. A price change never applies to a period you have already paid for." D14's notice now
  backs it, and the catalog's test holds D14's window inside it.
- *Removed in r4:* the old-subscriber paragraph (legal spec `:627`). Ruling (6) keeps no old subscription.

*The terms date* is the commit's own date (UTC). It must be no later than the day after the staging proof,
because the production account function refuses a sign-up version later than tomorrow
(`account/handler.ts:199-201`) and the proof's sign-ins send `TOS_VERSION`. Checkout no longer has a date
rule (D3).

**These terms replace bump A's interim terms**, so bump A carries privacy only. `TOS_VERSION`
(`account.rs:21-24`), the catalog's `TERMS_VERSION` (D13) and the page move in one commit. A new test ties
the three, beside the privacy tie (`app/tests/bundle_config.rs:134-157`). Until that commit the catalog's
`TERMS_VERSION` is today's 2026-09-10, which the staging proof's checkouts record. `site/subscribed.html:13`
changes to say 14 days, a first subscription, and Settings → Membership.

*Existing accounts.* This holds whatever `account` function production runs:
- **Anyone who subscribes** accepts the new version at checkout, because the auto-renew row carries the
  catalog's version (D3).
- **For everyone else**, T0's read-only count decides, counting only **accounts that are not Quinn's**.
  Quinn has two production accounts (`HANDOFF.md:58`), and names them; the ledger records counts.
  - If every account is Quinn's, Quinn's acceptance is recorded by hand on each of them (legal spec §4).
  - If any account is not Quinn's, C1 (T5b) joins the lane and redeploys `account` before the terms
    publish.
  - *Kept in r4, with zero customers.* This is about accounts accepting terms, not about paying: a tester
    on a comp grant holds an account and accepted the old terms at sign-up ("these terms apply to you
    except the parts about payment", legal spec `:599`). So a tester account counts as not Quinn's.

**D12. The privacy page names Link (only if Managed Payments is in effect, D7).** Link receives the card, the billing
address and the email address. The page's list of companies is closed ("Nobody else gets anything",
`site/privacy.html:86`).
- The text uses the legal spec §3's B variants (*Your subscription*, the *Link* bullet, *Delete*).
- "Seven companies" (`:75`) becomes the legal spec's D3 wording.
- A build carries one privacy key (`app/tests/ui_account_privacy.rs:226-233, 257-258`). If bump A rides
  v0.4.1, these paragraphs join it. If not, this lane makes a Link-only bump.
- Quinn reads the page.

**D13. One price catalog on the server** (new in r4; claim narrowed in r5, m2).
`cloud/supabase/functions/_shared/price_catalog.ts` (new) is the only place in the billing code (the
`billing-checkout`, `stripe-webhook`, `billing-jobs` and `billing-portal` folders, and the catalog itself)
where a price amount, the name of a price variable, or the terms version that states the prices is
written. What it does not cover, and why that is safe:
- **Cadence words without a figure** ("every 4 weeks", "once a year") also appear in Settings' plan words.
  A key's cadence is fixed forever (below), so they never change with a price. *Changed 2026-11-27:* they
  no longer appear in D8's `custom_text`, which the session stopped sending (Stripe refuses it under
  Managed Payments; the paragraph under D8); until then D8 took its tax line and money-back line from the
  catalog's words.
- **Other `$` figures under `functions/`** are model costs in comments (`_shared/judge_caps.ts:37-125`), not
  prices. Two of those comments still name "$9.99 subscription" (`:45, 97`) as a yardstick; D16 lists them
  among the documents to check, and nothing reads them.
- **The app** holds no amount in the scripts that sell or show a plan (D9), and no dollar amount in any
  script or page under `app/static` (§6).
- **Documents** (VISION, CLAUDE.md's Direction line, `HANDOFF.md`, `docs/reference/app.md`, the cloud
  README, `site/index.html`) are checked by hand at each change (D16), not by CI.

It holds:
- **`TERMS_VERSION`**: the version of the terms page that states the selling prices.
- **The cadence of each key**, fixed: `four_weekly` is week × 4, `yearly` is year × 1.
- **`SELLING`**: one entry per key, a key and its cents, in display order, Yearly first (ruling 2).
- **`KEPT`**: entries no longer sold that a subscriber may still pay, each a key, its cents and its rule
  (`stay`, or `move` with its notice days; D14). Empty at v0.4.1: no subscriber pays anything else (ruling 6).
- **The words**: one formatter that turns an entry into "$12.99 every 4 weeks" or "$84.37 once a year"; the
  P9 lines the app shows (D9); "plus tax where it applies"; the money-back line. Every email, the offer and
  the terms tie use it.
- **The price variables.** Each entry's Stripe price id is read from an environment variable whose name the
  catalog derives from the entry: `STRIPE_PRICE_` + the key in capitals + `_` + the cents. So v0.4.1's are
  `STRIPE_PRICE_FOUR_WEEKLY_1299` and `STRIPE_PRICE_YEARLY_8437`. The names are computed, never written.

Every billing function reads it, through its `Deps` as a resolved catalog, so a test can hand it an
invented one:
- checkout: the offer (D9), the offer match (D3), the price check and the id it sells (D4);
- the webhook: the plan lookup (D2) and the trial reminder (D6);
- `billing-jobs`: the annual reminder (D10) and the move (D14).

The catalog's own test holds these rules:
- exactly the two selling keys; every entry's cadence is its key's; no key and amount twice;
- the yearly amount is 13 times a whole-cent amount, so "$6.49 every 4 weeks" is exact (P11's
  construction). A future yearly price keeps this, or its amendment changes the wording;
- a `move` entry's notice window lies inside the terms' 7 to 30 days (D14);
- **the terms tie** (r5: I1): every dollar figure in `site/terms.html` is one the catalog's formatter
  writes for some entry, which is an entry's cents or, for a yearly entry, its cents ÷ 13 (P11's "$6.49
  every 4 weeks, billed $84.37 once a year"); every figure the formatter writes for a selling entry is
  there; and the page's version is `TERMS_VERSION` (`bundle_config` ties that to `TOS_VERSION`, D11). So
  "$6.49" passes and "$6.50" fails.

*Reasons:*
- **Ruling (7): one place.** Today the amount is written three times: in Stripe, in the cents map
  (`billing-checkout/index.ts:20`) and in `priceOf` (`billing-jobs/handler.ts:71-73`). r3 added two more,
  the app's lines (D9) and a terms floor (D3).
- **A derived variable name.** A variable then names one Stripe price forever. A price change adds a
  variable and never re-points one, so a kept price's id cannot be overwritten. A catalog amount changed
  without a new Stripe price leaves its variable unset, which checkout answers with 500 before anything
  sells (D4).
- **Code, not a table.** The amounts sit in the same reviewed commit as the terms page that states them,
  and CI ties the two. A table could change without review and could not be tied to the page. The price ids
  stay in each project's environment (rule 1; test and live mode differ).
- **A fixed cadence** keeps plan words and Settings right across changes, and lets D14 change a
  subscription's price without moving its billing date.

**D14. Students already subscribed when a price changes: two rules, chosen per change** (new in r4). Each
later change is its own amendment (ruling 7), which picks one rule for each old price. The catalog records
it on that price's `KEPT` entry. v0.4.1 builds both rules and uses neither.
- **`stay`.** The subscriber keeps the old price until they cancel, and nothing is sent. The webhook still
  maps the old id to the key (D2), and every email that states their amount states the old one (D6, D10).
  Quinn archives the old Stripe price so nothing sells it again; a subscription on an archived price keeps
  renewing (`unverified (Stripe)`, T0r).
- **`move`, with N notice days.** Once a day, `billing-jobs` reads from Stripe each subscription that may be
  due, and moves it when its own price is that old price, it is not set to cancel at period end, and its
  renewal is inside the window.
  - **The window** (r5: I4) is exact: `N × 24h ≤ current_period_end − now < (N + 6) × 24h`, where
    `current_period_end` is the value in the Stripe subscription the job has just read, never the view's
    copy (`billing_subscribers.current_period_end`, `20260910000200_billing_jobs.sql:30`, which the webhook
    writes and which can lag). The view's copy only chooses whom to read: every subscriber whose copy lies
    within one day either side of some `move` entry's window. The renewal date the notice states comes from
    the same Stripe value. So with N from 7 to 24, every notice goes out at least 7 × 24 hours and less than
    30 × 24 hours before the renewal it names.
  - **A trial's first charge** (r5: I3) **[PENDING QUINN, §8 item 15]**. On a **raise** (the selling cents
    for the key are higher than the old entry's), `move` skips a `trialing` subscription for the charge that
    ends its trial, so the first charge is the amount the student's consent row records (D3). It is moved,
    with its own notice, before a later renewal, once it is `active`. On a **cut**, a `trialing`
    subscription is moved like any other: the student pays less than they agreed to. That happens only when
    the notice is shorter than the trial, because a trial is in the window only while its end is at least
    N × 24h away; with today's 14-day trial and N of 15 or more, no trial is ever in the window, and the
    first charge is the old price, which is what the consent row records (r5 round 2: N4). The rule follows
    from comparing the two entries' cents and needs no new catalog field.
  - **A yearly subscriber** is reached the same way, through the window before their yearly renewal, so a
    raise on the yearly plan waits for that renewal (the amendment's recommendation, `:1545-1547`).

  For each one, in this order:
  1. it emails the notice: the plan, the old and the new amount from the catalog, the renewal date (written
     in UTC−10, as D6), "plus tax where it applies", and how to cancel;
  2. then it marks the subscription as noticed for that renewal, in the subscription's Stripe metadata,
     writing **only its own key** (r5: m6), so `metadata[account_id]`, which the webhook keys on, is never
     touched (that a metadata write merges keys is `unverified (Stripe)`, T0r question 11);
  3. then it changes the subscription to the key's selling price with no proration, so the period already
     paid stays at the old price and the renewal bills the new one (`unverified (Stripe)`, T0r).

  A failed notice changes nothing and is tried again the next day. A noticed subscription whose change
  failed is tried again without a second notice. A subscriber the job has not finished by the renewal keeps
  the old price for that renewal, and is noticed again for the next one. One subscriber's failure never
  stops the run and is counted in `failed`.
- **In the loop** (r5: m6), the move runs before the annual reminder (D10), so a reminder sent the same day
  states the price the renewal will charge. `billing-jobs` resolves every price variable as optional, as the
  webhook does (D2): one unset `KEPT` variable leaves that entry matching no id and never answers 500 for the
  whole run.

*Reasons:*
- **Ruling (7) sets no policy** for existing subscribers (the amendment's *Open for Quinn, at the time of a
  change*, `:1543-1550`). The code allows either rule without a lane; the amendment chooses, above *the
  promises every later price change keeps* (`:1406-1415`).
- **The trial rule** (pending Quinn): the consent row records the cents the student agreed to (D3). A raise
  that lands inside the trial would make the first charge differ from it; a cut never charges more.
- **Tell before acting**, as the resume notice did (`billing-jobs/handler.ts:147-160`). The change happens
  only after a notice was sent, and never touches a period already paid.
- **A week-wide window** absorbs a missed daily run.
- **The mark lives on the subscription**, not in `billing_reminders` (`20260910000200_billing_jobs.sql:2-5`):
  that row is shared with the annual reminder, which could then suppress a price notice. No migration.
- **The renewal date is already in the view**, so only subscribers near the window cost a Stripe call; the
  decision and the date stated use Stripe's own value (I4).

**D15. A new terms version reaches students without an app release** (new in r4; the amendment's
*Reading (7)* leaves this to the lane, `:1400-1402`). It has three parts. Quinn confirms them (§8 item 14).
- **What checkout records.** The version the server names: the offer carries `TERMS_VERSION`, the app sends
  it back unchanged, and checkout requires it to equal the catalog's (D3). A terms bump on the server is
  what every build from v0.4.1 on records at checkout, and no build is refused for its age.
- **What sign-up records.** Unchanged: `TOS_VERSION`, compiled into the app (`account.rs:24`) and sent at
  sign-in (`account.rs:623, 1264-1267, 1297, 1599`). It still moves with the page in the same commit
  (`bundle_config`, D11), and reaches installed apps with the next ordinary release. Until then a sign-up on
  an older build records the older version; that account records the current one at checkout, before any
  charge. A server-named sign-up version belongs to the legal spec's C1 (re-acceptance), not this lane.
  Quinn confirms (§8 item 14).
- **Publishing the page.** `site/` reaches knowlu.com only through `release.yml`'s Pages step, which
  replaces the whole site (`release.yml:294-298`). That step first restores `site/releases/` (the versioned
  installer, `Knowlu-setup.exe` and `latest.json`) from the build's artifact (`:249-262, 288-293`;
  `scripts/release.ps1:293-297, 326`). In git, `site/releases/` holds only `.gitkeep`. **So a hand
  `wrangler pages deploy site` from a checkout takes the installer and the updater's manifest off
  knowlu.com.** T10 adds `.github/workflows/site.yml`:
  - started by hand (`workflow_dispatch`) and only from `main`;
  - **one tag** (r5: I5): it reads the latest GitHub Release's `tagName` once and uses that tag for both the
    restore and the privacy comparison. It refuses when any `v*` tag is newer than that tag, because a
    release is then in flight or has failed (`release.yml` creates the Release only after its own Pages
    deploy, `:294-311`). It reads the tags **from the remote** (`git ls-remote --tags origin`, or a checkout
    with `fetch-depth: 0`): the default `fetch-depth: 1`, which `release.yml`'s publish job uses (`:287`),
    fetches no tags, and the check would then pass on nothing. It compares them **as versions**
    (`--sort=v:refname`), so v0.10.0 sorts after v0.9.0 (r5 round 2: N1);
  - **never beside a release** (r5: I5): it refuses while a `release.yml` run is queued or in progress, and
    it has a `concurrency` group of its own, so two site runs never overlap;
  - its one job declares `environment: release`, as `release.yml`'s publish job does (`:277-281`), so a
    required reviewer added to that environment gates site publishing too, and the `CLOUDFLARE_*` secrets
    resolve whether or not they are scoped to it. That environment is also the subject of the signing
    credential's OIDC trust (`release.yml:144-156`), so `site.yml`'s permissions are exactly `contents: read`
    and `actions: read`, never `id-token: write`, and a test pins that. A deployment rule on the `release`
    environment that admits only tags would refuse `site.yml`, which runs from `main` (r5 round 2: N3);
  - before deploying, it puts that Release's installer, and its `latest.json` when the Release carries one
    (`release.yml:310`), into `site/releases/`, and copies the installer to `Knowlu-setup.exe` as
    `release.ps1:297` does; a missing installer stops it;
  - it refuses when `site/privacy.html` differs from that tag's, because the privacy version and its panel
    key are compiled into the app (the legal spec's C25 reason, `:850`);
  - then it runs the same `pages deploy site` command (`release.yml:298`).

  **Recovery, once `site.yml` exists** (r5: I6): if knowlu.com loses its installer or `latest.json` (a hand
  deploy, a failed run), the fix is another `site.yml` run, or, if that cannot run, a hand restore that does
  what it does: that Release's assets into `site/releases/` of a `main` checkout, and `site/privacy.html`
  taken from that Release's tag (or the restore stops if `main`'s copy differs), then the same deploy, on
  Quinn's word. Without that, a privacy bump merged and waiting for its release would go live before any
  build carries it (r5 round 2: N2). **Never re-run a tag's `publish` job**: it deploys the tag's `site/`, which after a price
  change holds the old terms, while checkout keeps recording the new `TERMS_VERSION`; and its artifact is
  kept 7 days only (`release.yml:262`).

  This narrows the legal spec's C25 ("legal pages deploy only on a release tag") to the privacy page: a
  payer's terms version no longer comes from the build. If the waitlist lane's site-only deploy (W-D3) has
  merged first, T10 instead narrows its guard and adds the restore if it lacks one. T10 does not gate the
  v0.4.1 tag, which publishes v0.4.1's own pages. It must merge before the first price change, and the
  runbook's first step checks that it has.

*Reasons:* *Reading (7)* gives this lane the release question and asks that the published terms state the
price in their own version, under the runbook (`:1397-1402`), so the terms must reach students the same way
the prices do. The restore step is there because the Release's assets, not git, are the lasting copy of the
installer the site serves. One tag and no overlap with a release keep the restored installer, the privacy
page and the builds students hold in step.

**D16. The price-change runbook** (new in r4). T11 writes `docs/reference/price-change.md`. It is the step
list for raising or lowering a price, in test mode on staging first, then live, every live step on Quinn's
word. A raise and a cut follow the same steps; only the amendment's rule differs. It changes an amount
only: **adding or dropping a plan key needs an app release and its own lane** (r5: m11), because the app's
buttons are per key, and a build whose offer lacks one of its keys can sell nothing ("Update Knowlu", D9).
0. **Before anything:**
   - `site.yml` exists (T10);
   - `site/privacy.html` on `main` equals the latest Release's (r5: m9). A privacy bump merged before its
     release makes `site.yml` refuse, so it blocks every price change until that release ships;
   - the amendment states the new amount and, for the old price, `stay` or `move` with N (D14), and Quinn
     signs it. A new yearly amount is 13 times a whole-cent amount (D13);
   - under `move`, D14's trial rule is ruled (§8 item 15) and the code matches the ruling.
1. **Stripe, test mode:** a Price for the new amount, the key's cadence, tax behaviour exclusive.
   - **If the amount is new for this key**, Quinn creates the Price, and the main session sets
     `STRIPE_PRICE_<KEY>_<CENTS>` on staging.
   - **If the key has been sold at this amount before** (r5: m4; for example 1299 → 1499 under `stay`,
     then back to 1299), reuse that Price: Quinn unarchives it, its variable keeps (or is set again to) the
     same id, and step 2 moves its entry from `KEPT` back to `SELLING`. Never create a second Price at the
     same key and cents: the derived name would name two prices.
2. **One Contract PR:**
   - the catalog: the new `SELLING` entry; the old one moved to `KEPT` with its rule; `TERMS_VERSION`;
   - `site/terms.html`: the figures and the version, and anything the amendment adds for kept subscribers;
   - `TOS_VERSION` in `account.rs`, by `contract-engineer`;
   - the one test that names the selling amounts (the catalog's first pin), so the change is seen in a test
     diff too. Handler tests build their own catalogs and do not change.

   CI ties the catalog, the page and `TOS_VERSION`, and finds any `$` figure, decimal money figure or
   `STRIPE_PRICE_` name left in the billing code outside the catalog. It does not scan bare cents such as
   `1299` (m3); a hard-coded amount shows in behaviour instead, because the handler tests run on invented
   catalogs (1399) (r5 round 2: N5). CI does not read documents, so the same PR checks by hand (r5: m2): VISION's One edition
   line, CLAUDE.md's Direction line, `HANDOFF.md`'s billing lines, `docs/reference/app.md`'s price line,
   the cloud README's Billing section, `site/index.html`'s pricing, and the `$9.99` yardstick comments in
   `_shared/judge_caps.ts:45, 97`. Each is changed, or named in the PR as left alone with a reason.
3. **Staging:** deploy the three billing functions from the PR head. With an unchanged installed build,
   the offer, both buttons and Stripe's test page show the new figure, and an offer from before the change
   is refused with D3's sentence. For `move`, rehearse one notice and one change on a test subscription
   (as S2 did).
4. **Live mode:** Quinn creates the live Price; the main session sets the production variable. **The reuse
   rule of step 1 holds in live mode too** (r6, found by the runbook's review, 2026-11-21): if the key has
   been sold at this amount before, Quinn unarchives that live Price and its variable keeps the same id.
   Never a second live Price at the same key and cents: subscribers kept on the first would stop being
   recognised (`stripe-webhook/handler.ts` writes `plan: null` for a price id the catalog does not resolve).
5. **Merge, then go live back to back:** publish the page with `site.yml`, then deploy the three billing
   functions from the merge SHA. Page first: in the minutes between, a checkout records the old version with
   the old price, a matched pair, and never a version that is not yet published.
6. **Check:** knowlu.com/terms.html's version equals the catalog's; the live offer shows the new figures;
   the live endpoint's deliveries since the deploy are all 2xx; one live checkout page opens (nothing paid);
   knowlu.com still serves the installer and `latest.json`. If either is gone, recover by D15's rule
   (`site.yml` again, or the hand restore), never by re-running a tag's `publish` job (r5: I6).
7. **Afterwards:** Quinn archives the old live Price. A `KEPT` entry is removed by a later PR, and its
   variable unset, only when Stripe's list of subscriptions on that price, **every status included**,
   shows none that could still bill (r5: m5): the job's view leaves out `past_due` and `unpaid`
   (`20260910000200_billing_jobs.sql:34`), and `move` skips one set to cancel at period end, and either can
   recover or be un-cancelled later. (That the list filters by price and takes every status is
   `unverified (Stripe)`, T0r.)
8. **No app release.** `TOS_VERSION` reaches installed apps with the next ordinary release (D15).

*Reason:* *Reading (7)*'s "one written procedure" (`:1396`): every place a change touches is named here; CI
fails if one in the billing code or the terms is missed, and step 2's list covers the documents.

## 3. What Managed Payments changes (each row confirmed by T0r, and in test mode by S1)

**Sources.**
- The legal-pages audit cites the two Stripe pages below (`docs/reports/2026-11-17-legal-pages-audit.md`,
  gap 1, sources list).
- The 2026-11-17 cost research read Stripe's pricing pages. It is kept in session notes, not in the
  repository.
- The repository's own record is cited in each row.
- **No Stripe page was opened for this note.**

| Area | What the sources say | Page or record | What this lane does |
|---|---|---|---|
| Account setting | Kept on in live mode on 2026-09-14. A live checkout with today's form, `automatic_tax` included, completed on 2026-09-24 and wrote the entitlement. | `HANDOFF.md:63, 80, 629`; amendment `:1157` | D7's readings (i) and (ii). T0r says which one holds. Quinn reads the 2026-09-24 payment's seller in the Dashboard. |
| Checkout Session | A Managed Payments session is asked for per session (`managed_payments[enabled]`), and it refuses `automatic_tax` and `customer_update[address]`. | `docs.stripe.com/payments/managed-payments/update-checkout` | D7. T0r quotes every refusal. Only the droppable keys may go, and a refused load-bearing key goes to Quinn at checkpoint B. |
| Tax | Link calculates, collects and remits it. | `docs.stripe.com/payments/managed-payments/how-it-works` | Prices are tax-exclusive (U12, D4). Kentucky registration is already on the lawyer's list. |
| Receipts | Link sends them, and the customer sees "Sold through Link". | same | The terms say so (D11). |
| Trial reminders | Link sends trial reminders. | same (audit) | Knowlu's goes anyway (D6). S1 records Link's. |
| Cancelling | The customer can cancel at link.com. | same (audit) | The in-app path stays. S1 (d). |
| Refunds | Link may refund within 60 days, at Knowlu's cost, and refunds on its own after 48 hours with no answer. Knowlu can refund from the Dashboard. | audit; cost research | The money-back guarantee and refund cases are honoured by hand in the Dashboard. S1 records whether the tax and the fee come back. |
| Fee | 3.5% on top. | cost research | Not an input here. The amendment's margin check assumes 7.1%. |
| Subscriptions and webhooks | Live mode worked on 2026-09-24 with Knowlu's endpoint. Test mode is not on record. | `HANDOFF.md:63` | S1 (a) in test mode, and the pre-tag live proof (§4, step 10b) in live mode. |

## 4. Order of rollout, and the Stripe steps only Quinn can take

**Step 0, at T0, read-only and on Quinn's word.**
- **The lists.** Read production's `supabase migration list` and `supabase functions list`.
- **The counts:**
  - the number of accounts, split into Quinn's (Quinn names them; `HANDOFF.md:58` records two) and others;
  - `entitlements` by plan and status;
  - open comp grants, if `public.comp_grants` exists;
  - Quinn reads any old-price subscription in the live Dashboard.
- **The ruling's premise** (r4). Ruling (6) rests on "zero customers". Kept in r4 as the check that the
  premise holds, at the cost of reading counts already taken: if any `active`, `trialing`, `past_due` or
  `unpaid` subscription belongs to an account that is not Quinn's, the lane stops and goes to Quinn.
- **The parity gate.** If production lacks `20261020000500_testers.sql` or `20261020000600_comp_guards.sql`,
  a named precondition is set: **"production parity for v0.4.1"**, on Quinn's go (§8 item 2). It is needed
  before the comp grants for Quinn's own accounts (below) and before step 10.
  - **Its scope.** `db push --include-all` pushes every pending local migration and has no "through"
    option. So the push runs from a **scratch copy of `cloud/supabase` whose `migrations/` ends at
    `20261020000600_comp_guards.sql`**, as L9 does (`HANDOFF.md:598`). L8's `entitlement` redeploy
    follows.
  - **The list first.** The main session shows Quinn the exact list of files the push will apply.
  - **If the gap is wider.** If anything missing below that stamp needs a secret, a Vault entry or a
    function redeploy beyond `entitlement` and `billing-checkout` (for example email's migrations, whose
    cron needs Vault entries first), the precondition is L9 proper, and v0.4.1 waits for it.
- *Why it gates step 10.* Today's checkout reads `effective_entitlements` (`billing-checkout/index.ts:35-42`),
  which the testers migration defines (`:76`). Without it every production checkout answers 5xx, and the
  comp grants below have no table to go in.

**Staging, in test mode.**
1. Item 4 is settled, then Quinn takes the test-mode steps of the checklist.
2. The main session sets `STRIPE_PRICE_FOUR_WEEKLY_1299` and `STRIPE_PRICE_YEARLY_8437` on staging.
3. After T1a-T4a, the main session deploys `billing-checkout`, `stripe-webhook` and `billing-jobs` from the
   lane branch, and builds a T4 dev app.
4. **S1**, on that build. Quinn is at the test-mode Dashboard. Test cards only; no money moves.
4b. **S2, the price-change rehearsal** (r4), on the same build, unchanged. The main session deploys the three
   functions from a scratch branch whose only change is the catalog: a test-mode price at another amount
   becomes the 4-week selling price (a raise), and 1299 moves to `KEPT`, first with `stay`, then with
   `move`. It shows, with no app rebuild: the new figure in the offer, the wizard and Stripe's page; an old
   offer refused (D3); under `stay`, a test subscription at 1299 still mapped and left alone; under `move`
   while it is still `trialing`, the trial rule (D14, pending item 15); then, **after its trial is ended so
   that a period is paid** (r5: I2), one notice and one price change with no proration on that subscription,
   with the paid invoice unchanged and the upcoming invoice at the new amount. Then the functions go back to
   the lane branch. The scratch branch is never merged.
5. **Checkpoint B.** Quinn rules on D7 if a gate failed.
6. After T5, the final staging proof on the PR head (plan §8 B, part 2).

**Production, in live mode, each step on Quinn's word.**

- **P-parity.** Done, if step 0 set it.
- **Quinn's own subscriptions** (ruling 6; this was §8 item 9, now ruled). Quinn's accounts and testers on
  comp grants are not customers. A tester holds no Stripe subscription, so nothing changes for one. For
  each of Quinn's accounts that holds an old-price subscription:
  1. A comp grant through `admin.comp_grant`. It lasts at most a year
     (`20261020000600_comp_guards.sql:8-12`) and is extended with `admin.comp_extend`.
  2. Then Quinn cancels its old-price subscription **immediately**, not at period end, in both modes.

  The grant comes first, so the account is never without access. The view matches a grant only when the
  Stripe status is not `active` or `trialing` (`20261020000500_testers.sql:76-87`). *Why it still has to
  end, with zero customers:* after step 10 its events map to no plan (D2), `billing-jobs` would count it as
  failed every day, and its renewal would charge an old price that no published terms state. It moves money
  in production, so it waits for Quinn's word, account by account.
7. **Read-only checks again.** Each must equal step 0's, except:
   - no `active` or `trialing` row on `monthly` or `academic_year`;
   - the comp grants above.

   A new account that is not Quinn's stops the rollout and makes T5b required.
8. **The PR merges**, with CI green and every review passed.
9. Quinn takes the live-mode steps of the checklist, and reads back the live endpoint's enabled events.
10. The main session sets `STRIPE_PRICE_FOUR_WEEKLY_1299` and `STRIPE_PRICE_YEARLY_8437` on production and
    deploys the three functions from the merge SHA. Then Quinn reads the live endpoint's recent deliveries:
    every one answered 2xx since the deploy.
10b. **The pre-tag live proof**, on a fresh production scratch account. It is created by emailed code at an
    address Quinn supplies, never written in the repository, and it counts as a founder test account.
    - **The old-build refusal.** `POST /billing-checkout` with `{"plan":"monthly","terms_version":"2026-09-10"}`
      answers 409 with D3's exact sentence in `error`. That body proves the new function is live, apart
      from the tester refusal.
    - **The offer** (r4). `GET /billing-checkout` answers the two plans, Yearly first, with the catalog's
      lines and the new terms version. A `four_weekly` POST whose `amount_cents` is one cent off answers 409
      with D3's "changed" sentence.
    - **The live pages.** `yearly` and `four_weekly` checkouts each open a live page. A headless read shows:
      - $84.37, then $12.99 every 4 weeks;
      - 14 days free, the tax line and D8's sentence (*changed 2026-11-27:* Stripe's own renewal lines
        instead, as listed under D8);
      - "Sold through Link" if Managed Payments is in effect, and its absence if item 10 took the
        switch-off fallback.

      Nothing is paid.
    - **The live reminder round trip, if Quinn agrees (§8 item 11).**
      1. Quinn completes the 4-week checkout with their own card under the 14-day trial. Nothing is
         charged at checkout.
      2. The webhook writes `four_weekly`, `trialing`.
      3. Quinn moves the trial end in the live Dashboard to **no closer than 24 hours away**, and within
         3 days, if T0r confirms that this fires the event.
      4. Wait **at most 30 minutes** for the reminder, sent through production's `EMAIL_*` secrets.
      5. **Quinn cancels immediately, whether or not the reminder came.** A missing reminder is recorded as
         a failed gate.
      6. Read back `canceled`, in Stripe and in `entitlements`, before anything else runs.
    - **Deletion.** Only after the `canceled` read-back: the account is deleted through `DELETE /account`,
      and its rows are gone. Deletion alone would only set the subscription to cancel at period end
      (`account/handler.ts:88-94`), so it is a backstop, not the cancel.
    - **What stays (§8 item 12).**
      - The consent rows carry the new versions before they are published. Deletion keeps them, with the
        account id nulled (`account/index.ts:114`).
      - The live Stripe customer stays, and Link may keep a record.
      - The ledger records all of these as counts.
11. **The pre-tag gate, then the `v0.4.1` tag.** CI publishes the app, and `release.yml:294-297` deploys
    `site/`, replacing the whole live site. So the gate requires that **`site/` equals the live site plus
    T5's pages**. The controller's site port (branch `site/port-live-1119`) merges before the v0.4.0 tag,
    which settles this outside the lane; the gate checks it. The terms and `latest.json` go live together.
12. If every account is Quinn's, Quinn's acceptance of the new terms is recorded by hand on each of Quinn's
    accounts.
13. **After the tag.** The main session confirms that knowlu.com/terms.html carries the new version and
    `subscribed.html` says 14 days. The updater offers 0.4.1. Then it unsets `STRIPE_PRICE_MONTHLY` and
    `STRIPE_PRICE_YEAR` on both projects (r4): nothing reads them.
14. **T10's first run** (r4; not before the tag, and on Quinn's word, because it replaces the live site).
    First, `git diff v0.4.1 main -- site/` is empty (r5: m9), so the run publishes exactly what the tag did.
    Then `site.yml` deploys `site/`, and knowlu.com serves the same pages, the installer, and `latest.json`,
    and the updater still offers 0.4.1. It must pass before any price change. If it drops a file, the
    recovery is D15's hand restore from the v0.4.1 Release's assets, never a re-run of the tag's `publish`
    job (r5: I6): that job's artifact lasts 7 days (`release.yml:262`), and step 14 may come weeks later.

*Why the functions go before the tag.* The other order lets old builds buy at $9.99 while the published
terms say $12.99, and v0.4.1 builds would get no offer (today's function answers only POST,
`billing-checkout/handler.ts:91`). Between steps 10 and 11 no build can subscribe at all.
That is acceptable, because outside sales are closed (Amendment 2026-10-16b) and the only install is
Quinn's. Step 11 follows on the same day.

*If the round trip is declined, or the event does not fire on a trial-end move.* Quinn's read of the
endpoint's events stands. Production's `EMAIL_*` is then proven only by one `mail-canary` send, which uses
the same key (`mail-canary/index.ts:6, 174`), if that function is on production. The reminder's own code
path is then proven on staging only. Quinn accepts that gap at checkpoint D.

**Quinn's checklist.** Do each step in test mode, then in live mode at step 9.
- [ ] On the existing product, create a price of **USD 12.99, recurring every 4 weeks** (week × 4), **tax
      behaviour exclusive**. Send the id; it is public. It is set as `STRIPE_PRICE_FOUR_WEEKLY_1299`.
- [ ] On the same product, create a price of **USD 84.37, recurring yearly**, tax behaviour exclusive. Send
      the id. It is set as `STRIPE_PRICE_YEARLY_8437`.
- [ ] **Archive the old prices** ($9.99 monthly, $69.99 yearly) once your own subscriptions on them have
      ended (r4, ruling 6). Nothing sells or renews on them after that. (r3 kept them for P10.)
- [ ] **Confirm that Managed Payments is on** in live mode (recorded 2026-09-14). Turn it on in test mode if
      it is off. Check the product's tax code. Read the 2026-09-24 payment's seller, which settles D7's
      reading.
- [ ] Add **`customer.subscription.trial_will_end`** to each webhook endpoint (today's list:
      `docs/plans/2026-09-09-c1-accounts-plan.md:2198`). Adding it early is safe: today's webhook answers
      200 "ignored" (`stripe-webhook/handler.ts:118-125`). In live mode, read back the enabled events.
- [ ] **Customer Portal:** cancellation on, no survey, **plan switching off**.
- [ ] Note whether Stripe's own trial-reminder email is on. Turn it off if Link sends one.
- [ ] **Deactivate the live 100% promotion code** once the comp grants on your accounts exist (P10's first
      sentence, which stands: comp grants are the comp mechanism). Kept in r4: with zero customers the code
      is still a live way to take the new prices for nothing.
- [ ] Check that the **terms URL** in the public business details is knowlu.com/terms.html.
- [ ] No new secrets on the same Stripe account. The new-account case is §8 item 4.

## 5. What must not break

- **The two overriding rules.**
  - Price ids come from each project's environment.
  - No account, person or address is named in code, tests or docs. Test data uses `example.invalid`.
  - The reminder's date rule assumes no time zone.
  - Nothing in `engine/` changes, so no frozen reference is touched.
- **The gate.** It reads status only, so `trialing` is entitled, and testers come from
  `effective_entitlements`.
- **Checkout's order.**
  - Every refusal comes before the Stripe customer, the consent row and the session: D3's checks, the 18+
    check (`handler.ts:103`), the tester check (`:107`), D5's already-subscribed check and D4's price check.
  - The consent row is still written before the redirect (`handler.ts:6-8`).
  - No load-bearing parameter (D7) leaves the form without Quinn's ruling.
- **The webhook.**
  - The order holds: signature before parse (`stripe-webhook/handler.ts:105-115`), then duplicates, then
    the stale guard, and a deleted account is ignored.
  - It stays the only writer of `entitlements`.
  - No unset price value can make it answer 500.
  - The API pin stays (`_shared/stripe.ts:82`).
- **One source for every price** (r4, D13).
  - No amount, cadence or price-variable name is written under `cloud/supabase/functions/` outside the
    catalog.
  - The app holds no price and caches none. It shows only the offer the server sent for this step, and
    sends that offer's amount and terms version back (D3, D9).
- **Prices that change** (r4, D14). A key never changes with its price, and no price variable is ever
  re-pointed. A `move` notices first, 7 to 30 days ahead, and never touches a period already paid.
- **`billing-jobs`.** One failure never stops the run, and no guessed price is mailed
  (`billing-jobs/handler.ts:14-16, 69-75`): an email states only the catalog's amount for the subscriber's
  own price. Logs name the account id only.
- **The app.**
  - Settings shows no price.
  - No trial word or trial length appears anywhere in the app.
  - "Manage subscription" keeps its exact words.
- **The wording rule.** Every price Knowlu writes says "every 4 weeks" or "once a year". None says "a month",
  "/mo" or "monthly" beside a price.
- **The legal pages.**
  - The version ties hold: privacy, and the new terms tie (`TOS_VERSION`, the catalog's `TERMS_VERSION` and
    the page are one date).
  - A build carries one privacy key, and the privacy page publishes only with a release tag (C25, narrowed
    to it, D15).
  - The existing `engine/tests/site.rs` pins pass.
- **Releases.** Releases are CI-only. Production steps and the tag wait for Quinn's word. No deploy of
  `site/` ever takes the installer or `latest.json` off knowlu.com (D15).

## 6. Tests that prove it (each written first, and failing)

**Cloud, `deno test`** (run as `ci.yml:153-172` runs it; no test reads a `STRIPE_*` or `EMAIL_*` value from
the environment).

**`_shared/price_catalog_test.ts` (new, r4):**
- The rules of D13: the two selling keys in order, Yearly first; every entry's cadence is its key's; no key
  and amount twice; the yearly amount is 13 times a whole-cent amount; a `move` entry's N is 7 to 24.
- The derived variable names: `STRIPE_PRICE_FOUR_WEEKLY_1299` and `STRIPE_PRICE_YEARLY_8437`; a resolved
  catalog with one variable unset still resolves the others, and that entry matches no id.
- The formatter: "$12.99 every 4 weeks", "$84.37 once a year", the P9 lines, and, for an invented entry at
  1399, "$13.99 every 4 weeks". Generic cadences ("every 2 months", "every year") for D6's unknown ids.
- **`the_terms_state_the_catalogs_prices`**: every dollar figure in `site/terms.html` is one the catalog's
  formatter writes (an entry's cents, or a yearly entry's cents ÷ 13), every figure it writes for a selling
  entry is there, the page's version is `TERMS_VERSION`, and its price-change paragraph says "between 7 and
  30 days ahead". Written with the terms (plan T5), because it is red until they change. Its own red cases
  run on text the test supplies (r5: I1): "$6.49" passes, "$6.50" fails, and a page missing "$84.37" fails.
- Only the first rule names the selling amounts. Handler tests build their own catalogs, so a price change
  edits one test (D16 step 2).

**`_shared/price_single_source_test.ts` (new, r4; its own file so its task's files stay disjoint):**
- **`no_amount_is_written_outside_the_catalog`**: reads every non-test `.ts` file under `billing-checkout/`,
  `stripe-webhook/`, `billing-jobs/` and `billing-portal/`, and fails on a dollar figure, a decimal money
  figure, a `STRIPE_PRICE_` name, "per month" or "a month". (r5: m3) It does not match bare cents: a cents
  rule would fail on `fail(500, …)` the day a price is $5.00.

**`billing-checkout/handler_test.ts`:**
- The two plans reach Stripe with their price id and the catalog's cents.
- **The offer** (r4): `GET` with a session answers the selling plans in the catalog's order, each with its
  key, cents and lines, plus `terms_version` and the tax line, and reads no account; without a session it
  is 401; a method other than GET or POST is 405. With an invented catalog at 1399, the offer says
  "$13.99", which proves the answer is the catalog's.
- D3:
  - the old names answer 409 with the exact sentence, with only the session check before it (no account,
    tester or Stripe call);
  - an unknown plan answers 400;
  - a missing `amount_cents` or `terms_version` answers 400;
  - an `amount_cents` one cent off, or a `terms_version` other than the catalog's, answers 409 with the
    "changed" sentence, before any account read;
  - the consent row records the catalog's cents and `TERMS_VERSION`.
- D5:
  - "14" on a first subscription;
  - absent after a canceled one;
  - 409 when the row is `active`, `trialing`, `past_due` or `unpaid`.
- D4: one case per check: amount, currency, `active`, `tax_behavior`, interval and count. That includes
  **`four_weekly` pointing at a `year` × 1 price at 1299**, and the reverse for `yearly`. An unset selling
  variable is the seventh case. Each answers 500 before the consent row.
- D7: every load-bearing key is present (the trial key on a first subscription only, D5), and the dropped
  keys are exactly those checkpoint A allowed.
- D8, *changed 2026-11-27*: for each plan and trial combination, no key of the form starts with
  `custom_text`, because Stripe refuses it under Managed Payments (the paragraph under D8). Until then:
  `custom_text` for each combination, carrying an invented catalog's tax and money-back words (r5: m2).
- The 18+ and tester tests are unchanged (their request bodies gain the offer's two fields).
- A text-read of `index.ts` (`handler_test.ts:331-343`) checks that it hands the handler the catalog
  resolved from the environment, and that `isTester` is unchanged.

**`stripe-webhook/handler_test.ts`:**
- The plan comes from the price id, through an injected catalog: both selling ids, a `KEPT` id (the
  `stay` case: same key, other cents), an unknown id gives `null`, and a missing variable matches nothing
  and does not answer 500.
- `trial_will_end`:
  - one email, naming the plan words and the amount from the catalog entry the id matches, "plus tax where
    it applies", the UTC−10 date and "Manage subscription", and never "in 3 days"; with an invented entry at
    1399 the email says "$13.99";
  - an unknown price id still gets the email, without plan words, its amount from the event's price;
  - nothing is sent if the subscription will cancel at period end, is not `trialing`, or has a `trial_end`
    already past;
  - a failed email gives 502, nothing is recorded, and the log names the account id only;
  - a duplicate event sends nothing;
  - no entitlement is written.

**`billing-jobs/handler_test.ts`:**
- The pause and resume tests go with the code (D10). No subscriber is ever sent a `pause_collection` call.
- **The annual reminder comes from the catalog** (r4): the subscription is read, its price id is looked up,
  and with an invented catalog at 1399 the mail says "$13.99 every 4 weeks, plus tax where it applies" and
  never "per month". A `KEPT` `stay` id gets its own (old) amount. An id the catalog lacks is not mailed and
  counts in `failed`. A failed subscription read counts in `failed`, and the run goes on.
- "Manage subscription" stays (`:328`), and "gear" is gone.
- **The move (D14)**, with an injected catalog whose `KEPT` entry is `move` with N:
  - a subscriber on that price renewing inside the window gets one notice naming the old and the new
    amount and the date, then one metadata write carrying only the mark key, then one price change to the
    selling price with no proration, in that order;
  - **the window's edges** (r5: I4): exactly N × 24h before the renewal is inside; one second less than
    N × 24h, and exactly (N + 6) × 24h, are outside;
  - **the date source** (r5: I4): when the view's copy and the Stripe subscription disagree, the window and
    the stated date follow Stripe's;
  - outside the window, or on another price: nothing; set to cancel at period end: no mail and no write
    (r5: m10; the flag comes from the Stripe read);
  - **a `trialing` subscription** (r5: I3, pending item 15): under a raise, no mail and no write before the
    charge that ends the trial; under a cut, moved like any other;
  - **a `yearly` subscriber** (r5: I3): a renewal a year out, reached through the window, is noticed and
    moved;
  - a failed notice: no metadata write and no price change; the next run tries again;
  - noticed but not changed: the next run changes it and sends no second notice;
  - a `stay` entry: never noticed, never changed;
  - one subscriber's failure does not stop the next;
  - (r5: m6) a subscriber both due a move and due the annual reminder on the same day: the move runs
    first, and the reminder states the new amount.
- (r5: m6) A text-read of `index.ts`: every price variable is resolved as optional, never through the file's
  throwing `env` (`billing-jobs/index.ts:6-10`).

**App, `cargo test`.**
- **`ui_wizard`**
  - `the_subscribe_buttons_send_the_signed_plans`: order, classes and values; `open_checkout` gets the
    plan, the offer's `amount_cents` and its `terms_version`; billing's list of plans.
  - `the_wizard_holds_no_price_and_renders_the_offer` (r4; replacing `the_wizard_states_no_price_or_trial`):
    - it reads every string literal in `wizard.js` **and `picker.js`**, as today's test does, with the
      status word `"trialing"` as the one exception;
    - no literal contains "$", a decimal money figure, "a year", "every 4 weeks" beside a figure, "9.99",
      "12.99", "84.37", "6.49", "a month", "per month", "/mo", "trial", "14 days", "14-day", "free first" or
      "Pell";
    - the step asks `checkout_offer` and writes its lines with `textContent`, never `innerHTML`;
    - the panel's markup carries no price line.
  - `without_an_offer_the_plan_buttons_wait`: until an offer arrives the buttons are disabled; on a
    failure the step shows a Try again button and the sentence for that failure: the connection sentence
    only for an `UNREACHABLE` error, the command's own `error` for a refusal, and the update sentence for an
    offer missing a plan (r5: m1).
- **`ui_settings`**
  - The table carries the two plan words, and no `monthly` or `academic_year`.
  - `membership_shows_no_price` gains "84.37", "6.49" and "14 days".
- **`ui_plans` (new)**
  - The sheet sends both plans, Yearly first, with the offer's two fields, and only its two buttons start a
    checkout.
  - `carried/account.js` holds no price literal, by the wizard test's rule.
  - Every plan the app sends is one the catalog sells; every key the catalog can name has a word in
    Settings.
  - (r5: m2) No `.js` or `.html` file under `app/static` contains a dollar amount with cents
    (`\$\d+\.\d\d`). None does today; the SVG logos are not read.
- **`account` (`app/tests/account.rs`, loopback, r4)**
  - `checkout_offer_at` sends `GET /billing-checkout` with the bearer and returns the body unchanged; a
    non-2xx gives the server's sentence.
  - `checkout_url_at` posts `plan`, `amount_cents` and `terms_version` as given, and not `TOS_VERSION`.
- **`bundle_config`:** `the_terms_version_constant_is_the_published_terms_date`, which also requires the
  catalog's `TERMS_VERSION` to equal `TOS_VERSION`.
- **`engine/tests/workflows.rs`** (r4, T10): `site.yml` is dispatch-only, runs only on `main`, pins every
  action by SHA, restores the installer (and `latest.json` when present) before deploying, refuses a
  changed `site/privacy.html`, and never runs self-hosted (today's self-hosted pin names only `release.yml`,
  `workflows.rs:216-217`). (r5: I5) It reads one tag, the latest Release's `tagName`, for both the restore
  and the privacy comparison; refuses when a newer `v*` tag exists, read from the remote and compared as
  versions (N1), or a `release.yml` run is queued or in progress; has a `concurrency` group; its job
  declares `environment: release`, pinned beside `workflows.rs:190`; and its permissions are exactly
  `contents: read` and `actions: read` (N3).
- **DOM checks** (`scripts/ui-fixture`): with a `checkout_offer` envelope carrying an invented price
  ("$1.23 every 4 weeks."), the wizard and the sheet show exactly that line, which proves the app draws
  what it is sent; with an unreachable envelope, the buttons are disabled and the connection sentence
  shows; with a refusal envelope, the refusal's own sentence shows (r5: m1).
- **`engine/tests/site.rs`**
  - The terms contain the signed figures and clauses.
  - The terms contain no "a month", "per month", "/mo", "$9.99", "$69.99", "Pell", "Alabama" or gift
    paragraph.
  - `subscribed.html` says "14 days".
  - If Managed Payments is in effect, the terms say "Sold through Link" and the privacy page names Link.
    Otherwise neither names Link.

**S1, S2, the staging proof and the pre-tag live proof** (the main session; plan §8). These show what no
unit test can:
- the real pages;
- both plans written by the webhook;
- the reminder delivered, in test mode and, if Quinn agrees, in live mode;
- the trial on a first subscription only;
- cancelling and deletion;
- the wizard's prices against Stripe's;
- the old-build refusal by its body;
- (S2) a catalog change reaching an unchanged build, and one real notice and price change in test mode on
  a subscription with a paid period (r5: I2).

## 7. Not in this lane

These stay in the v0.5.0 billing lane or later:
- **C1**, terms re-acceptance, unless T0's count brings it in as T5b. That includes a sign-up version named
  by the server rather than compiled in (D15).
- **C2**, emailing every account on a material policy change.
- **C7's record**: the consent record keeping the plan and the text shown. `price_cents` already tells the
  two plans apart.
- **C8**, the confirmation email.
- **C10's yearly half**, the yearly renewal notice. It must land before the first yearly renewal, which
  comes no sooner than a year and 14 days after the first yearly checkout. (Its price-change half is D14.)
- **C12**, refunds and disputes in the webhook.
- **C13**, closing an account, and the shutdown.
- **C15** and **C16**.
- **The gift year** (C24), **Pell**, and **plan switching**.
- **The trial in the app's text.**
- **Any price on the homepage.** A2's homepage clause stands: "to be announced".

*Moot in r4:* C11, the earlier pause-resume notice. There is no pause left to resume (D10).

Outside paid sign-ups still wait on Amendment 2026-10-16b's gate. v0.4.1 makes billing correct. It does not
open sales.

## 8. Only Quinn can answer (in the order they block)

Item numbers are r3's, so the plan's Q-numbers still hold. r4 marked three items answered and added item
14. r5 marks items 5, 6 and 7 settled by the signed amendment and adds item 15.

**Still open:** 10, 11, 12 and 13. Items 2, 4, 8, 14 and 15 were answered on 2026-11-21, and item 16 was added
and answered the same day.

1. **Answered 2026-11-20: yes.** Amendment 2026-11-10 goes to `main`, with Amendment 2026-11-17, signed by
   Quinn on 2026-11-20 (PR #83), recording the 2026-11-17 rulings and rulings (6) to (10). T1a waits until
   both are on `main`.
2. **Answered 2026-11-21: there is nothing to push.** A read-only read of production, on Quinn's word, found
   every migration in the repository applied (42 of 42), `20261020000500_testers` and
   `20261020000600_comp_guards` among them. The question was: **the production parity push, if T0 finds the
   testers or comp-guard migration missing** (§4 step 0).
   Blocks the comp grants on Quinn's accounts and step 10. *Recommend:* yes, if those two stamps are the
   whole gap, pushed from a scratch copy ending at `20261020000600`. Otherwise it is L9 proper, and v0.4.1
   waits for it.
3. **Answered 2026-11-20 by ruling (6).** No `academic_year` subscription is kept, under any name (D1).
4. **Answered 2026-11-21: before v0.5.0 is released.** The first step is one question to Stripe Support:
   whether the account's business details can change in place while Managed Payments is on. If they can, it
   is a details change before step 9; if not, a new account is set up after v0.4.1 and before v0.5.0. The
   question was: **moving Stripe to KNOWLU LLC.** It blocked the test-mode checklist, and so S1. *Recommend:* defer a new
   account until after v0.4.1, but before 10-16b's gate opens sales; after that it means recreating every
   paying subscription. A new account costs new secrets, a new endpoint, product, prices, portal and
   Managed Payments setting, every stored customer id cleared, and S1 again. If the move is only the same
   account's business details, do it before step 9, so the charge's merchant name matches the pages.
5. **Settled 2026-11-20 by the amendment's (8)** (`:1446-1451`): the subscribe step shows P9's price lines
   (D9). They come from the server, so the app holds no figure.
6. **Settled 2026-11-20 by the amendment's (9)** (`:1453-1457`): the upgrade sheet sells both plans, Yearly
   first, replacing the 2026-09-14 one-plan ruling (D9).
7. **Settled 2026-11-20 by the amendment's (10)** (`:1459-1462`): the gift year stays signed and these terms
   leave it out until the gift lane ships (D11).
8. **Answered 2026-11-21: email only for now.** It goes on the lawyer's list. The question was: **a mailing
   address for the terms' Notices clause.** It blocked T5. *Recommend:* give it before T5, or rule
   email-only (it then goes on the lawyer's list).
9. **Answered 2026-11-20 by ruling (6).** Each of your accounts that holds an old-price subscription gets a
   comp grant, then that subscription is cancelled immediately, in both modes (§4). It is no longer a
   question, but each act is a production write and still waits for your word at the time.
10. **Managed Payments in effect, so Link on the terms and the privacy page** (D7, D12; at checkpoint B).
    Blocks T5's scope. *Recommend:*
    - under reading (i): the per-session change and Link if S1 passes; otherwise ship without either;
    - under reading (ii): ship the Link texts. Turning the live setting off would reverse R2 and U5.
11. **The live reminder round trip** (§4 step 10b): your card under the 14-day trial on a scratch account;
    the trial end moved no closer than 24 hours; a cancel at 30 minutes whether or not the reminder came;
    `canceled` read back before the delete. Blocks step 10b, and so the tag. *Recommend:* yes. Otherwise
    production's reminder ships unproven.
12. **Accept 10b's leftover production records:** consent rows carrying the not-yet-published versions, the
    live Stripe customer, and any record Link keeps. They go in the ledger as counts. Blocks step 10b.
    *Recommend:* accept them.
13. **VISION's "One edition" line and CLAUDE.md's Direction line, in your words.** Blocks the tag.
    *Recommend:* yes.
14. **Answered 2026-11-21: yes to all three.** The question was: **the price-change design (new in r4).** The amendment's *Reading (7)* leaves to this lane whether a
    change needs an app or a terms release (`:1400-1402`); this is the lane's answer, for you to confirm
    before T5. Three parts, one answer:
    - checkout records the terms version the server names, so a terms bump refuses no build (D3, D15);
    - sign-up keeps recording the version compiled into the app. After a terms bump published without a
      release, a sign-up on an older build records the older version until that student updates; anyone
      who subscribes records the current one first. The alternative is a server-named sign-up version now,
      which reaches into C1's re-acceptance design;
    - the terms page publishes with a new site-only workflow (T10). That narrows the legal spec's C25 rule
      ("legal pages deploy only on a release tag") to the privacy page. r5 tightens it (D15): it reads one
      tag (the latest Release's) for both the installer it restores and the privacy page it compares,
      never runs beside a release, and is gated by the same `release` environment; once it exists, a lost
      installer is restored by it or by hand from the Release's assets, never by re-running a tag.

    Blocks T5 and T10. *Recommend:* yes to all three. Price changes are rare, and the lag in the second
    part never touches a charge.
15. **Answered 2026-11-21: the first charge stays as agreed**, which is the recommendation below, so D14 is
    no longer pending. The question was: **what `move` does to a trial's first charge (new in r5; review I3).** A student who starts a trial at
    the old price a few days before a change would otherwise have their first charge moved, while their
    consent row records the old cents. Blocks the first price change that uses `move`; T3 builds the
    recommendation and may merge with this open, because `KEPT` is empty at v0.4.1. *Recommend:* on
    a raise, `move` skips a `trialing` subscription for the charge that ends the trial, so that charge is
    the amount they agreed to, and the price changes at a later renewal with its own notice; on a cut, it
    moves a trial like any other. A cut reaches a trial only when the notice is shorter than the trial: with
    the 14-day trial and N of 15 or more, no trial is in the window, so the first charge stays the old price,
    as the consent row records (N4). D14 is written this way, marked pending. The other answer is to treat a
    trial like any period and move it either way.
16. **Answered 2026-11-21 (new; raised while the roadmap was planned): the privacy page says only what is
    kept.** Checkout records the price and the terms version accepted; nothing stores the plan and the
    wording shown. The privacy text's sentence about keeping "the renewal terms you were shown" is dropped,
    and T5 describes what is recorded. The fuller record is built only if the lawyer asks for it.

*Settled outside this lane, and not on this list:* the tag replaces the whole live site with `site/`
(`release.yml:294-297`). The controller ports the live site into `site/` (branch `site/port-live-1119`)
and merges it before the v0.4.0 tag. The pre-tag gate checks that `site/` equals the live site plus T5's
pages.

## 9. Answers to the reviews

Item numbers below are r3's §8 list. Some rows describe r3 text that r4 later replaced; §10 says which.

**Review r2 (PASS with changes):**

| Finding | Change |
|---|---|
| 1 | "Managed Payments is in effect" (reading (ii), or (i) with the change) now governs D11's Link line, D12 and 10b's check (D7). The fallback differs by reading. Item 10. |
| 2 | D4 ties the interval and the count to the plan, and lists six numbered checks. §6 adds the `four_weekly`-on-`year` × 1 case. |
| 3 | §4 step 0: the push runs from a scratch copy ending at `20261020000600`; a wider gap is L9 proper. Item 2. |
| 4 | §4 staging and plan §8 B: S1 follows T1-T4 and runs on a T4 build. A part 2 runs after T5. |
| 5 | D11, §4 and plan: only accounts that are not Quinn's count. Quinn's two (`HANDOFF.md:58`) are handled each. |
| 6 | D3: the `YYYY-MM-DD` pattern comes first, and "same rule" is gone. §6 adds "2026-11-19x". |
| 7 | §6 and plan T1: the text-read checks only the constant's shape; `bundle_config` ties its value. |
| 8 | §4 step 11 and the plan's pre-tag gate: "`site/` equals the live site plus T5's pages". The port lands outside this lane. |
| 9 | §4 10b: no closer than 24 hours, a cancel at 30 minutes whatever happens, and `canceled` read back before the delete. Item 11. |
| m1 | D5 refuses `past_due` and `unpaid` too. |
| m2 | D3 states the floor's consequence. |
| m3 | D11 and plan T5: the terms date is the commit date. |
| m4 | §6 and plan T1: "the trial key on a first subscription only". |
| m5 | §6 and plan T4: literals over `wizard.js` and `picker.js`, with the `"trialing"` exception. |
| m6 | D9 and plan T4: only the two buttons start a checkout, pinned in `ui_plans`. |
| m7 | The note no longer uses r1's "Q2". |
| m8 | `account/handler.ts:88-94` (cancel at period end) is cited, and S1 (e) is reworded. |
| m9 | Item 1 records item 6. |
| m10 | Item 12. |

**Review r1:**

| Finding | Change |
|---|---|
| C1 | §4 step 0: T0 reads production's lists, and missing testers or comp-guard migrations become precondition "production parity for v0.4.1" (item 2). The existing-accounts reasoning in D11 no longer depends on production's `account` function. |
| I1 | A2's wizard clause ends with this lane. D9 shows P9's lines. The tests flip from banning the figures to requiring them and tying them to the cents (§6). Item 5 covers the alternative. |
| I2 | D7 and §3 start from the record (`HANDOFF.md:63, 80, 629`) and give readings (i) and (ii). The checklist says "confirm". S1 covers test mode only. |
| I3 | D7 names the load-bearing parameters (now including `metadata[account_id]` and `client_reference_id`) and the only ones that may be dropped. A refusal goes to Quinn at checkpoint B. |
| I4 | D3's refusal comes first, and step 10b asserts its exact body. The live pages are read on a fresh production scratch account. Item 9 grants first, then cancels immediately (§4). |
| I5 | Step 9 reads back the live endpoint's events. Step 10 checks deliveries. Step 10b makes the live reminder round trip a pre-tag gate (item 11), and §4 gives a stated fallback. |
| I6 | Item 4 gives both branches, the cost of the new-account branch, and a recommendation to defer. T0r asks which branch applies. |
| I7 | Items 3, 6 and 7. |
| M1 | D4 checks `tax_behavior`. |
| M2 | D6: no "in 3 days", nothing sent after `trial_end`, the amount taken from the event's price, an email for unknown ids, and logs with the account id only. |
| M3 | D3's shape check. |
| M4 | D5's already-subscribed 409. |
| M5-M10 | Plan §3, §8 and §2: the S1 additions, the optional webhook values (D2), P1's split, the promotion code (checklist), the public S1 report, and the docs merge. |

## 10. Revision 2026-11-20 (Quinn's ruling)

r4 applies Quinn's ruling of 2026-11-20 (quoted under *Authority*), the coordinator's two additions (the
terms version and the site deploy) and Amendment 2026-11-17's rulings (6) and (7) (signed 2026-11-20). A
delta review reads only the rows below. Decisions not listed (D5, D7, D8, D12, §3) are unchanged from r3.
*Revision r5*, at the end, lists what the r4 review changed.

**Removed because no customer exists (ruling 6):**

| Where | What went | Why it could go |
|---|---|---|
| D1, §8 item 3 | `academic_year` keeps its name and $69.99, and counts as yearly for the money-back (U10) | No such subscription is kept |
| D2 | The two old price values in the webhook | They named only old-price subscriptions |
| D10 | The old plans' email texts; the summer pause and its resume notice | No subscription carries `monthly`; ruling (1) of 11-10 says no pause |
| D11 | The old-subscriber paragraph in the terms | Nothing to describe |
| D9, Settings | The words for `monthly` and `academic_year` | No row will carry them |
| §4 checklist | "Keep the old prices while any subscription uses them" | Replaced by "archive them" |
| §5 | P10's "old subscriptions keep their price, name and pause" | Moot |
| §7 | C11, the pause-resume notice | No pause |

**Kept, with zero customers, and why:**

| What | Why it stays |
|---|---|
| D3's 409 for `monthly` and `academic_year` | It sells nothing either way (an unknown name is 400), but it gives an old build an actionable sentence, its body proves the new function is live at step 10b, and the list never grows |
| §4 step 0's counts | They now check the ruling's premise: a paying subscription that is not Quinn's stops the lane |
| Ending Quinn's own old-price subscriptions (was item 9), on Quinn's word | After step 10 they map to no plan, fail `billing-jobs` daily and would renew at a price no terms state |
| D11's count of accounts that are not Quinn's, and T5b | It is about accepting terms, not paying; a tester holds an account |
| Deactivating the 100% promotion code | It would still take the new prices for nothing |

**Changed decisions:** D1 (keys stable across price changes), D2 (lookup through the catalog; derived
names), D3 (the offer match replaces the floor and the shape check), D4 (checks against the catalog; an
unset variable is a case), D6 (amount from the catalog), D9 (the app renders the server's offer; no price
in the app; offline behaviour; `checkout_offer`), D10 (pause removed; reminder from the subscriber's own
price), D11 (the price-change paragraph added; the terms tie is `TOS_VERSION` = `TERMS_VERSION` = page).

**New decisions:** D13 (the price catalog), D14 (`stay` and `move` for existing subscribers), D15 (terms
without an app release: server-named checkout version, `site.yml`), D16 (the runbook).

**Other sections:** Authority (the ruling and the amendment), §1, §4 (step 0's premise check, S2, Quinn's
own subscriptions, steps 10, 10b, 13, 14, the checklist), §5, §6 (new catalog tests, the offer, the move,
the app's no-literal and render tests, `account.rs` and `workflows.rs` tests), §7, §8 (items 1, 3 and 9
answered; 5 to 7 then awaiting the amendment's signature, settled in r5; item 14 new).

**Plan tasks changed or added** (`docs/plans/2026-11-19-v041-pricing-plan.md`): T0 (premise check), T0r
(three questions), T1a (new: the catalog), T1, T2, T3, T4, T4a (new: `account.rs`'s command and body), T5,
T6, S2 (new: the rehearsal), T10 (new: `site.yml`), T11 (new: the runbook), the ledger, §8's steps, the
risks and the budget.

### Revision r5 (the r4 delta review, and the amendment signed)

r5 applies `docs/reports/2026-11-20-v041-pricing-r4-review.md` (PASS with changes) and Amendment
2026-11-17's signing by Quinn on 2026-11-20 (PR #83). Round 2 reads only these rows.

| Finding | What changed |
|---|---|
| I1, the terms tie | D13's rule and §6: a figure counts when the catalog's formatter writes it (an entry's cents, or a yearly entry's cents ÷ 13); red cases "$6.49" passes, "$6.50" fails. Plan T5. |
| I2, S2 on a trial | §4 4b and plan §8 S2: the trialing run shows the trial rule; then the trial is ended so a period is paid, N = 22, and Quinn reads the paid invoice, the absence of a proration line and the upcoming invoice. |
| I3, a trial's first charge | D14 written as recommended (a raise skips a trial's first charge; a cut moves it), marked **[PENDING QUINN, §8 item 15]**; §6 and plan T3 gain a trialing case per direction and a yearly case. New §8 item 15. |
| I4, the window's edges | D14: `N × 24h ≤ current_period_end − now < (N + 6) × 24h`, on Stripe's value, which also gives the stated date; the view's copy only chooses whom to read. §6 and T3 pin both edges and the date source. |
| I5, `site.yml`'s tags | D15: one tag (the latest Release's `tagName`) for the restore and the privacy check; refuses a newer `v*` tag or a running release; `concurrency`; `environment: release`. §6 and plan T10 pins. §8 item 14 notes it. |
| I6, recovery | D15, D16 step 6, §4 step 14, plan §9 and T6: once `site.yml` exists, recovery is `site.yml` or a hand restore from the Release's assets, never a tag's `publish` re-run. |
| m1, refusal vs connection | D9: the connection sentence only for `UNREACHABLE` (`account.rs:175-178`); a refusal shows its own sentence; a missing plan says "Update Knowlu". §6, plan T4 and the DOM envelopes. |
| m2, "one place" | D13's claim narrowed to the billing code, with what it does not cover; D8 takes its tax and money-back words from the catalog; a `\$\d+\.\d\d` scan of `app/static` `.js` and `.html`; D16 step 2 lists the documents to check by hand. |
| m3, cents as tokens | The source scan drops the bare-cents rule (§6, plan T3). |
| m4, name collision | D16 step 1: an amount the key has had before reuses (unarchives) that Price; never a second Price at the same key and cents. |
| m5, removing `KEPT` | D16 step 7: removed only when Stripe's list of subscriptions on that price, every status, shows none that could bill. |
| m6, three T3 gaps | D14: the move runs before the annual reminder; the metadata write carries only the mark key; every price variable optional. §6 and plan T3 pin each. |
| m7, Link's notice | Plan T0r question 9 asks whether Link sends or requires its own notice of a price change. |
| m8, the amendment | Signed: *Authority*, §1, D9, D10, D14, D15, §8 and this section cite the signed text (`:1368-1379`, Reading (7) `:1395-1404`, `:1480-1482`, `:1543-1550`); items 5-7 settled by its (8)-(10); "open item 2" is now Reading (7). |
| m9, `site.yml` preconditions | §4 step 14 requires `git diff v0.4.1 main -- site/` to be empty first; D16 step 0: a privacy bump merged before its release blocks a price change. |
| m10, a test's wording | §6 and plan T3: cancelling at period end means "no mail and no write". |
| m11, plan changes | D16: adding or dropping a plan key needs an app release and its own lane. |

**r5 round 2** (`docs/reports/2026-11-20-v041-pricing-r5-review.md`, PASS with changes; the last round):

| Finding | What changed |
|---|---|
| N1, tags `site.yml` cannot see | D15 and §6: the tags are read from the remote (`git ls-remote --tags origin`, or `fetch-depth: 0`) and compared as versions (`--sort=v:refname`); plan T10 pins both. |
| N2, the hand restore's privacy page | D15's recovery: the hand restore takes `site/privacy.html` from the Release's tag, or stops when `main`'s copy differs; plan T6, §9 and T11 say the same. |
| N3, the release environment's OIDC subject | D15: `site.yml`'s permissions are exactly `contents: read` and `actions: read`, never `id-token: write`; a tag-only deployment rule on `release` would refuse it. Plan T10 pins the permissions. |
| N4, a cut and a trial | D14 and §8 item 15: a cut moves a trial only when the notice is shorter than the trial; with N of 15 or more, no trial is in the window. |
| N5, what CI finds | D16 step 2: CI finds `$` figures, decimal figures and `STRIPE_PRICE_` names, not bare cents; the handler tests' invented catalogs catch a hard-coded amount by behaviour. |
