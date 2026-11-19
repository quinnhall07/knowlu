# v0.4.1 pricing: design note

Planner, 2026-11-19. **Size: Contract**: it moves money, it moves `TOS_VERSION` in `app/src/account.rs`
(contract list), it publishes new terms, and, if Managed Payments ships, it changes the privacy page.
**Status: r3, final.** r2 answered review r1 (`docs/reports/2026-11-19-v041-pricing-spec-review-r1.md`).
r3 applies review r2's fixes (`docs/reports/2026-11-19-v041-pricing-spec-review-r2.md`, PASS with
changes). §9 maps each finding to its fix. There is no further review round; what stays open is §8's
list, for Quinn. Plan: `docs/plans/2026-11-19-v041-pricing-plan.md`.

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

**When rule A2 ends.** The v0.4.0 note's A2 (`docs/specs/2026-11-17-v0.4.0-new-ui-design.md:24-27`) kept
prices out of the wizard "until" billing moved to the signed prices in a later lane. This is that lane, so
A2's wizard clause ends here and P9 governs the subscribe step. D9 shows P9's lines. Quinn confirms (§8,
item 5). A2's homepage clause ("to be announced") is a separate ruling, and it stands.

The gift year (ruling 9, P13) has its own spec and lane (legal spec C24) and is not built here. The v0.4.0
note (§13) and the legal spec (§5) place the billing move in v0.5.0. Quinn's 2026-11-19 ruling is later,
so on *when* billing moves, this note wins.

**Limits of what I checked.**
- I had no shell and no web tools.
- **The amendment is not on `main`.** Commit `5dac15c7` is only on `docs/waitlist-spec` (review r1). I read
  the amendment from that worktree (`docs/specs/2026-09-09-knowlu-cloud-design.md:993-1232` there).
  Precondition P1 (§8, item 1) puts it on `main` before T1.
- **I opened nothing on Stripe's site.** Every Stripe statement here is marked `unverified (Stripe)` and
  names its page. Plan task T0r confirms each one, and the test-mode spike S1 tests it.
- **Production's state is not known from the repository.**
  - `HANDOFF.md:598` says production holds C1's migrations of 2026-09-14 "and nothing later".
  - `20261020000600_comp_guards.sql:2-3` says the testers migration "is applied on staging and prod".
  - `HANDOFF.md:70` records sixteen functions deployed to production on 2026-11-10.

  T0 reads production's migration and function lists before anything depends on them (§4, step 0).
- Code facts are cited `path:line` against this worktree, and I opened each one.

## 1. What changes for the student

- **The wizard's Subscribe step** shows the two plans, Yearly first. Each plan is a button with its price:
  - **Yearly:** "$6.49 every 4 weeks, billed $84.37 once a year. Renews yearly.", then "30-day money-back
    guarantee on each yearly charge."
  - **Every 4 weeks:** "$12.99 every 4 weeks."
  - **Below both:** "Plus tax where it applies."

  This is recommended pending Quinn's confirmation (D9). The step does not mention the trial, because only
  the server knows whether this account still has one (D5); Stripe's page states it.
- **The upgrade sheet** offers the same two plans, with the same lines (D9, pending Quinn).
- **Stripe's checkout page** shows:
  - the plan's price;
  - 14 days free, on the account's first subscription;
  - tax added on top;
  - one renewal sentence, which on the yearly plan names the money-back guarantee (D8).

  Under Managed Payments it also says "Sold through Link".
- **Before the first charge**, Knowlu emails the student the plan, the amount, the date and how to cancel.
- **Settings → Membership** reads, for example, "Active, Yearly" or "Active, Every 4 weeks", with no price.
- **knowlu.com** carries a new terms version that states these prices. The page after checkout says 14 days.
- **A build older than v0.4.1** that tries to subscribe is told to update Knowlu.
- Nothing changes for a tester on a comp grant, and nothing changes in the engine.

## 2. Decisions, each with its reason

