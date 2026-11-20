FAIL

# Review, round 1: the draft Amendment 2026-11-10 (the price)

**Reviewed:** `docs/specs/2026-09-09-knowlu-cloud-design.md:993-1087`, the uncommitted draft on `docs/waitlist-spec`, against base `5828b361`. Nothing else in the diff (`git diff --stat`: that one file, +96).
**Mode:** spec/plan, round 1 of 2.
**Verdict: FAIL. 0 Critical, 13 Important, 12 Minor.** Quinn's rulings are not in question, and every finding has a fix. The draft should not be signed as written, for three reasons. Ruling (6) has instants and cases that a planner cannot build without Quinn's word. The consequences list misses about a dozen surfaces in the code, two of which would send a Pell student a wrong price in a legally required email. And the disclosure sentence ties the disclosure to the wrong moment.

Arithmetic: every figure in the margin table and the "Why" paragraph recomputes correctly. The one exception is "about twice" (M3).

---

## Important

### I1. "Checkout states this rule before the student pays" names the wrong moment (`:1024`)
- **Conflicts with:** ruling (7) (`:1025`): the card is taken at day 0 and the first charge falls at day 14. ROSCA, as summarized in the legal landscape (`:252`), requires the material terms before the billing information is obtained, not before the charge.
- **Scenario:** a planner reads "before the student pays" literally. The rule goes into the day-0 claim email, or a notice before the day-14 charge, but not onto the screen where the card is entered. The student consents to a $44.85 yearly plan without being shown, at consent, that it can become 6 months and renew at $89.70.
- **Also:** checkout is Stripe-hosted (`billing-checkout/handler.ts:1-8`). The only screen this repo controls before the card is the app's subscribe step (`app/static/index.html:135-136`).
- **Fix:** "The subscribe step and checkout state this rule, with both amounts ($12.99, $89.70) and the 6-month outcome, before the card is entered, and the consent record keeps it (I8)."

### I2. Ruling (6) on yearly: two instants are undefined (`:1018-1021`)
- **(a) "the plan ends 6 months after it began".** The subscription begins at the trial's start; the $44.85 is paid 14 days later.
  - Scenario: claim on 1 Sep, charge on 15 Sep, no letter.
  - Counted from 1 Sep, the plan ends on 1 Mar. That is 5½ paid months for half the yearly price, below the "regular yearly rate" the sentence promises.
  - Counted from 15 Sep, it ends on 15 Mar.
  - Fix: "6 months after the first charge".
- **(b) "A letter that arrives late … on yearly restores the full 12 months".** This does not say what happens once the shortened period has already renewed.
  - Scenario: the student's letter arrives in month 7, after the $89.70 renewal in month 6.
  - The sentence could mean four different things: refund $44.85, credit it, extend the new year by 6 months, or nothing until the next renewal. "Never charges the difference" does not answer it.
  - Fix: Quinn rules on one, with the recommendation: before the shortened end, restore the 12 months; after it, the Pell price applies from the next renewal, with no refund.

### I3. Ruling (6) can be repeated without ever sending a letter (`:1013-1017`, `:1025`)
- **Code:** every checkout grants a trial (`billing-checkout/handler.ts:59`, `trial_period_days` hard-coded, no check for an earlier subscription). Under (5) every checkout can carry a claim.
- **Scenario:** a student claims Pell monthly and gets 14 free days plus one month at $6.49. No letter arrives, so they cancel before the $12.99 charge and subscribe again, with a new trial and a new claim. That is about 44 days for $6.49, indefinitely, with no proof.
- **Fix:** add to (5)/(6): "One unproven claim per account: after a claim lapses, the Pell price needs an accepted letter first." Add to (7): "one trial per account". Without the second, the loop works even without Pell.

### I4. The letter's deadline while a check is pending, and the 7-day extension, are ambiguous (`:1021-1022`)
- **Scenario A:** a letter is uploaded on day 26 and checked on day 33. Did the deadline pass?
  - On yearly, the plan was shortened on day 30. Is it then restored?
  - Nothing says whether a pending letter holds the deadline.
