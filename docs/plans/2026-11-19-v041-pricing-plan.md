# v0.4.1 pricing: build plan

> **For agentic workers:** REQUIRED SUB-SKILL: use superpowers:subagent-driven-development (recommended)
> or superpowers:executing-plans to run this plan task by task. Steps use checkbox (`- [ ]`) syntax.

**Status:** r1, for review together with the note (one review and one re-review, reports in
`docs/reports/`). **Size:** Contract (money; `TOS_VERSION`, and with D12 `PRIVACY_VERSION`, in
`app/src/account.rs`; a release; the terms; with D12, the privacy page).
**Spec:** `docs/specs/2026-11-19-v041-pricing-design.md` r1, "the note" below, cited by section (§n) and
decision (D1-D12, P1, Q1-Q5). Where this plan and the note disagree, the note wins and the plan is wrong.
§11 lists the places where the note left room, and how this plan reads them.

**Goal.** Ship v0.4.1. Billing charges the signed prices: $12.99 every 4 weeks, or $84.37 once a year,
with 14 days free once per account and a reminder 3 days before the first charge. Managed Payments is in
if S1 passes. The app labels the plans, and the terms state them. CI publishes it all on a `v0.4.1` tag,
after Quinn's word.

**Architecture.** One integration branch, `feat/v0.4.1-pricing`, cut from `main` after v0.4.0 has merged.
- **Three cloud functions change:** `billing-checkout`, `stripe-webhook` and `billing-jobs`. No migration
  and no new function.
- **Three app scripts change their labels.**
- **One legal commit** moves the terms, `TOS_VERSION` and checkout's terms floor together. With D12 it also
  moves the privacy page, `PRIVACY_VERSION` and the panel key.
- **Order.** The functions go to staging for S1 and the proof, then to production before the tag (note §4).

**Code read for this plan.** Beyond the note's citations:
- the full `billing-checkout`, `stripe-webhook`, `billing-jobs`, `billing-portal` and `entitlement`
  handlers and their `index.ts`;
- `_shared/stripe.ts` and its seven importers;
- `app/tests/ui_wizard.rs:579-616`, `app/tests/ui_settings.rs:53-94, 119-133`,
  `app/tests/ui_account_privacy.rs:224-243` and `app/tests/bundle_config.rs:134-157`;
- `scripts/ui-fixture/checks/t21.py:16, 261-273`, the DOM check that pins today's plan buttons;
- `app/src/account.rs:24-25, 210-217, 678-688, 1292-1301, 1611-1621`;
- `cloud/supabase/functions/account/handler.ts:203, 234`;
- `site/terms.html` and `site/subscribed.html`, whole;
- `site/privacy.html:37-38, 75-86`;
- `.github/workflows/release.yml:294-298`.

The code facts are from `feat/v0.4.0-ui` as checked out in the `v041-pricing` worktree on 2026-11-19. T0
re-checks them on `main` once v0.4.0 has merged.

## 1. VISION, the two rules, the contract list

- **VISION.** These hold:
  - one edition, an account required, no free tier (a trial is not a tier);
  - failures are visible: a refusal says why, and an unrecognised plan is counted in `billing-jobs`'
    `failed`;
  - "Knowlu never sends anything to anyone else": the reminder goes only to the account's own address;
  - no telemetry is added.

  VISION's "One edition" price line is stale. Rewriting it is Quinn's words, and it is precondition P1.
- **Rule 1, no single-user assumption.** Price ids are read from each project's environment and never
  written into code. No account id, address or person appears in code, tests, fixtures or docs. Test data
  uses `example.invalid` addresses and invented ids. The reminder's date rule (UTC−10) assumes no student's
  time zone.
- **Rule 2, no frozen reference regenerated.** Nothing under `engine/src/` or `engine/tests/fixtures/`
  changes. This lane edits `engine/tests/site.rs` (not a fixture, and not on the list) to add page pins.