**D1. Rename the plan keys.** New subscriptions get the keys `four_weekly` and `yearly`. The keys `monthly`
and `academic_year` remain only as names for subscriptions started before the switch, and nothing sells
them again.
- **The pause job would catch a reused key.** `billing-jobs` pauses every `monthly` subscription in June to
  August (`billing-jobs/handler.ts:126-137`). Ruling (1) says the 4-week plan has no pause.
- **P10 and the amendment ask for it.** P10 keeps an old subscription "under its old plan name", and the
  amendment's consequences say the 4-week plan "does not take the name `monthly`".
- **No migration is needed.** `entitlements.plan` is free text with no check
  (`20260910000100_accounts.sql:27`).
- **Testers and the gate are unaffected.** `tester` comes from the view (`20261020000500_testers.sql:76-87`),
  and the gate reads status only (`_shared/entitlement.ts:22, 79-81`).
- **Settings already knows `yearly`** (`app/static/js/pages/settings.js:32`).

*A reading of signed text (Quinn confirms, §8 item 3).* The amendment's consequences say "existing
`academic_year` rows map to the yearly plan". This note reads that line with P10 ("keeps its price and its
terms … under its old plan name") and U10. An `academic_year` row keeps its name and its $69.99 price, and
counts as yearly for the money-back guarantee. Renaming the row to `yearly` would make `priceOf` mail
"$84.37" to a $69.99 subscriber. If Quinn takes the recommendation in §8 item 9, production may hold no
such row at all.

**D2. The plan comes from the price id, not the interval.** The webhook looks up `items.data[0].price.id`
in a table built from four environment values:
- `STRIPE_PRICE_FOUR_WEEKLY` and `STRIPE_PRICE_YEARLY` (new);
- `STRIPE_PRICE_MONTHLY` and `STRIPE_PRICE_YEAR` (existing, `billing-checkout/index.ts:16-19`).

Any other id gives `plan: null`, which `billing-jobs` already counts as a failure without mailing a guessed
price (`billing-jobs/handler.ts:69-75, 119-123`). **All four values are optional in the webhook.** A missing
one matches nothing and is logged once per cold start, never answered 500, so one unset value cannot take
down every event. Checkout requires the two new values (D4).

*Reasons:*
- Both yearly prices bill by `year`, so the interval cannot tell them apart.
- Today a `week` interval maps to `null` (`stripe-webhook/handler.ts:66-67`).
- Price ids are public, set per project (`docs/plans/2026-09-09-c1-accounts-plan.md:2207-2209`).

**D3. Older builds cannot buy the new prices.** `billing-checkout` sells two plans, `four_weekly` and
`yearly`. Validation runs in this order:

1. **Old plan names.** A request for `monthly` or `academic_year` is refused with 409 and the sentence "This
   version of Knowlu can't subscribe any more. Update Knowlu, then try again."
2. **The terms version's shape.** It must match `^\d{4}-\d{2}-\d{2}$` and sort no later than tomorrow
   (UTC). Anything else is refused with 400: "a version must be a YYYY-MM-DD date no later than tomorrow".
   The account function checks only the comparison with tomorrow (`account/handler.ts:199-201`). Here the
   pattern comes first, so a value such as "2026-11-19x", which sorts between the floor and tomorrow, is
   refused too.
3. **The floor.** A terms version older than the terms that state these prices (a constant,
   `CHECKOUT_TERMS_VERSION`) is refused with 409 and the same update sentence.
4. **Unknown plan names** stay a 400.

These checks read only the request, so they come first: after the session check and the parse
(`billing-checkout/handler.ts:92-93`), and before the account read, the 18+ check and the tester check
(`:98-107`). The app shows the server's
sentence as written (`app/src/account.rs:210-217`).

*Reasons:*
- **Consent integrity.** The consent row records the version the client sends (`handler.ts:120-127`), and
  every earlier build sends 2026-09-10 (`account.rs:24, 1300`). If `monthly` were aliased to the new plan,
  Knowlu would record consent to the $9.99 terms for a $12.99 charge.