- **Scenario B:** a letter is rejected on day 29.
  - "7 more days" could run from the rejection (day 36) or from the deadline (day 37).
  - "within 7 days of the deadline" could mean before it, or before or after it.
  - A second rejection inside the extension could chain without end.
- **Fix:**
  - A letter uploaded before the deadline holds it until the letter is decided.
  - A rejection gives 7 days from the rejection, or until the deadline if that is later, once only.
  - Define the deadline as an instant: the claim plus 30 × 24 h, UTC.

### I5. No notice before the shortened yearly renews at $89.70 (`:1014`, `:1019`)
- **The gap:** the reminders run on days 0, 23 and 29.
  - The next contact before the month-6 renewal at $89.70 is none.
  - `billing-jobs`' reminder fires 365 days after `started_at` (`billing-jobs/handler.ts:61-65`), so it never fires before a 6-month end.
- **Scenario:** a student who ignored three emails in their first month is charged $89.70 five months later, on a date that differs from the one on their receipt.
- **Conflicts with:** the published terms, which promise an email "before it happens, with the new price, the date and a link to cancel" (`site/terms.html:25`).
- **Fix:** add a notice before the shortened renewal, with the amount, the date and how to cancel. The lawyer sets the window (§ Legal, item 3).

### I6. The yearly re-check of ruling (4) of 2026-10-16 is undefined against award years and the shortened year (`:999-1000`, `:1083`)
- **Scenario A, monthly:** a letter for the 2026-27 award year is accepted in May 2027, a month before that award year ends on 30 June.
  - "Checked once a year" from acceptance gives 11 months of Pell price on an award that has ended.
  - At the award year's end, the student is re-asked within 6 weeks.
- **Scenario B, yearly Pell renewal:** is the renewal charge at $44.85 a new claim with its own 30 days and ruling-(6) outcome, or does it need the new letter first?
- **"Wrong award year" (`:1021`) has no definition.** A May claimant holds a letter for the year that ends in June, and next year's letter may not exist yet.
- **Fix:** if Quinn agrees, the draft rules on these three points, or names them explicitly as open items for the Pell spec. The recommendation:
  - "verified through" is the end of the award year;
  - the Pell price lasts to the first renewal after that date, which is treated as a new claim with 30 days;
  - the right award year is the one covering the claim date, or the next.

### I7. The consequences list misses surfaces the code has (`:1076-1081`)
These name a price, plan, trial or pause, or will have to change for four plans, and are not in the list:
- **`cloud/supabase/functions/stripe-webhook/handler.ts:67`.** It derives the plan from the price's interval alone (`month`→`monthly`, `year`→`academic_year`), so a Pell subscription cannot be told from a regular one. This is the source of I8.
- **`billing-checkout/index.ts:20`.** A hard-coded cents map (`999`, `6999`) feeds the consent log's `price_cents`, apart from the Stripe price ids. Scenario: only `STRIPE_PRICE_MONTHLY` is swapped in the dashboard, and the 3-year consent log records $9.99 for students charged $12.99.
- **`app/static/console.js:3079`.** A second checkout entry point (`#up-subscribe`, `app/static/index.html:268`) hard-codes `plan: "monthly"`. It has no yearly-first choice and no Pell claim.
- **`app/static/index.html:123`.** A second price line ("$9.99 a month after a 7-day trial"), beside `:135-136`.
- **`site/subscribed.html:13`** ("the seven-day trial").
- **`site/terms.html`:**
  - `:27-29`, the whole "summer pause" section;
  - `:25`, the price-change promise (I5, and Legal item 4);
  - `:36`, the annual reminder (I12).