- **The contract list.**
  - `app/src/account.rs` is on it. Only `contract-engineer` edits it (T5's single legal commit), and
    `contract-reviewer` reviews that commit.
  - Nothing else on the list is touched.
  - A task that finds it needs a listed file stops and reports (§9).
- **Commands.** No Tauri command is added, renamed or changed. `open_checkout` passes `plan` through as it
  does today (`account.rs:1611-1621`). `commands.rs` computes nothing.

## 2. Branches, worktrees, PR, red-first evidence

- **Integration branch:** `feat/v0.4.1-pricing`, in a new worktree `.claude/worktrees/v041-build`, cut by
  T0 from `main` after the v0.4.0 merge.
- **Task branches.** Each task works on `v041/tNN-<slug>` in its own worktree. It is squash-merged into the
  integration branch as one commit, with the message `tNN: <what>`, once it is green and its review has
  passed. **Merges are the main session's** (the v0.4.0 lean-execution precedent).
- **Red first.** Each task commits its failing tests first, and that commit is tagged locally
  `v041-red/tNN` until the `v0.4.1` tag. The squash body carries the red SHA and each red test's failure
  reason, copied from the ledger.
- **The ledger** is `.superpowers/sdd/2026-11-19-v041-pricing/progress.md`. It is local and git-ignored.
- **The PR.** One Contract PR, `feat/v0.4.1-pricing` → `main`, merged as a merge commit. The pre-tag gate
  runs on that merge SHA (§8 D).
- **Housekeeping.** After each merge, the main session runs the workspace gate on the integration branch
  and deletes the task worktree's `target/`.
- **This plan and the note** sit on `docs/v041-pricing` (cut from `feat/v0.4.0-ui`). They reach `main` with
  the v0.4.0 merge, or as a docs-only push, before T0 cuts the code branch.

## 3. Preconditions (Quinn's), and what T0 settles

| # | What | Owner | Blocks |
|---|---|---|---|
| P1 | Amendment 2026-11-10 on `main`. An amendment recording the 2026-11-17 rulings (U6 Pell dropped; Managed Payments with Link as seller; tax-exclusive prices). VISION's "One edition" line and CLAUDE.md's Direction line in Quinn's words (legal spec C26) | Quinn's words; the main session commits; `docs-keeper` adds the markers | T1 |
| P2 | Q1 (Managed Payments in v0.4.1) answered, or deferred to checkpoint B | Quinn | T5's Link text |
| P3 | **The read-only production checks**, run once at T0 on Quinn's word: `select count(*) from public.accounts`; `select plan, status, count(*) from public.entitlements group by 1, 2`; `select count(*) from public.comp_grants where ended_at is null and now() < ends_at`. Quinn also reads, in the live Dashboard, any subscription on an old price. The answers set D11's old-subscriber paragraph and whether C1 (T5b) joins the lane | the main session (read-only, through the production project's SQL path) and Quinn | T5 |
| P4 | Q2: Quinn's own old-price subscriptions cancelled in both modes, and a comp grant for that account, if Quinn takes the recommendation | Quinn | T5's old-subscriber paragraph |
| P5 | Q3: the mailing address for the terms' Notices clause, or the ruling "email only" | Quinn | T5's commit |
| P6 | Q4: whether the Stripe account is moved to KNOWLU LLC first | Quinn | the live-mode checklist (§8 C) |
| P7 | The test-mode half of the note's §4 checklist: two prices, Managed Payments, the `trial_will_end` event, the portal, the reminder setting noted | Quinn | S1 |

**T0r settles the Stripe facts (note §3).** For every row of the note's §3 table, the researcher opens the
page and quotes the exact lines into `docs/reports/2026-11-19-v041-managed-payments-facts.md`, with each
URL and the date read. It also answers each of the following:
- the exact Managed Payments parameter on a Checkout Session, and whether it needs a preview API version;
- whether Managed Payments refuses any of the form's parameters (`handler.ts:51-72`):
  - `billing_address_collection`, `consent_collection[terms_of_service]`, `allow_promotion_codes`;
  - `payment_method_collection`, `subscription_data[trial_period_days]`, `custom_text`;
  - an existing `customer`;
- whether subscriptions created under Managed Payments live in Knowlu's account, carry
  `subscription_data[metadata]`, and send `customer.subscription.*` events to the endpoint;
- when `customer.subscription.trial_will_end` fires, and whether moving a test subscription's trial end to
  within 3 days fires it at once;
- how Checkout displays `interval=week, interval_count=4`;
- whether the Customer Portal works for a Managed Payments subscription;
- what Link sends: the receipt, the trial notice and their timing;
- whether a refund returns the tax and the fee;
- whether Managed Payments accepts tax-exclusive prices.

Each answer is a quote or "not found". The researcher is read-only and writes only that report.

## 4. Tasks at a glance

Agents and efforts follow CLAUDE.md's table. Every `cloud-engineer`, `console-ui`, `test-writer`,
`mechanical` and `docs-keeper` diff goes to `reviewer`. T5's legal commit, which touches `account.rs`, goes
to `contract-reviewer`, and its page text also goes to `reviewer`.

| Task | Note | Owns | Agent (model, effort) | After | Review |
|---|---|---|---|---|---|
| T0 baseline | — | the ledger, the worktree | main session | P1 | — |
| T0r Stripe facts | §3, D7 | `docs/reports/2026-11-19-v041-managed-payments-facts.md` | `researcher` (Opus, high), read-only with web | — | checkpoint A (main session) |
| T1 checkout | D3, D4, D5, D7, D8 | `cloud/supabase/functions/billing-checkout/{handler,index,handler_test}.ts` | `cloud-engineer` (Opus, high) | T0, checkpoint A | `reviewer`, one dispatch with T2 and T3 |
| T2 webhook and reminder | D2, D6 | `cloud/supabase/functions/stripe-webhook/{handler,index,handler_test}.ts` | `cloud-engineer`, the same run as T1 | T0, checkpoint A | as T1 |
| T3 billing mails | D10 | `cloud/supabase/functions/billing-jobs/{handler,handler_test}.ts` | `cloud-engineer`, the same run | T0 | as T1 |
| S1 test-mode spike | D7 gate, D6 | none (the ledger and a short report) | main session, with Quinn at the test Dashboard | T1-T3 merged, P7 | checkpoint B (Quinn if a gate fails) |
| T4 app labels | D9 | `app/static/js/wizard.js`, `app/static/js/pages/settings.js`, `app/static/js/carried/account.js`, `app/tests/ui_wizard.rs`, `app/tests/ui_settings.rs`, `app/tests/ui_plans.rs` (new), `scripts/ui-fixture/checks/t21.py`, `scripts/ui-fixture/checks/t16.py` (one assertion), `scripts/ui-fixture/envelopes/t16-settings-answers.json` (one value) | `console-ui` (Opus, high), red pins first | T1-T3 merged (billing's list of plans; the webhook's table) | `reviewer` |
| T5 legal commit | D11, D12, D3's floor | red: `engine/tests/site.rs`, `app/tests/bundle_config.rs`, with D12 `app/tests/ui_account_privacy.rs`. Draft: `site/terms.html`, `site/subscribed.html`, with D12 `site/privacy.html` and `app/static/js/carried/reconsent.js`. Commit: those plus `app/src/account.rs` (two constants) and the one floor line in `billing-checkout/index.ts` | `test-writer` (red), then `console-ui` (draft HTML and panel lines), then `contract-engineer` (Opus, xhigh) makes the one commit | checkpoint B, P3-P5, T1 merged | `contract-reviewer` (the commit); `reviewer` (page text); Quinn reads the pages |
| T5b C1, only if P3 finds another account | D11 | as the legal spec's C1: `account.rs`, `cloud/supabase/functions/account/handler.ts` and its test, `carried/reconsent.js` | `contract-engineer`, `cloud-engineer`, `console-ui` | P3 | `contract-reviewer`, `reviewer` |
| T6 docs | all | `HANDOFF.md`, `docs/reference/app.md`, `cloud/supabase/README.md`; marker lines in the legal spec (§5) and the v0.4.0 note (§13) | `docs-keeper` (Sonnet, medium) | T5 | `reviewer` |
| T7 version 0.4.1 | — | `Cargo.toml`, `app/tauri.conf.json`, `Cargo.lock` | main session (three lines, one screen) | T5 | T8 |
| T8 whole-branch review | all | none | `reviewer` (Opus, high) | T4-T7 | — |
| T9 proof, PR, rollout, tag | §4 | none | main session; Quinn's word at each production step | T8 PASS | Quinn |

**What can run at once.**
- T0r runs beside T0.
- T1, T2 and T3 are one `cloud-engineer` run in one worktree, with three red-then-green pairs. Their files
  are disjoint, so they can split into three runs if time matters.
- T4 and T5's red dispatch can run at once after T1 merges.
- T6 and T7 run in parallel.

**What runs in order, and why.**
- `billing-checkout/index.ts`: T1 writes it, and T5 moves its one floor line.
- `app/src/account.rs`: T5, then T5b if it runs.
- `app/tests/ui_account_privacy.rs`: only T5 touches it.

## 5. Tasks

**Common to every task.** Brief the agent with four things:
- the goal;
- the note's decisions, extracted by heading (for example, "**D3.**" up to "**D4.**");
- this task block;
- the files it owns and nothing else, and §1's constraints.

Red first: commit the failing tests, and record each failure reason in the ledger.

**Acceptance.** For a cloud task: `deno test` over `cloud/supabase/functions/<fn>/` is green. Then
`deno check`, `deno lint` and `deno test` pass exactly as `ci.yml:153-172` runs them. That run grants
`--allow-read`, `--allow-net=127.0.0.1` and an `--allow-env` list with no `STRIPE_*` or `EMAIL_*` name, so a
test never reads a price id or an email setting from the environment. Those reach the handler through
`Deps`, and `index.ts` wiring is checked by reading its text. For an app or site task: `cargo test -p knowlu --test <file>` (or
`-p knowlu-engine --test site`) is green, then the workspace gate:
- `cargo build --workspace` and `cargo test --workspace`;
- 0 warnings except the accepted `.rsrc` line;
- `pwsh scripts/ci/eol-check.ps1`.

### T0. Baseline (main session)
- [ ] Check that v0.4.0 has merged to `main`. Cut `feat/v0.4.1-pricing` and the worktree. Record `main`'s
      head, `cargo test --workspace` (passed, failed, ignored, and CI's warning line) and the deno count.
- [ ] Check P1 on `main`: `docs/specs/2026-09-09-knowlu-cloud-design.md` carries "Amendment 2026-11-10" and
      the 2026-11-17 amendment. If either is missing, stop: T1 does not start.
- [ ] Re-confirm the code facts the tasks rely on, and correct this plan before any dispatch if one has
      moved:
      - `billing-checkout/handler.ts:14-16, 51-72, 95-107, 120-127`;
      - `index.ts:16-20`;
      - `stripe-webhook/handler.ts:64-67, 97-101, 120-125`;
      - `billing-jobs/handler.ts:69-75, 126-137, 86, 101`;
      - `wizard.js:32-36, 641-642`;
      - `settings.js:32, 184`;
      - `carried/account.js:137`;
      - `ui_wizard.rs:606-616`.
- [ ] Recount the files that import `_shared/stripe.ts`. There are seven functions today: `account`,
      `billing-checkout`, `billing-jobs`, `billing-portal`, `mail-canary`, `mail-process` and
      `stripe-webhook`.
- [ ] Run P3's three read-only queries on Quinn's word. Record the counts in the ledger, as numbers only,
      with no ids or addresses. Record Quinn's Dashboard answer.
      - **More than one account** → T5b joins the lane.
      - **No old-price subscription live, after P4** → T5 omits the old-subscriber paragraph.

### T0r. Stripe facts (`researcher`)
- [ ] Do §3's list of questions, writing only the report. Mark every answer as a quote with its URL, or
      as "not found".
- **Checkpoint A** (the main session reads the report):
  - It writes T1's exact Managed Payments form keys into T1's brief.
  - If Managed Payments cannot be turned on for this account (for example, it is waitlisted), Quinn rules
    on Q1 now, and T1 builds without Managed Payments.
  - If the report contradicts a decision in the note, the note is amended before T1. The re-review round
    then covers the amendment.

### T1. Checkout (`cloud-engineer`)
- **Red** (`billing-checkout/handler_test.ts`):
  - `the_signed_plans_reach_stripe_with_their_price`: `four_weekly` and `yearly` each send their own price
    id to the session, and the consent row carries 1299 or 8437.
  - `an_older_build_is_told_to_update`: each of `monthly`, `academic_year` and `terms_version` "2026-09-09"
    (below the floor) answers 409 with exactly D3's sentence. The fake records nothing: no customer, no
    consent row, no Stripe call.
  - `an_unknown_plan_is_still_400`: today's test, kept.
  - `the_trial_is_fourteen_days_on_a_first_subscription_only`: the form carries "14" when
    `hadSubscription` answers false, and no trial key when it answers true. `payment_method_collection` is
    "always" in both cases.
  - `the_price_is_checked_before_it_is_sold`: one case each for a wrong amount, a wrong currency, the
    interval `month`, a count of 1 on the 4-week plan, and `active: false`. Each answers 500, and no
    consent row or session follows.
  - `the_form_is_a_managed_payments_session`: the keys from checkpoint A are present, and
    `automatic_tax[enabled]` and `customer_update[address]` are absent. If checkpoint B or A rules Managed
    Payments out, this test instead pins today's tax keys.
  - `stripes_page_states_the_renewal_terms`: `custom_text[submit][message]` equals D8's sentence for each
    combination of plan and trial.
  - An update to `index.ts wires …` (the text-read precedent, `handler_test.ts:331-343`). The environment
    names are `STRIPE_PRICE_FOUR_WEEKLY` and `STRIPE_PRICE_YEARLY`. The cents map is
    `{ four_weekly: 1299, yearly: 8437 }`. The floor constant is declared once, as
    `CHECKOUT_TERMS_VERSION = "2026-09-10"`. `isTester` is unchanged.
  - Every existing test whose body sends `monthly` or `academic_year` moves to the new plans. The 18+ and
    tester tests keep their assertions.
- **Change.** `Plan` becomes `"four_weekly" | "yearly"`, and a separate list holds the two old names for
  the 409. `Deps` gains three things:
  - `hadSubscription(accountId)`, which reads `entitlements.stripe_subscription_id is not null` for the
    account through PostgREST;
  - `getPrice(id)`, a Stripe GET through `stripeGetFrom`;
  - `termsFloor`.

  The refusal order is: method, user, plan list, `terms_version` present, account, 18+, tester, the old
  names or the floor (409), the price check, the customer, the consent row, the session. The Managed
  Payments session call carries a different API version only if T0r says so, and only on that one call
  (note D7). `_shared/stripe.ts` is not edited. `successUrl` and `cancelUrl` are unchanged.
- **Stop rule.** If Managed Payments needs a change to `_shared/stripe.ts`, stop and report. Redeploying
  the seven functions is the main session's call.

### T2. Webhook and reminder (`cloud-engineer`)
- **Red** (`stripe-webhook/handler_test.ts`):
  - `the_plan_comes_from_the_price_id`: the four ids map to their four names. An unknown id gives `null`.
    A `week` × 4 price whose id is unknown also gives `null`, because the interval no longer decides.
    This replaces today's interval assertions (`:18-62`), and the period-end and paused assertions stay.
  - `trial_will_end_sends_one_reminder`, for a trialing subscription with a known price:
    - one `sendEmail` to the account's address;
    - the text names the plan's words, the amount ("$84.37 once a year" or "$12.99 every 4 weeks"), "plus
      sales tax where it applies", "in 3 days", the date computed in UTC−10, and "Manage subscription";
    - the event is recorded;
    - `writeEntitlement` is not called.
  - `no_reminder_for_a_subscription_that_will_not_charge`: `cancel_at_period_end: true`, or a status that is
    not `trialing`, sends nothing and still records the event.
  - `a_failed_reminder_is_retried_by_stripe`: `sendEmail` throws, the response is 502, and nothing is
    recorded.
  - `a_duplicate_trial_will_end_sends_nothing`: `seenEvent` answers true, and no mail is sent.
  - `the_date_is_never_later_than_the_students_own`: a pure helper. For trial ends at 09:00Z and at 02:00Z
    it names the date in UTC−10. One case asserts the date one day earlier than UTC.
  - `index.ts` text-read: it reads the four `STRIPE_PRICE_*` names and the three `EMAIL_*` names.
- **Change.** `entitlementFromSubscription(sub, prices)` takes the id-to-name table. Add a branch for
  `trial_will_end`, after the signature check, the duplicate check and the account-exists check. Its deps
  are `accountEmail(accountId)` and `sendEmail`, with the `billing-jobs/index.ts:26-36` shape. Add
  `customer.subscription.trial_will_end` to the handled types. Keep the module comment's "only writer of
  `entitlements`" true.

### T3. Billing mails (`cloud-engineer`)
- **Red** (`billing-jobs/handler_test.ts`):
  - `the_new_plans_are_never_paused`: `four_weekly` and `yearly` on 2027-06-02, with `paused: false`. There
    is no Stripe call, and the counts show `paused: 0`.
  - `the_new_plans_reminder_says_every_4_weeks_or_once_a_year`: the annual reminder text contains "$12.99
    every 4 weeks" (or "$84.37 once a year") and "plus sales tax where it applies", and no "per month".
  - `the_cancel_path_names_settings_and_membership`: both mails contain "Settings", "Membership" and
    "Manage subscription", and do not contain "gear".
  - The existing `monthly` and `academic_year` tests stay, including the "$9.99" pin at `:149` (P10).
- **Change.** `priceOf` gains the two new entries. The pause branch (`:126-137`) is unchanged. Replace the
  sentence "click the gear" in both mails.

### S1. Test-mode spike (main session, with Quinn at the test-mode Dashboard)
- Run note D7's gate, steps (a) to (e), and record the results, per the plan's §8 B.

### T4. App labels (`console-ui`)
- **Red:**
  - `ui_wizard::the_subscribe_buttons_send_the_signed_plans`, renamed from `…todays_plans_by_name`:
    - `id="wiz-sub-year">Yearly</button>` comes before `id="wiz-sub-month">Every 4 weeks</button>` in the
      panel;
    - the yearly button has class `ibtn` and the other `wbtn`;
    - the mapping reads `? "yearly" : "four_weekly"`;
    - `billing-checkout/handler.ts` contains `const PLANS: readonly Plan[] = ["four_weekly", "yearly"];`.
  - `ui_wizard::the_wizard_states_no_price_or_trial`: the banned list gains "12.99", "84.37", "6.49",
    "14-day", "14 days", "fourteen" and "/mo".
  - `ui_settings::membership_shows_no_price`: the banned list gains "84.37", "6.49" and "14 days".
  - New `ui_settings::plan_words_cover_every_plan_billing_can_write`: `PLAN_WORDS` carries
    `four_weekly: "Every 4 weeks"`, `yearly: "Yearly"`, `monthly: "Monthly"` and
    `academic_year: "Academic year"`.
  - New file `ui_plans.rs`:
    - `the_upgrade_sheet_buys_the_four_week_plan` (`carried/account.js` sends `{ plan: "four_weekly" }`);
    - `every_plan_the_app_sends_is_one_billing_sells` (every `plan:` literal in `wizard.js` and
      `carried/account.js` is in billing's list of plans);
    - `every_plan_the_webhook_can_name_has_a_word` (the four names in `stripe-webhook/index.ts`'s table are
      keys of `PLAN_WORDS`).
  - DOM: `t21.py:261-273` expects "Yearly" and "Every 4 weeks" and
    `open_checkout == [{"plan": "yearly"}, {"plan": "four_weekly"}]` in click order. Its docstring line
    `:16` changes to match. `t16.py::check_settings` (`:248-250`) gains one assertion: `#set-member`
    contains "Active, Every 4 weeks", which is `settings.js:185`'s format. `t16-settings-answers.json:10`'s
    `"plan": "monthly"` becomes `"four_weekly"`. The other envelopes that carry `"plan": "monthly"`
    (t16-t21) only pass data through, and none of their checks reads the plan word, so they stay.
- **Change.**
  - `wizard.js:34`: swap the order of the buttons and relabel them.
  - `wizard.js:642`: change the mapping.
  - `settings.js:32`: add the entry.
  - `carried/account.js:137`: change the plan.
  - The wizard's lede is unchanged.
- **DOM run.** T21's and T16's checks pass through the fixture runner (`scripts/ui-fixture/run.py`) at
  both sizes and in both themes.

### T5. The legal commit (`test-writer`, then `console-ui`, then `contract-engineer`)
The legal spec's D17 shape applies: one commit moves the constants and carries the pages, the panel key and
the floor, so the version ties never fail on the integration branch.

- **Red** (`test-writer`):
  - `engine/tests/site.rs::the_terms_state_the_signed_prices`: the note's §6 lists (present and absent).
    It also checks that "summer pause" appears only in "There is no summer pause." and, if T0 kept the
    paragraph, in the paragraph that starts "If you subscribed before".
  - `site.rs::the_terms_carry_the_interim_terms_clauses`: the legal spec §7 item 8's strings ("KNOWLU LLC",
    "Commonwealth of Kentucky", "Jefferson County", "Sales tax is added at checkout where it applies"), and
    no "Alabama".
  - `site.rs::the_terms_promise_no_code_this_release_lacks`: no "When you subscribe:", no "Before the
    yearly plan renews", no "A year for someone else", no "does not count as paid".
  - `site.rs::the_subscribed_page_names_fourteen_days`: "14 days", "first subscription", "Settings", and no
    "seven-day" or "gear".
  - `bundle_config::the_terms_version_constant_is_the_published_terms_date`: the shape of the privacy tie,
    on `site/terms.html`'s two date lines. In the same test, `billing-checkout/index.ts`'s
    `CHECKOUT_TERMS_VERSION` equals `TOS_VERSION`. This is green on arrival and is recorded as a guard.
  - **With D12:**
    - `site.rs::the_privacy_page_names_link`: the Link bullet's "seller of record", the subscription
      entry's "sold through Link", and no "Seven companies";
    - `ui_account_privacy::the_panels_changes_name_link`, which replaces
      `the_panels_changes_name_gmail_and_amazon`. A comment says the old test described the 2026-10-20
      change. If bump A has already merged, the new test asserts all three names.
- **Draft** (`console-ui`, handed to `contract-engineer` as files, never committed separately):
  - **`site/terms.html`**, composed exactly as note D11 lists, from the legal spec by heading:
    - *Interim terms with bump A* for the frame;
    - *The subscription*, *How to cancel* and *Refunds* from §4's full text;
    - a new *Emails about your subscription*, with two bullets.

    Markers and bracketed notes are never published. `[TERMS DATE]` is the commit's date, and
    `[MAILING ADDRESS]` is P5's answer.
  - **`site/subscribed.html`:** the new trial sentence.
  - **With D12:**
    - `site/privacy.html`: the legal spec §3's B texts for *Your subscription*, *Link* and *Delete*, and
      D3's company sentence;
    - `carried/reconsent.js`: one new `PRIVACY_CHANGES` key with two or three short lines, no typed year,
      naming Link.
- **Commit** (`contract-engineer`, xhigh). One commit makes four kinds of change:
  - it moves `TOS_VERSION` (`account.rs:24`), and with D12 `PRIVACY_VERSION` (`:25`);
  - it moves `CHECKOUT_TERMS_VERSION` in `billing-checkout/index.ts`;
  - it adds the pages and the panel key;
  - it turns the red pins green.

  Message: `t05: terms <date> state the signed prices[; privacy <date> names Link]`.
- **Stop rule.** If bump A's PR is open when T5 starts, the main session decides with the legal lane's
  owner which of them carries the privacy text (note D12). Only one privacy key may ship in v0.4.1.

### T5b. Terms re-acceptance (only if T0's count finds another account)
- Build the legal spec's C1 to its §7 item 10. A later `tos_version` is recorded once. An equal or older
  version gets the silent 200. The panel shows only on an exact `needs_accept === true`.
- Owners: `contract-engineer` for `account.rs`, `cloud-engineer` for `account/handler.ts` and its test,
  `console-ui` for the panel. It must land before T9's tag.

### T6. Docs (`docs-keeper`)
- **`HANDOFF.md` §1:** v0.4.1 is billing at the signed prices, and v0.5.0 keeps the rest of the billing
  lane (note §7).
- **`docs/reference/app.md:309-311`:** the price line follows P1's amendment.
- **`cloud/supabase/README.md`:** a short "Billing" section listing the plan keys (D1), the four
  `STRIPE_PRICE_*` names (D2), the webhook's six event types, the 409 for older builds (D3) and the price
  check (D4).
- **Marker lines:**
  - the legal spec §5's "Terms + Privacy B" row and the v0.4.0 note §13 item 1 each get "(the prices and
    the terms moved to v0.4.1: `docs/specs/2026-11-19-v041-pricing-design.md`)";
  - the legal spec's interim-terms block gets "(superseded by v0.4.1's terms; bump A carries privacy only)".

### T7. Version (main session)
- `Cargo.toml:9`, `app/tauri.conf.json:4` and `Cargo.lock` move to 0.4.1. Re-check these line numbers at
  T0.

### T8. Whole-branch review (`reviewer`)
- The reviewer reads the whole diff against the note and this plan's §6 ledger, row by row.
- It spot-checks red-first: each `v041-red/tNN` tag's tests fail at the tag and pass at the squash.
- It confirms no file outside each task's Owns column changed.
- The report goes to `docs/reports/2026-11-2x-v041-whole-branch-review.md`.

### T9. Proof, PR, rollout, tag (main session)
- §8, in order. Quinn's word is needed at every production step and for the tag.

## 6. Fidelity ledger

"Pin" is a deno or cargo test, written `file::name`. "S1" and "Proof" are §8's steps. **⚑** marks a row with
no automated test: it rests on S1, the proof, a review or Quinn.

| Note | Requirement | Task | Proof |
|---|---|---|---|
| §1 | the student journey as described | T1-T5 | Proof B1-B6 |
| D1 | new keys `four_weekly` and `yearly`; old keys never sold; no migration | T1, T2, T4 | `billing-checkout::the_signed_plans_reach_stripe_with_their_price`, `::an_older_build_is_told_to_update`; `ui_wizard::the_subscribe_buttons_send_the_signed_plans`; `ui_plans::every_plan_the_app_sends_is_one_billing_sells` |
| D1 | `academic_year` keeps its name and price | T2, T3 | `stripe-webhook::the_plan_comes_from_the_price_id` (the old id → `academic_year`); `billing-jobs` existing academic-year tests unchanged |
| D2 | plan from the price id; unknown → `null`; four env names | T2 | `stripe-webhook::the_plan_comes_from_the_price_id`; the `index.ts` text-read; S1 (a) |
| D2 | an unknown plan is a counted failure, never a guessed price | T3 | `billing-jobs` existing unrecognised-plan test (kept) |
| D3 | old names and old terms version → 409, nothing called first; unknown → 400 | T1 | `billing-checkout::an_older_build_is_told_to_update`, `::an_unknown_plan_is_still_400`; Proof B5 |
| D3 | the floor moves with the terms | T1, T5 | `bundle_config::the_terms_version_constant_is_the_published_terms_date` |
| D4 | the price check before the consent row | T1 | `billing-checkout::the_price_is_checked_before_it_is_sold`; S1 (a) reads both prices |
| D5 | 14 days on a first subscription only; card always | T1 | `billing-checkout::the_trial_is_fourteen_days_on_a_first_subscription_only`; S1 (b) |
| D6 | the reminder: when, what, the date rule, retry, no entitlement write | T2 | `stripe-webhook::trial_will_end_sends_one_reminder`, `::no_reminder_for_a_subscription_that_will_not_charge`, `::a_failed_reminder_is_retried_by_stripe`, `::a_duplicate_trial_will_end_sends_nothing`, `::the_date_is_never_later_than_the_students_own`; S1 (c) |
| D6 ⚑ | the event fires 3 days before the trial ends | T0r, S1 | the T0r quote; S1 (c) |
| D7 | Managed Payments form keys; forbidden keys absent | T1 | `billing-checkout::the_form_is_a_managed_payments_session` |
| D7 ⚑ | gates (a) to (e); what Link sends; the refund path | S1 | S1 (a)-(e), each recorded with Dashboard and DOM evidence |
| D7 | `_shared/stripe.ts` not edited | T1 | T8 checks the diff; T1's stop rule |
| D8 | the renewal sentence on Stripe's page | T1 | `billing-checkout::stripes_page_states_the_renewal_terms`; Proof B2 (the page shows it) |
| D9 | labels, order, values; plan words; the sheet's plan; A2 bans | T4 | `ui_wizard::the_subscribe_buttons_send_the_signed_plans`, `::the_wizard_states_no_price_or_trial`; `ui_settings::membership_shows_no_price`, `::plan_words_cover_every_plan_billing_can_write`; `ui_plans::the_upgrade_sheet_buys_the_four_week_plan`, `::every_plan_the_webhook_can_name_has_a_word`; DOM t21, t16 |
| D9 ⚑ | the portal has no plan switching | Quinn (checklist) | S1 (d) records the portal's options |
| D10 | new `priceOf`; no pause for the new plans; the cancel path wording | T3 | `billing-jobs::the_new_plans_are_never_paused`, `::the_new_plans_reminder_says_every_4_weeks_or_once_a_year`, `::the_cancel_path_names_settings_and_membership` |
| D11 | the terms text, composed as listed | T5 | `site::the_terms_state_the_signed_prices`, `::the_terms_carry_the_interim_terms_clauses`, `::the_terms_promise_no_code_this_release_lacks`; `reviewer` reads the text against D11; Quinn reads the page |
| D11 | `TOS_VERSION` equals the terms date | T5 | `bundle_config::the_terms_version_constant_is_the_published_terms_date` |
| D11 | `subscribed.html` | T5 | `site::the_subscribed_page_names_fourteen_days` |
| D11 ⚑ | existing accounts: the count, then by-hand acceptance or C1 | T0, T5b, T9 | the ledger's count; §8 C step 12; T5b's own tests if it runs |
| D12 | Link on the privacy page; one key; the tie | T5 | `site::the_privacy_page_names_link`; `ui_account_privacy::the_panels_changes_name_link`, `::the_panels_changes_describe_this_privacy_version`; `bundle_config::the_privacy_version_constant_is_the_published_pages_date` |
| D12 ⚑ | re-consent shows and records the new version | T9 | Proof B6 |
| §4 ⚑ | rollout order: functions to production before the tag | T9 | §8 C, each step's record in the ledger |
| §5 | the gate, testers, the 18+ order unchanged | T1 | existing `billing-checkout` 18+ and tester tests, unchanged; `_shared/entitlement_test.ts` untouched |
| §5 | the webhook's signature, duplicate, stale and deleted-account guards | T2 | existing `stripe-webhook` tests, unchanged |
| §5 | P10: the old pause runs for `monthly` alone | T3 | existing pause tests; `billing-jobs::the_new_plans_are_never_paused` |
| §5 | the wording rule everywhere Knowlu writes a price | T1, T3, T5 | the `custom_text`, `priceOf` and terms pins (each bans "per month" or "a month") |
| §5 ⚑ | no single-user assumption; no frozen reference touched | all | T8 (the diff touches no `engine/src` or fixture; no id or address in any file) |

## 7. Review checkpoints

| Checkpoint | When | Who | Passes when |
|---|---|---|---|
| Note and plan review | before T0 | `reviewer`, plan mode, then one re-review; reports `docs/reports/2026-11-19-v041-pricing-review-r1.md`, `-r2.md` | PASS, or Quinn's short list of decisions after r2 |
| **A** | after T0r | main session | every §3 row answered; T1's form keys written into its brief; the note amended if a fact contradicts it |
| Cloud review | T1-T3 green | `reviewer` (one dispatch) | PASS; red-first visible in the history |
| **B** | after S1 | main session; **Quinn** if any of D7's (a)-(e) failed | each gate recorded; Q1 settled; T5's D12 scope known |
| App review | T4 green | `reviewer` | PASS |
| Legal review | T5 committed | `contract-reviewer` (the commit), `reviewer` (page text, batched with T6's review) | PASS; **Quinn reads the terms (and the privacy page)** |
| **C**, whole branch | T4-T7 merged | `reviewer` (T8) | PASS |
| **D**, pre-tag | after merge | main session, then **Quinn's word** | §8 D all recorded |

## 8. S1, the live proof, production rollout, pre-tag gate (main session)

No subagent holds a session or a token. Sign-in uses an emailed code on staging: the OTP is read through
the Gmail connector, at most two an hour. Desktop safety holds throughout: the DOM only, through CDP, with
no synthetic keyboard or mouse input and screenshots by `PrintWindow`. Scratch profiles and scratch vaults
only (`scripts\scratch-vault.ps1`). The ledger records each step's evidence: counts and statuses, never an
id, an address or a token.

**A. Staging setup**
- [ ] Quinn has done P7, the test-mode checklist.
- [ ] The main session sets `STRIPE_PRICE_FOUR_WEEKLY` and `STRIPE_PRICE_YEARLY` on staging. They are
      public values, set through `supabase secrets set` as the C1 plan did
      (`docs/plans/2026-09-09-c1-accounts-plan.md:2207-2209`).
- [ ] The main session deploys `billing-checkout`, `stripe-webhook` and `billing-jobs` to staging from the
      integration branch (`--use-api`, `--project-ref <staging>`). After T5 moves the floor, it redeploys
      `billing-checkout` before the proof.

**B. S1, then the proof**
- [ ] **S1 (a) and (b).** Use a fresh scratch staging account with no earlier subscription. POST
      `/billing-checkout` with `yearly` and with `four_weekly`, and load each returned URL in a headless
      browser in the session scratchpad. Read the page text: the price, "every 4 weeks" or the yearly
      wording, "14 days", the tax line, D8's sentence, the test-mode banner, and "Sold through Link" if
      Managed Payments is on. Quinn completes one test-mode checkout with Stripe's test card; no money
      moves. Then, read-only on staging:
      - `entitlements` shows `plan = 'four_weekly'`, `status = 'trialing'` and a `current_period_end` about
        14 days out;
      - the consent row carries 1299 and the new terms version.
- [ ] **S1 (c).** Quinn moves that test subscription's trial end to two days out in the Dashboard (the
      trigger T0r confirmed). The reminder arrives in the scratch inbox, read through the connector. The
      main session records its wording against D6.
- [ ] **S1 (d).** Through the app (proof B4), Manage subscription opens the portal or Link's page. One click
      cancels, and `entitlements` follows. The portal shows no plan switch.
- [ ] **S1 (e).** A second scratch account subscribes in test mode, then is deleted through Settings →
      Delete my data. Its subscription is cancelled, and the webhook ignores the later events (no such
      account).
- [ ] **S1 record.** Write `docs/reports/2026-11-2x-v041-s1-managed-payments.md` covering:
      - which emails Link sent, and when;
      - the receipt's sender;
      - whether a test refund, made by Quinn, returned the tax;
      - what the portal offers.

      Then go to checkpoint B.
- [ ] **Proof B1.** Build the dev app from the PR head, copy it to the scratchpad, and launch it with the
      staging environment and CDP on a new scratch profile.
- [ ] **Proof B2.** Run the wizard: sign in, then reach the Subscribe step.
      - DOM: "Yearly" comes before "Every 4 weeks". No "$", no "trial" and no "14" appear in the panel's
        text.
      - Click Yearly, then Every 4 weeks. `open_checkout` opens Stripe's test page in the browser. Take a
        `PrintWindow` shot of the browser window: $84.37, then $12.99 every 4 weeks, the test-mode banner,
        and D8's sentence.
      - Do not pay. Close the browser tab.
- [ ] **Proof B3.** Finish the wizard on the scratch vault, then go to Home. Nothing else in the journey
      changed.
- [ ] **Proof B4.** Use the S1 account, which has a trialing subscription, signed in on a second scratch
      profile. Settings → Membership reads "Active, Every 4 weeks", and Manage subscription opens the
      portal (S1 (d)).
- [ ] **Proof B5.** POST `/billing-checkout` with `{"plan":"monthly","terms_version":"2026-09-10"}` and the
      scratch token. It answers 409 with D3's sentence, and no consent row is added.
- [ ] **Proof B6 (D12 only).** Stage an older recorded privacy version for the scratch account. The panel
      shows the Link lines. Accept, and the server records the new version.
- [ ] **Clean up.** Remove the scratch profiles, vaults, credentials (`knowlu/<profile_id>/…`) and autostart
      entries. Delete the scratch staging accounts, which also cancels their test subscriptions.
- [ ] **The PR body** says what was run and what was seen, step by step.

**C. Production (each numbered step on Quinn's word; the note's §4 steps 7-13)**
- [ ] **7.** Re-run P3's read-only queries. Each answer must equal T0's, or the main session tells Quinn
      before going on.
- [ ] **8.** The PR merges with CI green, T8 passed and Quinn's reading done. Record the merge SHA.
- [ ] **9.** Quinn does the live-mode checklist (note §4) and sends the two live price ids.
- [ ] **10.** The main session sets the two production price ids, then deploys the three functions to
      production from the merge SHA. Check by POSTing `/billing-checkout` with `monthly`: 409.
- [ ] **11.** Go through §8 D, then tag `v0.4.1` on that SHA and push the tag. Watch `release.yml` through
      the Pages deploy (`release.yml:294-298`).
- [ ] **12.** If T0 found only Quinn's account, record Quinn's acceptance of the new terms version by hand,
      after Quinn accepts in writing. Use the legal spec §4's insert into `public.consents` and the update
      of `tos_version` and `tos_accepted_at`, as one production write on Quinn's go. If T5b ran, this step
      does not apply.
- [ ] **13.** Check that knowlu.com/terms.html carries the new version line and `subscribed.html` says 14
      days. Quinn's own account opens a live checkout page, and the main session reads the page (headless,
      as in S1 (a)). Nothing is paid. The updater offers 0.4.1 to a v0.4.0 install.