- **Early placement.** Refusing first means every old build gets the same answer whatever its account
  state. It also means the deploy check in §4 step 10 can tell this refusal from the tester refusal by its
  body.
- **The shape check.** Values such as "9999-99-99" or "2026-11-19x" would otherwise pass the floor and be
  written into `consents.version`.
- **A consequence, intended.** The floor equals `TOS_VERSION` (D11's tie). So every later terms bump, a
  typo fix included, refuses every older build at checkout. A build that shows terms must send the version
  it shows, and the updater offers the new build.
- **Who is affected.** Almost nobody: nobody but Quinn has the app (v0.4.0 note, A1), and the updater offers
  0.4.1.

**D4. Checkout checks the price before selling it.** `index.ts` maps each plan to its price id and its cents:
`four_weekly` is 1299 and `yearly` is 8437 (today's map is `index.ts:16-20`). Before the consent row is
written, the function reads the price from Stripe and requires six things:
1. the price is `active`;
2. the currency is `usd`;
3. `unit_amount` equals the plan's cents;
4. `recurring.interval` is the plan's own interval: `week` for `four_weekly`, `year` for `yearly`;
5. `recurring.interval_count` is the plan's own count: 4 for `four_weekly`, 1 for `yearly`;
6. `tax_behavior` is `exclusive`.

The pair is tied to the plan, not just to a list of allowed pairs. Otherwise a $12.99 price created as
yearly by mistake would pass under `four_weekly`.

Any failure answers 500 "the function is not configured" and logs which field failed, by name only.

*Reason:* `price_cents` comes from the map, not from Stripe. Without the check, a price entered wrongly in
the Dashboard would be sold silently under terms that say something else. That covers a wrong interval,
and an inclusive price under terms that say tax is added (U12). The check costs one Stripe GET.

**D5. 14 days free, once per account, and no second live subscription.** The checkout reads the account's
`entitlements` row (`stripe_subscription_id`, `status`; `20260910000200_billing_jobs.sql:13`):
- **A row with a subscription id whose status is `active`, `trialing`, `past_due` or `unpaid`** is refused
  with 409: "This account already has a subscription. Manage it in Settings, then Membership." The refusal
  comes before the customer and the consent row. A student whose card failed is sent to Manage
  subscription to fix it, not to a second subscription while the first is still dunning.
- **No row with a subscription id:** the session asks for `subscription_data[trial_period_days]` = 14.
- **Any other row with a subscription id** (`canceled`, `incomplete_expired`, and so on): no trial.

`payment_method_collection: always` stays (`handler.ts:60`), for the reason its test gives
(`handler_test.ts:22-25`).

*Reasons:*
- Ruling (7), and P3's one trial per account.
- Two checkouts in parallel would otherwise give two trials and two subscriptions. Account deletion sets
  only the one stored id to cancel at period end (`account/handler.ts:88-94`), so the second would bill a
  deleted account. The app asks before opening checkout; this is the server's backstop.

*Known limit:* a deleted account's address can trial again on a new account.

**D6. The reminder is Knowlu's own email, sent by the webhook** on Stripe's
`customer.subscription.trial_will_end` event. Stripe sends that event three days before a trial ends
(`unverified (Stripe)`: docs.stripe.com/billing/subscriptions/trials).
- **When it is sent.** Only while the subscription is `trialing`, is not set to cancel at period end, and
  has a `trial_end` still in the future. A retry that arrives after the trial ended sends nothing and is
  recorded.
- **What it says.**
  - The plan's words, when the price id is known.
  - The amount, taken from the event's own price (`unit_amount`, `currency`, `recurring.interval`,
    `recurring.interval_count`): "$84.37 once a year" or "$12.99 every 4 weeks". Any other interval is
    written as "every N weeks", "every N months" or "every N years", with N = 1 written without a number.
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
- `custom_text` (D8, P9).

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

**D9. The app's subscribe step and labels.**
- **The wizard** (`app/static/js/wizard.js:32-36`).
  - "Yearly" comes first, red (`ibtn`), and sends `yearly`. "Every 4 weeks" comes second (`wbtn`) and
    sends `four_weekly` (`:642`).
  - Each button carries §1's price lines, built from the same figures as the cents map. A test ties the
    two (§6).
  - The lede drops "and shows the price".
  - The element ids stay the same: no student sees them, and renaming them would churn the DOM checks.
  - *Reason:* P9 governs once A2's wizard clause ends. Ruling (2) gives the order and the yearly wording,
    and P11 sets the figure ($6.49 × 13 = $84.37), with the billed amount beside it.
- **The upgrade sheet** (`app/static/js/carried/account.js:83, 137`) gets the same two buttons and lines in
  place of its one Subscribe button. *Reason:* the amendment prices the 4-week plan "to make the year the
  obvious buy" (`:1111-1112` there). A sheet that sells only that plan sends a lapsed student to the one the
  price steers away from. This replaces the 2026-09-14 one-plan ruling (`HANDOFF.md:80`), so Quinn confirms
  (§8 item 6).
  - Only the two buttons start a checkout. Today's handler starts `subscribe()` from any click inside
    `#up-subscribe` (`carried/account.js:214`), so a click on the price lines would open a checkout with no
    plan.
- **Settings** adds `four_weekly: "Every 4 weeks"` and keeps `monthly` and `academic_year` for old
  subscriptions. It still shows no price.
- **The Customer Portal** offers no plan switching, because the terms describe none.
- **If Quinn keeps the app free of prices** (§8 item 5): the price lines are not built, and the A2 pins
  gain the new figures instead. Item 1's amendment then records A2 as narrowing P9, so the signed text is
  changed rather than read around.

**D10. The billing emails.**
- `priceOf` gives "$12.99 every 4 weeks, plus tax where it applies" and "$84.37 once a year, plus tax
  where it applies". The two old plans keep their text (P10).
- The pause stays `monthly`-only (`billing-jobs/handler.ts:130`), and the annual reminder goes to every
  plan.
- "click the gear" (`:86`, `:101`) becomes "go to Settings, then Membership". "Manage subscription" stays
  (`billing-jobs/handler_test.ts:153`, `app/tests/ui_settings.rs:132`).

**D11. The terms publish with this lane, as one new version.**

*What the text is made of.* It starts from the legal spec's interim terms (§4, "Interim terms with bump A",
U10). Their *The subscription*, *How to cancel* and *Refunds* sections are replaced by §4's full text. These
paragraphs are left out, because this release builds none of their code:
- the confirmation email (C8);
- the yearly renewal notice (C10);
- "a disputed charge is not a paid period" (C12);
- the "We" and shutdown paragraphs (C13);
- *A year for someone else*. The amendment has the gift year (ruling 9), and R3 says to keep it if the
  amendment has it. These terms still leave it out until the gift lane ships, because terms must not
  promise a checkout that does not exist. Quinn confirms (§8 item 7).