- **`app/src/account.rs:24` `TOS_VERSION`, and `checkout_url_at` (`:1292-1301`).** A terms change bumps the first. A Pell claim changes the body of the second (`{plan, terms_version}`). Both are on the **contract list** (`app/src/account.rs`), so this work goes to `contract-engineer` and `contract-reviewer`, and the draft should say so.
- **The Stripe Customer Portal's plan-switching configuration** (dashboard-managed; `billing-portal/handler.ts:22` uses the default configuration). Scenario: once the four prices exist, a portal that offers plan switching lets a regular subscriber switch to $6.49 without a claim, or lets a lapsed claimant switch back. Fix: Pell prices are never switchable in the portal; switches go through our endpoint.
- **`cloud/supabase/migrations/testers_cases.ts:120` and `cloud/supabase/proofs/testers_staging.sql:212`.** Both use `academic_year` as a plan label. Harmless as data, but they must follow whatever plan vocabulary replaces it.
- **Documents:**
  - `docs/reference/app.md:272-274` ("$9.99/month"; "R3: the academic-year price and the June–August pause both stay");
  - `HANDOFF.md:243` and `:489` (Launch's "$9.99");
  - `VISION.md:234`, the pilot paragraph's "discounted price", tied to the first open item;
  - `docs/specs/2026-10-23-onboarding-redesign-draft.md:147`, the in-flight onboarding payment copy ("$9.99 a month, after 7 days free").

**Fix:** add these to the list. Also state that 2026-10-16's term was never built: the code is still at `academic_year`/$69.99, so there is nothing of the term to remove.

### I8. `billing-jobs` would send a Pell student a wrong price in the reminder California requires (`billing-jobs/handler.ts:71-75`, `:119`)
- **Code:** `priceOf` keys on the plan label, and the webhook's label cannot carry Pell (I7).
- **Scenario:** after the switch, a Pell monthly subscriber gets the yearly reminder, which `site/terms.html:36` sends because California requires it. It reads "Knowlu, $12.99 per month", when they pay $6.49. A required notice that misstates the price is worse than none.
- **Fix:** add to the consequences:
  - the webhook records the price (or the plan from the price's id or metadata), not the interval;
  - `priceOf` reads the amount actually billed;
  - the plan names chosen for the four prices are stated (does `academic_year` become `yearly` in `entitlements.plan`, and what happens to rows that already carry it?).

### I9. Amendment 2026-10-16b's gate still requires the 2026-10-16 prices (`:823`)
- **Conflict:** the gate's line reads "the terms (`site/terms.html`) carrying the Amendment 2026-10-16 prices". The draft replaces those prices but does not list this line among what it amends (`:998`).
- **Scenario:** before the first outside charge, the controller checks the gate literally, against $39.99 and $2.49.
- **Fix:** add "and 10-16b's gate line (`:823`) now reads the 2026-11-10 prices" to the authority paragraph.

### I10. The waitlist consequence covers a fraction of what the founding offer touches (`:1085-1087`)
- **What the draft says:** drop the `founding` column, §6's money work and the prices row.
- **What the design also threads the offer through** (`2026-11-09-waitlist-design.md`):
  - `offer_version` and `offer_cutoff` (§3.1 `:249`);
  - `waitlist_join`'s offer parameters (`:341`);
  - `waitlist_confirm`'s returned `founding` (`:344`, `:600`, `:613`);
  - retention's "founding facts billing needs" (`:401`), and the rejoin that keeps the founding flag (`:402`, `:1769`);
  - the registry's offer row (`:1283`);
  - the confirm page's offer display (`:1289`, `:1329`);
  - tests (`:1695`, `:1779`, `:1831`);
  - Q4's ruling (`:134-135`, `:1966`), which this amendment supersedes ("one discount shared…", "Pell students pay the lower price, never both", "the claim expires when the list closes");
  - the title and §1's scope (`:1`, `:158`, `:167`).
- **The privacy consequence:** §9.1's privacy-page bullet "if a price offer is running when you ask, which offer it was and whether you qualify for it" (`:1121`). Kept, it describes data that is never collected.
- **The "Why" paragraph's duty:** it says "the waitlist tests it [the yearly price] before launch" (`:1035`). The waitlist design has no price test, and its §6.1 forbids any price but the signed ones. Scenario: a waitlist planner adds an A/B price test to honour this sentence, and brings back per-row offer tracking.
- **Fix:**
  - Name all of these places.
  - Say Q4 is superseded.
  - Keep W7's "no `6.99`, no `founding` under `site/`" test as a standing guard.
  - Either define "tests it" (for example, "the quiet release's yearly-versus-monthly share is read before Launch") or strike it.

### I11. The Pell claim is a new data class beyond the letter (`:1066-1068`, `:1082-1084`)
- **What is open now:** only "what Knowlu keeps from the award letter".
- **Scenario:** the claim at checkout records "this account says it receives Pell", a financial-aid status that implies low income, on day 0, before any letter exists. It also goes to Stripe: the four price ids tell Stripe which students are Pell recipients. Today the privacy page says only "which plan you are on" (`site/privacy.html:38`), and Stripe's entry reads "payments and the subscription" (`:78`).
- **Undecided:** who or what reads the letter. A person reading financial-aid letters must be disclosed. A model check sends the letter to the inference provider, which is a new transfer.
- **Collision:** the waitlist lane also moves `PRIVACY_VERSION` (its Q2).
- **Fix:** widen the open item:
  - the claim, the Pell flag at Stripe, the reviewer of the letter, and retention all go on the privacy page before the claim ships;
  - the Pell flag never enters telemetry or logs;
  - the Pell lane and the waitlist lane each name which privacy bump carries their text, and in what order.

### I12. The annual reminder is described as yearly-only (`:1080`)
- **The text:** "its yearly reminder (kept for the yearly plan)".
- **What the code and terms do:** the reminder goes to every plan (`billing-jobs/handler.ts:128-135`, `:174-178`). The terms promise it to everyone (`site/terms.html:36`), and the code's own header gives California's ARL for monthly as the reason (`billing-jobs/handler.ts:8`).
- **Scenario:** a planner removes the monthly path along with the pause code beside it, and monthly subscribers lose a reminder the terms promise them.
- **Fix:** "its annual reminder (kept, for every plan)".

### I13. "Never cuts off access inside a paid period" has no rule for a refund or chargeback (`:1022-1024`)
- **Code:** the webhook handles no refund or dispute event (`stripe-webhook/handler.ts:97-122`), so nothing that reads `entitlements` learns of either.
- **Scenario:** a Pell yearly student disputes the $44.85 charge, or is refunded under the terms' 30-day rule (`site/terms.html:39`). Read literally, (6) forbids ending their access, and the 6-month job still sees a paid plan.
- **Fix:** add "a refunded or disputed charge is not a paid period", and name refund and dispute handling in the Pell spec's scope.

## Minor

- **M1. Section references (`:998`, `:1001`).**
  - The billing line is §5.1 (`:135`), not §3.
  - The body also names the old price, trial or pause at §2 (`:30`), §4.2 step 2 (`:109`), §10's VISION diff (`:293`, `:316`) and §13 (`:382`).
  - The existing 2026-10-16 markers on D2 (`:14`) and R3 (`:342`) state facts that become false ("the pause stands"). Say they are replaced, not appended to.
- **M2. 2026-10-16's margin paragraph and consequences paragraph are not addressed (`:782-796`).** The revisit trigger ("revisited if it runs above about $0.80") and the list of surfaces to change "to the term" stay live unless the draft says they are replaced.
- **M3. "75% needs about twice the Pell buyers" (`:1037`).** It is exactly twice the revenue. On what is kept, a 75% price ($3.25 a month / $22.43 a year) keeps $2.26 and $14.28, against $5.40 and $36.05. That needs about 2.4× the monthly buyers and 2.5× the yearly. Say "at least twice".
- **M4. The Pell yearly per-month display.** $44.85 / 10 = $4.485, which cannot be shown exactly ($4.49 × 10 = $44.90). Say whether Pell yearly shows a per-month figure at all.
- **M5. "If serving costs double, the Pell yearly keeps about 64%" (`:1056`).** That is at 2.9% + $0.30. At 7.1% + $0.30 it is 60%. Name the basis.
- **M6. Managed Payments is already on** (`HANDOFF.md:598`, "kept Managed Payments on (P3)").
  - It is not only a fee question. If Stripe is the merchant of record, that bears on the seller the terms name, on 10-16b's Kentucky-registration item, and on whether the conditional pricing of (6) can be expressed at all. Add it to the spike in I2's open item.
  - "Read off the first live charge" cannot be read off the only live charge so far, which was made with a 100% promotion code (`HANDOFF.md:39`). It needs a non-zero charge.
- **M7. Promotion codes stay open.** Checkout allows them (`billing-checkout/handler.ts:61-63`), and a 100% code exists in live mode (`HANDOFF.md:39`). "No founding price … no other discount" does not say whether the field stays, or whether a code can stack with the Pell price.
- **M8. When the reminders stop.** The day-23 and day-29 reminders should stop on acceptance, cancellation or the end of the subscription. Scenario: a student claims Pell, cancels in the trial on day 5, and on day 23 is told their price "becomes $12.99".
- **M9. Existing subscriptions and the pause's removal.**
  - Say what happens to subscriptions at old prices: the live 100%-code monthly subscription, staging test subscriptions, and any taken before the switch. Grandfather them, or migrate them with notice.
  - Ship the pause's removal before 1 June 2027, or keep the resume path while any `entitlements.paused` row is true. Otherwise a subscription paused at removal is never resumed (`billing-jobs/handler.ts:152-160`).
- **M10. A claim after checkout.** (5) allows a claim only at checkout. Scenario: a $12.99 subscriber learns of the Pell price in month 3, and their only route is to cancel and subscribe again, which I3's fix would block. Recommend a claim from Settings, priced from the next charge, with the same 30 days.
- **M11. Plan switches inside the 30 days.** State that a switch keeps the original claim instant. On monthly→yearly, the yearly's "began" for (6a) is the switch's charge.
- **M12. Tax beside the displayed amounts.** Kentucky's 6% is added (`billing-checkout/handler.ts:66-69`), so "$89.70 billed once a year" is not the amount billed there. Say "plus tax where it applies", or leave it to the lawyer.

---

## Answers to the dispatch's questions

**1. Consistency with the spec:** findings I9, M1, M2. D2, R2 and R3 are named correctly. "§3" is §5.1. The 10-16b gate line is missing. 10-16's margin paragraph and consequences paragraph are unaddressed. 10-16's open item on the award letter is carried forward correctly.

**2. VISION and the waitlist:** VISION's One edition line (`VISION.md:153-156`) and Launch (`:236`) are named; the pilot paragraph (`:234`) is not (I7). The waitlist consequence is incomplete (I10).

**3. The code:** surfaces missing from the list are in I7 and I8. The engine names no price. `engine/src/entitle.rs` reads status only, and the grep's engine hits are grade-scale numbers.

**4. Arithmetic:** every line below is correct, except M3.

| Figure | Recomputed | Draft |
|---|---|---|
| Monthly at 2.9% + $0.30 | 12.99 − 0.677 − 0.60 = 11.71 (90.2%) | ✓ |
| Yearly | 89.70 − 2.901 − 7.20 = 79.60 (88.7%) | ✓ |
| Pell monthly | 6.49 − 0.488 − 0.60 = 5.40 (83.2%) | ✓ |
| Pell yearly | 44.85 − 1.601 − 7.20 = 36.05 (80.4%) | ✓ |
| Monthly / yearly / Pell monthly / Pell yearly at 7.1% + $0.30 | 86.0%, 84.5%, 79.0%, 76.2% | ✓ |
| Pell yearly at double serving | 28.85 (64.3%) | ✓ (at 2.9%; M5) |
| 9 × $9.99 | $89.91; 21¢ above $89.70 | ✓ |
| 12 × $12.99 | $155.88; saves $66.18, 42.5% | ✓ |
| 14-day and 7-day trial | $0.28 and $0.14 | ✓ |
| 75% against 50% | 2× revenue; 2.4× / 2.5× kept | "about twice" (M3) |

**5. Can (6) be built?** Not without these decisions:

| Case | Where answered |
|---|---|
| The claim instant | Make it the subscription's creation (`checkout.session.completed`), not the moment the box is ticked. Scenario: a student ticks Pell, abandons Checkout and returns 3 days later. Fold this into I4's definition. |
| Plan switches | I7 (the portal), M11 |
| The yearly re-check against a 6-month year | I6 |
| Refund or chargeback | I13 |
| Cancelling in the trial after a claim | M8. Nothing is charged; the claim lapses with the subscription. |
| Fresh consent for the $89.70 renewal | Plausibly not needed if the amount and the 6-month outcome are disclosed and consented to before the card, recorded, and preceded by a notice. I1, I5 and the lawyer. The current consent record has one `price_cents` (`billing-checkout/handler.ts:113-120`) and cannot hold the conditional terms. |
| Whether a subscription schedule shortens a paid period | See below. |

On subscription schedules, the repo can confirm only what it does today. It never shortens a period:
- it has no subscription schedule, no `billing_cycle_anchor`, no price update on a live subscription, and no refund;
- the only calls that change a subscription are `pause_collection` (`billing-jobs/handler.ts:141`, `:160`) and `cancel_at_period_end` (`account/handler.ts:93`);
- the API is pinned to `2025-03-31.basil` (`_shared/stripe.ts:82`), and the webhook reads `current_period_end` from the subscription item (`stripe-webhook/handler.ts:68-74`).

So whatever mechanism is chosen must move the item's period end, or `entitlements.current_period_end` will go on showing 12 months. I cannot confirm that a schedule does this. The draft's "confirmed in the plan" is right, and it should go further: a test-mode spike, under Managed Payments (M6), is the plan's first task, and Pell yearly is not offered until it passes.

## Legal: flagged, not ruled on

"Checkout states this rule before the student pays" is plausibly short of both laws. The fix is in I1. These are the points the lawyer read already required by 10-16b's gate (`:820-821`) should cover:

1. **Timing and placement.** The rule must appear before the card is entered (ROSCA) and clearly and conspicuously, near the consent (California's ARL). On a Stripe-hosted page, decide what goes on the app's subscribe step and what goes on Stripe's page.
2. **Conditional material terms.** The $12.99 fallback amount, the 6-month term and the $89.70 renewal must be stated as amounts and dates. As I understand ARL, the acknowledgment the student can keep (an email) must restate the terms and how to cancel; the claim email may be that acknowledgment.
3. **Notices before a changed charge.** Before the first $12.99 charge, the day-29 reminder comes about 15 days earlier. Before the shortened $89.70 renewal there is none (I5). The lawyer sets the window, against ARL's price-change and renewal-notice rules as amended.
4. **The published terms.** Two promises would be broken: "the change will not apply to a period you have already paid for" (`site/terms.html:25`), which the shortening does, and the refund promise for a change that costs money (`:70`). The new terms have to frame (6) as a disclosed term, not as a change.
5. **"$8.97 a month, plus 2 months free".** Nothing is ever sold at $8.97 a month. Check the "free" claim and the per-month framing against the FTC's guides on "free" offers and price comparisons.
6. **Yearly cancellation without a partial refund** (`:1069-1070`). Check each state's pro-rata refund rules.
7. **The 3-year consent log.** It must hold the plan, the Pell claim and the conditional terms shown, not one `price_cents`.
8. **The annual reminder and a renewal notice for yearly terms.** Today the reminder lands about 14 days before a yearly renewal (365 days after a start that includes the trial).
9. **Managed Payments as merchant of record** (M6). Who the terms name as seller.
10. **The award letter.** Collecting a financial-aid document, who reads it, and the privacy-page words for it (I11). Also "never sends anyone to collections" against a disputed charge (I13).