**D. Pre-tag gate (before step 11)**
- [ ] `git rev-parse <merge>^{tree}` equals the tree that was proved, or the proof restarts.
- [ ] `gh run list --workflow ci.yml --commit <merge>` shows success with the `warnings: N accepted
      (.rsrc), 0 tallies, 0 other` line.
- [ ] `pwsh scripts\release.ps1 -DryRun` passes on a clean checkout of that SHA.
- [ ] `pwsh scripts\ci\privacy-guard.ps1` passes.
- [ ] The production functions from step 10 are live: `supabase functions list --project-ref <prod>` shows
      the three updated after the merge.
- [ ] Ask Quinn, with these records.

## 9. Risks and stop rules

| Risk | Sign | What happens |
|---|---|---|
| Managed Payments is not available to this account, or needs a preview API on more than the session call | T0r, checkpoint A | Quinn rules on Q1 at A. The recommendation is to build without it (note D7's fallback), with no Link text and no D12. |
| Managed Payments needs `_shared/stripe.ts` changed | T1 | Stop. The main session decides, because seven functions bundle that file and each would be redeployed. |
| `trial_will_end` does not reach the endpoint in test mode | S1 (c) | Checkpoint B, with Quinn. The promise (P14) cannot ship without a reminder. The options are to hold the release, or to rely for one release on Link's or Stripe's own notice, if S1 showed one arrives 3 days ahead (`unverified (Stripe)`). The recommendation is to hold: a missing reminder makes the first charge refundable under the terms. |
| The portal cannot cancel a Managed Payments subscription in one click | S1 (d) | Checkpoint B. The terms' *How to cancel* is a promise, so Managed Payments falls back (D7). |
| A price is entered wrongly in the Dashboard | D4's check answers 500 on staging or at step 10 | Quinn corrects the price, and a new id is set. Nothing was sold. |
| The production count changed between T0 and step 7 | §8 C step 7 | Stop, and tell Quinn. If a new account appeared, T5b becomes required before the tag. |
| Bump A is open or merging at the same time | T5 start | T5's stop rule: one privacy key per build, decided before the commit. |
| A task needs a file on the contract list beyond `account.rs` | any | Stop and report. Only `contract-engineer` edits it, after the plan is amended. |
| The window between step 10 and the tag: v0.4.0 is refused, and v0.4.1 is not out yet | by design | Accepted. Outside sales are closed (Amendment 2026-10-16b), and the only v0.4.0 install is Quinn's. Keep the window short: step 11 follows step 10 the same day. |
| `release.yml` fails after step 10 | the tag's run | Fix forward on a new tag. If the release cannot ship that day, Quinn may roll the three functions back to the pre-merge SHA. Old functions with the old published terms are a coherent pair, so the rollback is safe. |
| A Stripe fact from the note proves false after it was built on | any later step | The note is amended. The change is re-reviewed inside the existing round, never a third round (CLAUDE.md, Workflow). |

**Two failed attempts** at a task means one level up, effort before model (CLAUDE.md). `max`, or Fable
for an implementing role, is Quinn's call.

## 10. Budget

- **Agent runs.** About eleven, plus fixes:
  - one `researcher` (T0r);
  - one or two `cloud-engineer` (T1-T3);
  - one `console-ui` (T4), and one for T5's drafts;
  - one `test-writer` (T5 red);
  - one `contract-engineer` (T5's commit);
  - one `docs-keeper` (T6);
  - `reviewer` four times: the cloud work, the app work, the page text with T6, and T8;
  - one `contract-reviewer`;
  - the note and plan review and its re-review (`reviewer`).

  T5b, if it runs, adds about three.
- **Fable** is used only by the planner, for this document and its revisions.
- **Quinn's time.**
  - The decisions in note §8.
  - The test-mode checklist, about 30 minutes.
  - S1, about an hour at the test-mode Dashboard, including the trial-end move and a test refund.
  - Reading the terms (and the privacy page).
  - The live-mode checklist, about 20 minutes.
  - Six production words: the T0 count, step 7, the merge, step 10, the tag and step 12.
- **Emailed codes on staging.** At most two an hour. S1 and the proof need three sign-ins:
  - account A, for S1 (a)-(d) and proof B4;
  - account B, which walks proof B2 to the Subscribe step before it subscribes for S1 (e);
  - Quinn's own, at step 13 in production.

  Plan S1 and the proof across two hours.
- **Disk.** At most four live builds. Delete each task worktree's `target/` after its merge.

## 11. Ambiguities, and how this plan reads them

1. **"Existing `academic_year` rows map to the yearly plan"** (Amendment 2026-11-10's consequences). Read
   with P10 and U10: the row keeps its name and its $69.99 price, and counts as yearly for the money-back
   guarantee. No row is rewritten (note D1).
2. **When billing moves.** The v0.4.0 note (A2, §13) and the legal spec §5 put billing in v0.5.0. Quinn's
   2026-11-19 ruling is later and moves the prices, the trial and the terms to v0.4.1. Everything else in
   the billing lane stays in v0.5.0 (note §7).
3. **A2 against P9.** The disclosure goes on Stripe's page (note D8), so the app keeps A2. If Managed
   Payments refuses `custom_text`, Quinn rules (D8).
4. **Bump A's interim terms against this lane's terms.** This lane's terms supersede them, and bump A
   carries privacy only (note D11). T6 marks the legal spec.
5. **The upgrade sheet's one plan.** It is `four_weekly`, following the 2026-09-14 ruling
   (`HANDOFF.md:80`). Ruling (2), which shows the yearly plan first, governs a choice between plans, and
   the sheet offers no choice. If Quinn wants the sheet to sell the yearly plan, the change is one value
   and one pin in T4.
6. **"Monthly" as a plan word.** The wording rule governs the words beside a price. Settings shows no
   price, and an old `monthly` subscription really is billed monthly, so its word stays (note D9).
7. **"One trial per account".** It means the account has no `entitlements` row with a subscription id.
   The deletion tombstone is not consulted, which is the known limit (note D5).
8. **The reminder's date.** It is written in UTC−10 with "in 3 days" beside it. No time zone is stored for
   the student, and the rule never names a date later than the student's own (note D6).
9. **The old-subscriber paragraph** in the terms appears only if T0's count and Quinn's Dashboard read
   find one (note D11). With P4 done, it is left out.
10. **The gift year** is its own lane (legal spec C24). These terms carry no gift paragraph, and T5's pin
    `the_terms_promise_no_code_this_release_lacks` holds that.
11. **The yearly price's 4-week equivalent ($6.49)** is never shown in the app (A2's pins ban it). Whether
    the site ever shows it is Q5's question, out of this lane.