*What it adds.*
- A new section, *Emails about your subscription*, naming the trial reminder (D6) and the annual reminder
  (D10).
- "Sold through Link", only if Managed Payments is in effect (D7).
- The old-subscriber paragraph, only if production still holds an old-price subscription at step 7.

*The terms date* is the commit's own date (UTC). It must be no later than the day after the staging proof,
because checkout (D3) and the production account function both refuse a version later than tomorrow.

**These terms replace bump A's interim terms**, so bump A carries privacy only. `TOS_VERSION`
(`account.rs:21-24`), checkout's `CHECKOUT_TERMS_VERSION` and the page move in one commit. A new test ties
the three, beside the privacy tie (`app/tests/bundle_config.rs:134-157`). Until that commit, the floor
equals today's 2026-09-10. `site/subscribed.html:13` changes to say 14 days, a first subscription, and
Settings → Membership.

*Existing accounts.* This holds whatever `account` function production runs:
- **A paying student** accepts the new version at checkout, because the auto-renew row carries it (D3).
- **For everyone else**, T0's read-only count decides, counting only **accounts that are not Quinn's**.
  Quinn has two production accounts (`HANDOFF.md:58`), and names them; the ledger records counts.
  - If every account is Quinn's, Quinn's acceptance is recorded by hand on each of them (legal spec §4).
  - If any account is not Quinn's, C1 (T5b) joins the lane and redeploys `account` before the terms
    publish.

