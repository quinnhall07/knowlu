PASS WITH FIXES

# v0.4.1 pricing: whole-branch review (T8)

Reviewer, 2026-11-21. Branch `feat/v0.4.1-pricing` at `e92b3d94`, against its merge base with `origin/main`
(`9f593aa2`). Authority: `docs/specs/2026-11-19-v041-pricing-design.md` (r5, "the note"); the plan
`docs/plans/2026-11-19-v041-pricing-plan.md`; the controller's 24 rulings. The task reviews in
`docs/reports/2026-11-21-v041-*.md` were read where a seam needed them.

`app/src/account.rs` is on the contract list. The contract reviewer reviewed it at T4a. This review checks
only its interface: `checkout_offer`, `checkout_offer_at`, `checkout_offer_answer`, `checkout_url_at` and
`open_checkout`, against their callers and the server.

I read the code in full, not only the diff, at every seam below. I ran one test file:
`deno test … _shared/price_catalog_test.ts` passed 8 of 8, the terms tie included. Nothing else was run.

**Result.** I found no Critical or Important defect in the code. The money path agrees end to end. The
offer, the purchase body, the 409s, the consent row, the plan key, the reminder wording, `billing-jobs`
and the three terms-version constants all agree on every name, type and sentence I traced. Three Minor
findings sit in the runbook, which a later task made partly untrue. The founder's list is longer than the
fix list: the published terms promise several things this branch's code does not keep on its own.

---

## Findings to fix on this branch

**M1 (Minor). `docs/reference/price-change.md:64-66`: the runbook names only one test that holds the selling amounts. There are two.**
- **What the runbook says.** Step 2 says the only test that names the selling amounts is
  `the_catalog_sells_two_plans_yearly_first`, and that nothing else changes.
- **What is true.** T5b (`3f7e028e`, merged after T11) added `engine/tests/site.rs:864`. That test
  requires `"$12.99"`, `"$84.37"` and `"$6.49"` literally in `site/terms.html`.
- **Failure scenario.** At the first price change, the operator follows step 2 and edits the catalog, the
  page, `TOS_VERSION` and the one named test. CI then goes red on `site::the_terms_state_the_signed_prices`.
  The PR is now editing a pin the procedure never mentioned, in a Contract PR, under time pressure.
  Nothing wrong ships, because CI stops it, but the procedure is wrong where it claims to be complete.
- **Smallest fix.** Name `engine/tests/site.rs::the_terms_state_the_signed_prices` in step 2, and carry
  the same line into note D16 step 2 with T6b's marker lines. The alternative is to drop the three
  literal figures from `site.rs:864`, because the deno tie `the_terms_state_the_catalogs_prices` already
  proves them against the catalog.

**M2 (Minor). `docs/reference/price-change.md:115-128`: step 5 says to publish, then deploy, "with no pause between", but never says to stop if the publish fails.**
- **Why it matters.** `site.yml` refuses in four cases: a newer `v*` tag, an unfinished `release.yml`
  run, a changed `site/privacy.html`, or a missing installer.
- **Failure scenario.** A `v0.4.2` tag was pushed earlier that day, so `site.yml` refuses. The operator
  takes "no pause between" literally and deploys the three functions anyway. Students who check out
  then consent to a `TERMS_VERSION` whose page is not on knowlu.com. That is the "consent to the wrong
  terms" case. Step 6's version check catches it only afterwards.
- **Smallest fix.** One sentence after item 3: "If the site run fails, stop. Do not deploy the
  functions. Fix the cause, then run it again."

**M3 (Minor). `docs/reference/price-change.md:60-63`: no rule for the date of the new terms version.**
- **The rule the runbook leaves out.** Note D11 states it: the version is the commit's own date (UTC).
  The production account function refuses any sign-up version later than tomorrow
  (`cloud/supabase/functions/account/handler.ts:199-201`).