**D12. The privacy page names Link (only if Managed Payments is in effect, D7).** Link receives the card, the billing
address and the email address. The page's list of companies is closed ("Nobody else gets anything",
`site/privacy.html:86`).
- The text uses the legal spec §3's B variants (*Your subscription*, the *Link* bullet, *Delete*).
- "Seven companies" (`:75`) becomes the legal spec's D3 wording.
- A build carries one privacy key (`app/tests/ui_account_privacy.rs:226-233, 257-258`). If bump A rides
  v0.4.1, these paragraphs join it. If not, this lane makes a Link-only bump.
- Quinn reads the page.

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
- **The parity gate.** If production lacks `20261020000500_testers.sql` or `20261020000600_comp_guards.sql`,
  a named precondition is set: **"production parity for v0.4.1"**, on Quinn's go (§8 item 2). It is needed
  before item 9's grant and before step 10.
  - **Its scope.** `db push --include-all` pushes every pending local migration and has no "through"
    option. So the push runs from a **scratch copy of `cloud/supabase` whose `migrations/` ends at
    `20261020000600_comp_guards.sql`**, as L9 does (`HANDOFF.md:598`). L8's `entitlement` redeploy
    follows.
  - **The list first.** The main session shows Quinn the exact list of files the push will apply.
  - **If the gap is wider.** If anything missing below that stamp needs a secret, a Vault entry or a
    function redeploy beyond `entitlement` and `billing-checkout` (for example email's migrations, whose
    cron needs Vault entries first), the precondition is L9 proper, and v0.4.1 waits for it.
- *Why it gates step 10.* Today's checkout reads `effective_entitlements` (`billing-checkout/index.ts:35-42`),
  which the testers migration defines (`:76`). Without it every production checkout answers 5xx, and item
  9's comp grant has no table to go in.

**Staging, in test mode.**
1. Item 4 is settled, then Quinn takes the test-mode steps of the checklist.
2. The main session sets the two price ids on staging.
3. After T1-T4, the main session deploys `billing-checkout`, `stripe-webhook` and `billing-jobs` from the
   lane branch, and builds a T4 dev app.
4. **S1**, on that build. Quinn is at the test-mode Dashboard. Test cards only; no money moves.
5. **Checkpoint B.** Quinn rules on D7 if a gate failed.
6. After T5, the final staging proof on the PR head (plan §8 B, part 2).

**Production, in live mode, each step on Quinn's word.**

- **P-parity.** Done, if step 0 set it.
- **Item 9, if Quinn agrees**, for each of Quinn's accounts that holds a subscription:
  1. A comp grant through `admin.comp_grant`. It lasts at most a year
     (`20261020000600_comp_guards.sql:8-12`) and is extended with `admin.comp_extend`.
  2. Then Quinn cancels its old-price subscription **immediately**, not at period end, in both modes.

  The grant comes first, so the account is never without access. The view matches a grant only when the
  Stripe status is not `active` or `trialing` (`20261020000500_testers.sql:76-87`).
7. **Read-only checks again.** Each must equal step 0's, except:
   - no `active` or `trialing` row on `monthly` or `academic_year`;
   - item 9's comp grants.

   A new account that is not Quinn's stops the rollout and makes T5b required.
8. **The PR merges**, with CI green and every review passed.
9. Quinn takes the live-mode steps of the checklist, and reads back the live endpoint's enabled events.
10. The main session sets production's two price ids and deploys the three functions from the merge SHA.
    Then Quinn reads the live endpoint's recent deliveries: every one answered 2xx since the deploy.
10b. **The pre-tag live proof**, on a fresh production scratch account. It is created by emailed code at an
    address Quinn supplies, never written in the repository, and it counts as a founder test account.
    - **The old-build refusal.** `POST /billing-checkout` with `{"plan":"monthly","terms_version":"2026-09-10"}`
      answers 409 with D3's exact sentence in `error`. That body proves the new function is live, apart
      from the tester refusal.
    - **The live pages.** `yearly` and `four_weekly` checkouts each open a live page. A headless read shows:
      - $84.37, then $12.99 every 4 weeks;
      - 14 days free, the tax line and D8's sentence;
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
    `subscribed.html` says 14 days. The updater offers 0.4.1.

*Why the functions go before the tag.* The other order lets old builds buy at $9.99 while the published
terms say $12.99, and v0.4.1 builds would get 400s. Between steps 10 and 11 no build can subscribe at all.
That is acceptable, because outside sales are closed (Amendment 2026-10-16b) and the only install is
Quinn's. Step 11 follows on the same day.

*If the round trip is declined, or the event does not fire on a trial-end move.* Quinn's read of the
endpoint's events stands. Production's `EMAIL_*` is then proven only by one `mail-canary` send, which uses
the same key (`mail-canary/index.ts:6, 174`), if that function is on production. The reminder's own code
path is then proven on staging only. Quinn accepts that gap at checkpoint D.

**Quinn's checklist.** Do each step in test mode, then in live mode at step 9.
- [ ] On the existing product, create a price of **USD 12.99, recurring every 4 weeks** (week × 4), **tax
      behaviour exclusive**. Send the id; it is public.
- [ ] On the same product, create a price of **USD 84.37, recurring yearly**, tax behaviour exclusive. Send
      the id.
- [ ] Keep the old prices while any subscription uses them (P10).
- [ ] **Confirm that Managed Payments is on** in live mode (recorded 2026-09-14). Turn it on in test mode if
      it is off. Check the product's tax code. Read the 2026-09-24 payment's seller, which settles D7's
      reading.
- [ ] Add **`customer.subscription.trial_will_end`** to each webhook endpoint (today's list:
      `docs/plans/2026-09-09-c1-accounts-plan.md:2198`). Adding it early is safe: today's webhook answers
      200 "ignored" (`stripe-webhook/handler.ts:118-125`). In live mode, read back the enabled events.
- [ ] **Customer Portal:** cancellation on, no survey, **plan switching off**.
- [ ] Note whether Stripe's own trial-reminder email is on. Turn it off if Link sends one.
- [ ] **Deactivate the live 100% promotion code** once item 9's comp grants exist (P10: comp grants are the
      comp mechanism). Otherwise it works on the new prices too.
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
- **P10.** Old subscriptions keep their price, name and pause. The pause runs for them alone.
- **`billing-jobs`.** One failure never stops the run, and no guessed price is mailed
  (`billing-jobs/handler.ts:14-16, 69-75`). Logs name the account id only.
- **The app.**
  - The subscribe step's figures equal the cents checkout charges.
  - Settings shows no price.
  - No trial word or trial length appears anywhere in the app.
  - "Manage subscription" keeps its exact words.
- **The wording rule.** Every price Knowlu writes says "every 4 weeks" or "once a year". None says "a month",
  "/mo" or "monthly" beside a price.
- **The legal pages.**
  - The version ties hold: privacy, and the new terms-floor tie.
  - A build carries one privacy key.
  - The existing `engine/tests/site.rs` pins pass.
- **Releases.** Releases are CI-only. Production steps and the tag wait for Quinn's word.

## 6. Tests that prove it (each written first, and failing)

**Cloud, `deno test`** (run as `ci.yml:153-172` runs it; no test reads a `STRIPE_*` or `EMAIL_*` value from
the environment).