- **Failure scenario.**
  1. The operator dates the new terms at the day the new price reaches current subscribers, 30 days out.
  2. The deno tie (`the_catalogs_terms_version_is_the_apps`) forces `TOS_VERSION` to the same future date.
  3. The next ordinary release, cut before that date, sends that `TOS_VERSION` at sign-in, and every
     sign-up consent is refused with a 400.
  4. The account's `age_attested_at` is never set, so every new account's checkout answers 403 ("the 18+
     attestation is missing"). This lasts until the date arrives.
- **Smallest fix.** Add to step 2: "`TERMS_VERSION`, `TOS_VERSION` and the page's date are the PR's merge
  date (UTC), never a future effective date. When a price reaches current subscribers is stated by the
  amendment's rule and the move notice, not by the terms date."

---

## For the founder (legal text and decisions)

Promises in the published terms (version 2026-11-21) that this branch's code does not keep on its own:

1. **"When you subscribe: a confirmation you can keep, with the plan, the price, how often it renews and
   how to cancel."** No code sends one; the note's D11 left this paragraph out. Ruling 16 puts this to
   you. Check at S1 what Link's receipt carries, and add the email before the tag if it does not cover
   all four items.
2. **"Before the yearly plan renews, between 15 and 45 days ahead: the amount, the date and how to
   cancel."** No code sends this either (D11 left C10's yearly half out).
   - `billing-jobs`' annual reminder is due more than 365 days after the start or the last reminder, so
     it arrives after the first yearly renewal, not before it.
   - The move notice goes only on a price change.
   - The first yearly renewal comes about a year and 14 days after the first yearly sale. Build the
     notice before then, or take the sentence out.
3. **"We email you at least 3 days before the first charge."** The webhook sends the email when Stripe's
   `trial_will_end` arrives, which is about 3 days before. If the first send fails, it answers 502 and
   Stripe retries later, so the email can arrive under 3 days ahead. That case may fall under refund (c).
   Decide whether "at least" stays.
4. **"Each account gets one free trial."** Two checkout pages opened before either is paid each carry 14
   days. This is the cloud review's finding 6, routed to you by ruling 8. A deleted account's address
   can also trial again on a new account (D5's known limit).
5. **"When we change these terms materially … Knowlu asks you to accept the new version, with its date,
   before you go on."** No in-app re-acceptance of the terms exists. C1 is not in this lane, and D15
   leaves sign-up recording the version compiled into the build. A material change other than a price
   would go out with no ask.
6. **Privacy page, "Your subscription":** "a record of the agreement itself — the plan, the price, the
   renewal terms you were shown, the version of the terms …".
   - The consent row stores the terms version, the cents, the IP address, the time and a hash of the
     email address. It stores neither the plan key nor the renewal wording.
   - The note's §8 item 16 (answered 2026-11-21) says that sentence is dropped. The published page
     still has it.
   - Fixing it means a privacy edit and a `PRIVACY_VERSION` bump that ships with a release.
7. **Refunds: "Your access ends the day of the refund", and cases (a) to (d).** Nothing in the code acts
   on a refund. Access ends only if someone also cancels the subscription in the Dashboard when
   refunding. This is manual by design. Say so in the operating notes.
8. **Promotion codes.** `allow_promotion_codes` is still on, and the reminder states the list price.
   This is the cloud review's finding 7, routed to you by ruling 8, and still open.
9. **The wizard and a lapsed card.** Take a student whose subscription is `past_due` who reinstalls.
   The wizard's Subscribe step answers "This account already has a subscription. Manage it in Settings,
   then Membership." The wizard has no Settings, so this student has nowhere to go from that step. It
   is rare, and there are zero customers today. Recommendation: accept it now, and name link.com or "reply to
   any billing email" in D5's sentence at the next checkout change.
10. **Copy you have not read:** `site/subscribed.html`'s "Otherwise your plan has started, charged as
    checkout showed." (ruling 21), and the sheet's "choose a plan again" (ruling 23).
11. **These depend on S1:** "Sold through Link" (pinned unconditionally in `site.rs`, ruling 15);
    "cancelling there takes one click"; "You can also cancel at link.com"; and Link sending the
    receipts.

## Could not verify

- **Whether knowlu.com already serves the 2026-11-21 terms.** If it does not, step 10b's production
  checkouts record a version before its page is live (§8 item 12, still open).
- **Stripe behaviour, which S1 and S2 are there to settle:**
  - `managed_payments[enabled]` accepted at the pinned API version;
  - `trial_will_end` and the subscription events delivered with `metadata.account_id`;
  - a no-proration change leaving a paid period alone;
  - a metadata-only write keeping `account_id`.
- **Things I did not run:** the DOM checks (t16, t20, t21), and the cargo and full deno suites. CI is
  green per the dispatch.
- **Outside the repository:** the `release` environment's branch rule, the Cloudflare preview alias,
  v0.4.0's Release assets, and whether any live subscription still carries `pause_collection`.
- **The local `v041-red/*` and `v041-green/*` tags.** They match `release.yml`'s `v*` trigger. One
  read-only `ls-remote` showed none on origin.
  - If they are ever pushed, each one starts a `release.yml` run, which fails at `release.ps1`'s
    tag-and-version check.
  - Per the T10 review's M4 they sort before `v0.4.0`, so they would not block `site.yml`.
  - Delete them after the merge.

---

## 1. The fidelity ledger, row by row (plan §6)

The row's test exists under that name and proves the note's requirement unless the row says otherwise. ⚑
marks a row that is open, not a finding.

| Note | Requirement | On the branch |
|---|---|---|
| §1 | journey | ⚑ the §8 B proof and step 10b |
| D1 | keys; old never sold or written | `billing-checkout::the_signed_plans_reach_stripe_with_their_price`, `::an_older_build_is_told_to_update`; `ui_plans::every_plan_the_app_sends_is_one_the_catalog_sells`; `ui_settings::plan_words_cover_every_plan_billing_can_write` |
| D1 | a key outlives its price | `price_catalog_test::every_entry_keeps_its_keys_cadence`; `stripe-webhook::the_plan_comes_from_the_price_id` (its `KEPT` case maps to the key) |
| D2 | plan by price id; unknown is null; optional values | `::the_plan_comes_from_the_price_id`, `::a_missing_price_setting_matches_nothing`, the `index.ts` text-read; S1 (a) ⚑ |
| D3 | old 409; unknown 400; missing 400; changed 409 before the account read | the four named tests; `an_older_build_is_told_to_update` asserts no call after the session check; step 10b ⚑ |
| D3 | consent row has the catalog's cents and version | `the_signed_plans_reach_stripe_with_their_price` asserts `price_cents` and `version`, and the call order |
| D4 | six checks plus an unset variable, tied to the key | `the_price_is_checked_before_it_is_sold` |
| D5 | trial on the first subscription only; card always; one live subscription | `the_trial_is_fourteen_days_on_a_first_subscription_only`, `an_account_with_a_live_subscription_is_refused` (refused before any price read or consent row); S1 (b) ⚑ |
| D6 | reminder: conditions, catalog words, unknown price, retry, log, no entitlement write | the six pins, plus `a_reminder_sent_but_not_recorded_is_not_retried` (ruling 7); the 1399 case is present |
| D6 ⚑ | event fires live | open |
| D7 | load-bearing keys kept | `the_form_keeps_every_load_bearing_key` |
| D7 ⚑ | reading; gates (a)-(e) | open |
| D7 | `_shared/stripe.ts` untouched | it is not in the diff |
| D8 | renewal sentence, catalog words | `stripes_page_states_the_renewal_terms` |
| D9 | offer from one catalog | `the_offer_is_the_catalogs`; `account::checkout_offer_at_reads_the_offer` |
| D9 | the purchase sends the offer back | `account::checkout_url_at_sends_the_offer_it_was_given`; `ui_wizard::the_subscribe_buttons_send_the_signed_plans`; t21 (not run) |
| D9 | holds no price; renders the offer | `ui_wizard::the_wizard_holds_no_price_and_renders_the_offer`; `ui_plans::the_sheet_holds_no_price`; t20 and t21 |
| D9 | no offer, no figure; reasons | `ui_wizard::without_an_offer_the_plan_buttons_wait`; t21's three envelopes |
| D13 | no dollar amount under `app/static` | `ui_plans::no_static_script_or_page_writes_a_dollar_amount` |
| D9 | the sheet: both plans; only the buttons start a checkout | `ui_plans::the_upgrade_sheet_sells_both_plans_yearly_first`, `::only_the_two_plan_buttons_start_checkout` |
| D9 | Settings words; no price | `ui_settings::plan_words_cover_every_plan_billing_can_write`, `::membership_shows_no_price` (gains "84.37", "6.49", "14 days"); `ui_plans::every_plan_the_catalog_can_name_has_a_word` |
| D9 ⚑ | the portal has no switching | open |
| D10 | pause gone; own price; cancel path | the five named `billing-jobs` pins |
| D13 | catalog rules | six `price_catalog_test` pins |
| D13 | no amount outside the catalog on the server | `price_single_source_test::no_amount_is_written_outside_the_catalog`; its rules are proved on supplied text |
| D13 | the terms tie | `::the_formatter_writes_every_4_weeks_and_once_a_year` (its figure list), `::the_terms_state_the_catalogs_prices`. The plan's `bundle_config` pin moved to deno by ruling 17 (`the_catalogs_terms_version_is_the_apps`). The page-to-`TOS_VERSION` half already existed as `ui_wizard::the_terms_version_constant_is_the_published_terms_date` |
| D14 | notice, then mark, then change; retries | the five named pins plus `price_catalog_test::a_move_entry_notices_inside_seven_to_thirty_days` |
| D14 | exact window, on Stripe's value | `the_window_is_exact`, `the_window_and_the_date_are_stripes` |
| D14 | trial rule (Q15 answered as recommended) | `a_raise_leaves_a_trials_first_charge`, `a_cut_moves_a_trial`, plus `the_raise_rule_skips_a_trial_before_the_selling_price_is_needed` |
| D14 | yearly subscriber | `a_yearly_subscriber_is_moved_before_the_yearly_renewal` |
| D14 | move before the reminder; optional variables | `the_move_comes_before_the_annual_reminder`; `billing-jobs` `index.ts` text-read; `an unset kept variable never answers 500 for the run` |
| D14 ⚑ ×2 | S2 (d), (e) | open |
| D15 | server's version; no build refused for its age | `a_changed_offer_is_refused`; `checkout_url_at_sends_the_offer_it_was_given` |
| D15 | `site.yml` guards | ten `workflows::site_yml_*` pins |
| D15 ⚑ | recovery wording | T11 and T6a reviews done; it matches (§2.6) |
| D15 ⚑ ×2 | the site-only run keeps the installer; a catalog change reaches an unchanged build | open (T10 preview, step 14, S2) |
| D16 | the runbook covers D16 | T11 review PASS; M1 to M3 are gaps found since |
| D11 | the terms text | `site::the_terms_state_the_signed_prices`, `::the_terms_carry_the_price_change_paragraph`. `::the_terms_carry_their_standing_clauses` replaces `…interim_terms_clauses`: a rename made on the T5b review's M5 (`-r2.md:17, 36-40`), because the published page is the full terms, not the interim ones. `::the_terms_promise_no_code_this_release_lacks` is not written (ruling 16); see founder items 1, 2, 5 |
| D11 | version tie; `subscribed.html` | see the D13 tie row; `site::the_subscribed_page_names_fourteen_days` |
| D11 ⚑ | existing accounts | open |
| D12 | Link on the privacy page; one key | `site::the_privacy_page_names_link` (T5b). The plan's `ui_account_privacy::the_panels_changes_name_link` arrived from main (#90) as `::the_panels_changes_name_the_launch_bumps_changes`, which requires "Link". `::the_panels_line_one_names_privacy_version_and_the_table_has_one_key` is present |
| §4 ⚑ ×5 | premise, parity, own subscriptions, rollout, tag site | open; §8 item 2 records parity as nothing to push |
| §5 | 18+, tester, gate unchanged | refactored onto `fakes()` with their assertions kept; the gate is not in the diff |
| §5 | webhook guards | the existing tests are present; the order is unchanged (`handler.ts:192-260`) |
| §5 | wording rule | the formatter, `custom_text`, reminder, wizard, terms and scan pins |
| §5 ⚑ | rule 1, rule 2, ids in reports | §6 below |

Every row's test is present, under its name or under a ruled or reviewed rename. None is weaker than its
requirement.

## 2. Seams between tasks

### 2.1 The offer
- **The server.** `price_catalog.ts:269-275` `offer()` returns `{ terms_version, plans: [{ plan,
  amount_cents, lines }], note }`, with plans in `SELLING` order (Yearly first). The lines are exactly
  note §1's.
- **The GET.** `billing-checkout/handler.ts:159-161`: GET answers it after `requireUser` and reads no
  account. Any method other than GET or POST is 405 naming both.
- **The app.** `account.rs` `checkout_offer_at` returns a 2xx object unchanged. `checkout_offer_answer`
  passes the server's three fields through and adds `ok` and `error`.
- **The page.** `wizard.js` and `carried/account.js` each run `offerFrom`, which:
  - keeps only `yearly` and `four_weekly` entries with an integer `amount_cents` and non-empty string
    `lines`;
  - requires a string `terms_version`;
  - otherwise says UPDATE_KNOWLU.
- **The cases.** A missing plan key gives "Update Knowlu". An empty `lines` gives "Update Knowlu" (T4
  review M4: no line, no offer). A server that adds a third plan is ignored. The order on screen comes
  from the markup, which matches the catalog's.
- Both sides agree.

### 2.2 The purchase
- **The button.**
  - The wizard maps `#wiz-sub-year` to `yearly`, the other button to `four_weekly`.
  - The sheet binds only `#up-sub-year` and `#up-sub-month`.
  - Each sends `{ plan, amountCents: offer.plans[plan].amount_cents, termsVersion: offer.terms_version }`.
- **The command.** `open_checkout(plan: String, amount_cents: i64, terms_version: String)` takes Tauri's
  camelCase. `checkout_url_at` posts exactly `{plan, amount_cents, terms_version}`. The consent retry in
  front still sends `TOS_VERSION` and `PRIVACY_VERSION` (sign-up's, D15).
- **The handler**, in order:
  1. retired name: 409 "This version of Knowlu can't subscribe any more. Update Knowlu, then try again.";
  2. unknown name: 400;
  3. a missing field: 400;
  4. not equal to the catalog: 409 "Knowlu's prices or terms have changed. Look them over again, then
     subscribe.";
  5. then the account, 18+ (403), tester (409), live subscription (409, D5's sentence), and the price
     check (500).
- **What the student sees.** `provider_error` returns the body's `error` as written. The wizard puts it
  in `WIZ.error` and asks for the offer again (`wizard.js:719`). The sheet puts it in `#up-error` and asks
  again.
- **The consent row** (`handler.ts:235-242`) records `entry.cents` and `catalog.termsVersion`. Step 4's
  equality makes those the shown amount and version.
- Agreed. The only gap is founder item 9: a wizard-side reader cannot act on D5's sentence.

### 2.3 The plan name
- **The webhook** (`stripe-webhook/handler.ts:149-154`) looks up `items.data[0].price.id` among
  `SELLING` and then `KEPT`, and writes the entry's key, or `null`. It can never write `monthly` or
  `academic_year`.
- **Settings** (`settings.js:33, 185-186`) words `four_weekly` "Every 4 weeks" and `yearly` "Yearly". Any
  other value, null included, reads as the status word alone ("Active").
- **`billing-jobs`** never reads the plan column; it reads the subscription's price id.
- **The gate** reads status only. The wizard compares the plan only to `"tester"`.
- **Checkout** answers `monthly` and `academic_year` with 409, and null or anything else with 400.
- **Plan words.** The webhook's and the job's `PLAN_WORDS` equal Settings' words. Cadence words without
  a figure are allowed by D13.

### 2.4 The reminder and `billing-jobs`
- **The reminder.** `trial_will_end` → `trialReminder`. For a known id: "Knowlu, Yearly, $84.37 once a
  year, plus tax where it applies." through `priceText`. For an unknown id: `unknownPriceText` from the
  event's own price.
- **The date** is UTC−10 (`chargeDate`), with no "in 3 days". The cancel path names Settings, Membership
  and Manage subscription.
- **`billing-jobs`** imports the same `chargeDate`. It reaches entries through `lookupPrice` and
  `sellingFor` on the same resolved catalog, and its annual-reminder sentence is note D10's exactly.
- **Price variables.** All three functions resolve them through `Deno.env.get`, every one optional
  except where checkout requires the selling id (`handler.ts:207-210`).
- The README records the deploy coupling ("Deploy all three together").

### 2.5 The terms version, through the rollout (plan §8 C)
- **The constants.** `TERMS_VERSION` (`price_catalog.ts:94`), `TOS_VERSION` (`account.rs:24`) and the
  page's "Effective" and "version" lines are all 2026-11-21. Deno pins the catalog to both.
- **Staging.** Since ruling 15, S1 records 2026-11-21, a version on main. Plan §8 B is now stale in two
  places, both for the controller:
  - B1 says the offer carries 2026-09-10;
  - B8's "an offer read before the redeploy (carrying 2026-09-10)" can no longer be staged as written.
    Any offer with another version gives the same 409.
- **Step 10 to step 11.** Each of the following was ruled acceptable or is already open:
  - Every v0.4.0 build gets the "Update Knowlu" 409 before v0.4.1 exists. Note §4 accepts this: the only
    install is the founder's.
  - Step 10b's production checkouts record 2026-11-21. That is a version not yet live on knowlu.com,
    unless the founder's merge was also deployed (could not verify; §8 item 12 open).
- **After the tag.**
  - A v0.4.1 build returns whatever version the offer named, so a later terms bump refuses no build.
  - Sign-up sends 2026-11-21, which is within the "no later than tomorrow" rule.
  - The only way to refuse every new sign-up is M3's future-dated version.
- **Later price changes.** The runbook's page-first order keeps the consent row on a published version,
  except in M2's case.

### 2.6 `site.yml` against `release.yml` and the runbook
- **The deploy command is identical:** `pages deploy site --project-name knowlu --branch main
  --commit-dirty=true`, with the same pinned `wrangler-action`.
- **The restore** brings back the same three files `release.yml`'s artifact carries: the versioned
  `*-setup.exe`, `Knowlu-setup.exe` (copied as `release.ps1:297` does) and `latest.json` (the manifest's
  name per `release.ps1`). The `.sig` is not matched, and the site never held it.
- **The privacy guard** is the same in all three: `site.yml:111-118`, the runbook's step 0 and its hand
  restore, and `app.md`'s recovery each compare with the latest Release's tag and stop on a difference.
- **"Never re-run a tag's publish job"** appears in all three.
- The runbook's live reuse rule (ruling 20) and its hand-restore caveat (check the tag and run guards by
  hand) agree with D15.

## 3. What must not break (note §5)
- **Price ids from the environment.** They come from `resolveCatalog(CATALOG, Deno.env.get)` in all three
  `index.ts` files. No id is written anywhere.
- **No person, account or address in code, tests or docs.**
  - New test data uses `example.invalid` only.
  - The lane's reports hold no id, address or local path. I scanned for each.
  - The runbook names a role, consistent with the repository's docs.
- **The date rule assumes no time zone:** UTC−10, `chargeDate`.
- **Nothing under `engine/src/` or `engine/tests/fixtures/` changed**, so no frozen reference is touched.
- **The gate.** `_shared/entitlement.ts` is untouched and reads status only. `trialing` is still
  entitled.
- **Checkout's order.** Every refusal comes before the customer, the consent row and the session
  (`handler.ts:166-221`). The call order is pinned in `the_signed_plans_reach_stripe_with_their_price`.
  The consent row is still written before the redirect.
- **Load-bearing keys** are all present (`checkoutForm`). Only `automatic_tax[enabled]` and
  `customer_update[address]` were dropped, at checkpoint A.
- **The webhook.**
  - Order: signature, then parse, then duplicates, then the account checks, then the stale guard. The
    reminder branch writes no entitlement.
  - It is still the only writer of `entitlements`.
  - No unset price value can answer 500: the catalog resolves through `Deno.env.get`.
  - The API pin stands: `_shared/stripe.ts` is untouched.
- **One source for prices.**
  - The server scan passes.
  - The app holds no figure. `ui_plans` and `ui_wizard` scan its literals, and no `$d.dd` appears under
    `app/static`.
  - The offer is never cached: it is asked again on every entry and after every refusal.
- **Prices that change.** A key never changes with its price. Variable names are derived. `move` sends
  the notice first, with N from 7 to 24, and makes the change with `proration_behavior: none`.
- **`billing-jobs`.** One failure never stops the run (a `try/catch` per subscriber). An unknown price is
  never mailed. Logs carry the account id only.
- **The app.**
  - Settings shows no price.
  - No trial word or length appears: the scans allow only `"trialing"`.
  - "Manage subscription" is unchanged.
- **The wording rule.** Every price the code writes says "every 4 weeks" or "once a year". The scan bans
  "a month" and "per month".
- **The legal pages.**
  - The ties hold: privacy through the existing pins, the terms through deno and `ui_wizard`.
  - A build carries one privacy key (2026-11-21).
  - `site.yml` refuses a changed privacy page.
- **Releases.** `release.yml` is not edited. `site.yml` restores the installer and `latest.json` before
  it deploys.

## 4. Scope
- **Every changed file is in a task's Owns column or covered by a ruling:**
  - `app/tests/ui_shell.rs` (ruling 10);
  - `app/README.md`'s command counts (ruling 18, T6a);
  - `ui_account_privacy.rs` and `ui_wizard.rs` in `7d18de97` (ruling 23).
- **One edit by a task outside its own column.** T5a's one-line edit to
  `billing-checkout/handler_test.ts` sits in T1's column. The commit message explains it: a literal now
  derives from `TERMS_VERSION`, so the case keeps its point. That is not a test bent to agree with new
  output.
- **`_shared/stripe.ts`** is untouched (D7).
- **Search of shipped code** (`app/static`, `app/src`, `cloud/supabase/functions` outside the catalog and
  tests), each hit judged:
  - **Dollar figures:** none, except the model-cost comments in `_shared/judge_caps.ts`, which include the
    known `$9.99` yardsticks at `:45` (D13 and D16 list them).
  - **"every 4 weeks":** only as the plan word "Every 4 weeks" (two button labels, Settings, the webhook's
    and the job's `PLAN_WORDS`). D13 allows cadence words without a figure.
  - **"14" or "trial":**
    - `billing-checkout/handler.ts:37`, `TRIAL_DAYS = 14`, and `renewalTerms`. D5 places the trial
      length in checkout, and D13's catalog does not cover trial length. A change would trip its own
      pins: "14" in the checkout test and "14 days free" in `site.rs`.
    - The reminder's subject, an email and not the app.
    - Comments.
    - `mail_batch`, `sweep` and `mail-pull`'s "14 days" are unrelated.
  - None needs a change.

## 5. Red tags, spot-checked
- **T01** (`v041-red/t01`, `d44c83a9`):
  - The commit holds the tests plus type-check scaffolding in `handler.ts`: optional `Deps` fields, and a
    `renewalTerms` stub that returns `""`. The message says so.
  - The stubs make no assertion pass.
  - The twelve failure reasons in `17acf9b1` follow from the red diff. For example, "GET is 405; only
    POST is allowed" matches v0.4.0's POST-only handler, and "the stub renewalTerms returns ''" matches
    the stub.
- **T4a** (`v041-red/t4a`, `69e5b482`): the commit holds `app/tests/account.rs` only. E0432 for the two
  missing functions and E0061 for the five-argument calls are what that diff produces.
- **T10** (`v041-red/t10`, `f327dd9f`): the commit holds `engine/tests/workflows.rs` only, and every pin
  fails because `site.yml` does not exist. The green commit corrected one red pin that could never pass
  (a `- uses:` against a `uses:` compare). The message discloses it, and the contract review read it.

## 6. The project's rules
- **No single-user assumption:** no account id, address, person or school is written. Price ids come only
  from the environment.
- **`engine/src/` and `engine/tests/fixtures/`:** untouched.
- **Tests stay on the machine:** the handlers take injected deps, `app/tests/account.rs` uses a loopback
  on `127.0.0.1:0`, and the rest read files only.
- **Line endings:** all 53 changed files are LF in the index and in the worktree.
- **Every refusal names its reason:**
  - Each 4xx and 5xx carries a sentence.
  - The page says the server's sentence, or "Prices could not be loaded" with the cause.
  - The one mislabel: a server answer with empty `lines` reads as "Update Knowlu". The T4 review accepted
    that.
- **Cloud calls are transport only:** no model is called anywhere in the lane.
- **Contract-list file:** `app/src/account.rs` was reviewed by `contract-reviewer` at T4a. Every caller
  agrees with its interface (§2.1, §2.2).
- **Command lists:** `checkout_offer` is in both `generate_handler!` lists, and `ui_shell` pins 30 and 72.

## 7. The lane's docs against the code
- **`cloud/supabase/README.md`, the Billing section:** correct. It covers the keys, the catalog and its
  derived names, the offer, the four 409s, the price check, the six event types, the move with its trial
  rule, deploying all three together, and a pointer to the runbook.
- **`docs/reference/app.md`:** correct.
  - The release section's `site.yml` paragraph matches the workflow and the recovery rule.
  - The commands paragraph (72 / 30 / 91) matches `main.rs`.
  - `app/README.md`'s per-module counts add up to 91, with 11 names shared between the lists.
  - `app.md:324`'s "$9.99/month" is T6b's price line, which is still to come. Not a finding.
- **`docs/reference/price-change.md`:** M1 (a test a later task added), M2 and M3. Everything else matches
  the code:
  - the variable names, `MOVE_NOTICE_*` and the trial rule;
  - the exact 409 sentence;
  - `site.yml`'s guards and restore;
  - the recovery rule.
- **For T6b.** The terms tie no longer lives in `bundle_config`. No doc on this branch says it does, but
  note D11 and §6 still say `bundle_config`. Note D14 still says "[PENDING QUINN, §8 item 15]", which §8
  answers. Both belong to T6b's marker lines.

PASS WITH FIXES