**`billing-checkout/handler_test.ts`:**
- The two plans reach Stripe with their price id and cents.
- D3:
  - the old names answer 409 with the exact sentence, with only the session check before it (no account,
    tester or Stripe call);
  - a version below the floor answers 409;
  - a malformed version or one later than tomorrow answers 400, including **"2026-11-19x"**, which sorts in
    range and is caught only by the pattern;
  - an unknown plan answers 400.
- D5:
  - "14" on a first subscription;
  - absent after a canceled one;
  - 409 when the row is `active`, `trialing`, `past_due` or `unpaid`.
- D4: one case per check: amount, currency, `active`, `tax_behavior`, interval and count. That includes
  **`four_weekly` pointing at a `year` × 1 price at 1299**, and the reverse for `yearly`. Each answers 500
  before the consent row.
- D7: every load-bearing key is present (the trial key on a first subscription only, D5), and the dropped
  keys are exactly those checkpoint A allowed.
- D8: `custom_text` for each plan and trial combination.
- The 18+ and tester tests are unchanged.
- A text-read of `index.ts` (`handler_test.ts:331-343`) checks:
  - the environment names;
  - the cents;
  - that `CHECKOUT_TERMS_VERSION` is declared once, as a quoted `YYYY-MM-DD` literal.

  Its value is `bundle_config`'s to tie, so T5's commit changes no checkout test.

**`stripe-webhook/handler_test.ts`:**
- The plan comes from the price id for all four ids; an unknown id gives `null`; a missing environment value
  matches nothing and does not answer 500.
- `trial_will_end`:
  - one email, naming the plan words, the amount taken from the event's price, "plus tax where it
    applies", the UTC−10 date and "Manage subscription", and never "in 3 days";
  - an unknown price id still gets the email, without plan words;
  - nothing is sent if the subscription will cancel at period end, is not `trialing`, or has a `trial_end`
    already past;
  - a failed email gives 502, nothing is recorded, and the log names the account id only;
  - a duplicate event sends nothing;
  - no entitlement is written.

**`billing-jobs/handler_test.ts`:**
- The new plans are never paused, get the annual reminder in the new wording, and never say "per month".
- The old plans' mails are unchanged (the "$9.99" pin at `:149`).
- "Manage subscription" stays, and "gear" is gone.

**App, `cargo test`.**
- **`ui_wizard`**
  - `the_subscribe_buttons_send_the_signed_plans`: order, classes and values, and billing's list of plans.
  - `the_wizard_states_the_signed_prices_and_no_trial` (replacing `the_wizard_states_no_price_or_trial`):
    - the subscribe panel carries ruling (2)'s exact line, "$12.99 every 4 weeks.", the money-back line and
      "Plus tax where it applies.";
    - it reads every string literal in `wizard.js` **and `picker.js`**, as today's test does, with the
      status word `"trialing"` as the one exception;
    - "$" and "a year" appear only inside those four lines;
    - no literal contains "9.99", "69.99", "a month", "per month", "/mo", "trial", "14 days", "14-day",
      "free first" or "Pell".
- **`ui_settings`**
  - The table carries the four plan words.
  - `membership_shows_no_price` gains "84.37", "6.49" and "14 days".
- **`ui_plans` (new)**
  - The sheet sends both plans, Yearly first, and only its two buttons start a checkout.
  - Every plan the app sends is one billing sells.
  - Every name the webhook can write has a word in Settings.
  - **The figures in `wizard.js` and `carried/account.js` equal `index.ts`'s cents**: "$12.99" for 1299,
    "$84.37" for 8437, and "$6.49" for 8437 / 13.
- **`bundle_config`:** `the_terms_version_constant_is_the_published_terms_date`, which also requires
  `CHECKOUT_TERMS_VERSION` to equal `TOS_VERSION`.
- **`engine/tests/site.rs`**
  - The terms contain the signed figures and clauses.
  - The terms contain no "a month", "per month", "/mo", "$9.99", "$69.99", "Pell", "Alabama" or gift
    paragraph.
  - `subscribed.html` says "14 days".
  - If Managed Payments is in effect, the terms say "Sold through Link" and the privacy page names Link.
    Otherwise neither names Link.

*If Quinn keeps the app price-free (§8 item 5):* the wizard test keeps r1's shape, banning the new figures,
and `ui_plans` drops the figure check.

**S1, the staging proof and the pre-tag live proof** (the main session; plan §8). These show what no unit
test can:
- the real pages;
- both plans written by the webhook;
- the reminder delivered, in test mode and, if Quinn agrees, in live mode;
- the trial on a first subscription only;
- cancelling and deletion;
- the wizard's prices against Stripe's;
- the old-build refusal by its body.

## 7. Not in this lane

These stay in the v0.5.0 billing lane or later:
- **C1**, terms re-acceptance, unless T0's count brings it in as T5b.
- **C2**, emailing every account on a material policy change.
- **C7's record**: the consent record keeping the plan and the text shown. `price_cents` already tells the
  two plans apart.
- **C8**, the confirmation email.
- **C10**, the yearly renewal notice. It must land before the first yearly renewal, which comes no sooner
  than a year and 14 days after the first yearly checkout.
- **C11**, the earlier pause-resume notice.
- **C12**, refunds and disputes in the webhook.
- **C13**, closing an account, and the shutdown.
- **C15** and **C16**.
- **The gift year** (C24), **Pell**, and **plan switching**.
- **The trial in the app's text.**
- **Any price on the homepage.** A2's homepage clause stands: "to be announced".

Outside paid sign-ups still wait on Amendment 2026-10-16b's gate. v0.4.1 makes billing correct. It does not
open sales.

## 8. Only Quinn can answer (in the order they block)

1. **Put Amendment 2026-11-10 on `main`, with an amendment recording the 2026-11-17 rulings and items 3, 5,
   6 and 7.** Blocks T1. *Recommend:* yes.
2. **The production parity push, if T0 finds the testers or comp-guard migration missing** (§4 step 0).
   Blocks item 9's grants and step 10. *Recommend:* yes, if those two stamps are the whole gap, pushed from a
   scratch copy ending at `20261020000600`. Otherwise it is L9 proper, and v0.4.1 waits for it.
3. **Keep `academic_year` rows under their own name at $69.99, rather than mapping them to `yearly`** (D1).
   Blocks T2. *Recommend:* keep them, recorded in item 1's amendment.
4. **Moving Stripe to KNOWLU LLC.** Blocks the test-mode checklist, and so S1. *Recommend:* defer a new
   account until after v0.4.1, but before 10-16b's gate opens sales; after that it means recreating every
   paying subscription. A new account costs new secrets, a new endpoint, product, prices, portal and
   Managed Payments setting, every stored customer id cleared, and S1 again. If the move is only the same
   account's business details, do it before step 9, so the charge's merchant name matches the pages.
5. **Show P9's price lines on the subscribe step, since A2's wizard clause ends with this lane** (D9).
   Blocks T4. *Recommend:* show them.
6. **The upgrade sheet sells both plans, Yearly first, replacing the 2026-09-14 one-plan ruling** (D9).
   Blocks T4. *Recommend:* yes.
7. **Leave the gift year out of these terms until the gift lane ships** (D11; the R3 reading). Blocks T5.
   *Recommend:* leave it out.
8. **A mailing address for the terms' Notices clause.** Blocks T5. *Recommend:* give it before T5, or rule
   email-only (it then goes on the lawyer's list).
9. **On each of your accounts that holds a subscription: a comp grant, then cancel the old-price
   subscription immediately, in both modes** (§4). Blocks T5's old-subscriber paragraph, and is done before
   step 7. *Recommend:* yes.
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

*Settled outside this lane, and not on this list:* the tag replaces the whole live site with `site/`
(`release.yml:294-297`). The controller ports the live site into `site/` (branch `site/port-live-1119`)
and merges it before the v0.4.0 tag. The pre-tag gate checks that `site/` equals the live site plus T5's
pages.

## 9. Answers to the reviews

Item numbers below are r3's §8 list.

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
