# v0.4.1 pricing: build plan

> **For agentic workers:** REQUIRED SUB-SKILL: use superpowers:subagent-driven-development (recommended)
> or superpowers:executing-plans to run this plan task by task. Steps use checkbox (`- [ ]`) syntax.

**Status:** r3, final. r2 answered review r1. r3 applies review r2's fixes
(`docs/reports/2026-11-19-v041-pricing-spec-review-r2.md`, PASS with changes). There is no further
review round; what stays open is the note's §8 list, for Quinn.

**Size:** Contract. It moves money; it moves `TOS_VERSION`, and with D12 `PRIVACY_VERSION`, in
`app/src/account.rs`; it is a release; it changes the terms; and with D12 it changes the privacy page.

**Spec:** `docs/specs/2026-11-19-v041-pricing-design.md` r2 ("the note"), cited by section (§n), by
decision (D1-D12) and by Quinn item: **Qn is item n of the note's §8 list** (the plan's own §8 is the proof and
rollout). Where this plan and the note disagree, the note wins. §11
lists where the note left room.

**Goal.** Ship v0.4.1 with three things true:
- **Billing charges the signed prices:** $12.99 every 4 weeks, or $84.37 once a year, with 14 days free
  once per account and a reminder before the first charge.
- **The app's subscribe step states those prices** (P9; Q5).
- **The terms state them.**

Managed Payments' per-session change is in if S1 passes. Every production step and the tag wait for
Quinn's word.

**Architecture.** One integration branch, `feat/v0.4.1-pricing`, cut from `main` after v0.4.0 has merged.
- **Cloud.** Three functions change (`billing-checkout`, `stripe-webhook`, `billing-jobs`). There is no
  migration and no new function.
- **App.** Two app scripts gain the plans and their prices (`wizard.js`, `carried/account.js`), and one
  gains a plan word (`settings.js`).
- **Legal.** One legal commit moves the terms, `TOS_VERSION` and checkout's terms floor together.
- **Order.** Production parity is checked first (note §4 step 0). The functions then go to staging for S1
  and the proof, and to production before the tag, with a pre-tag live proof between.

**Code read for this plan.** Beyond the note's citations:
- `billing-checkout/handler.ts:44-133`, whole;
- `stripe-webhook/handler.ts:1-12, 92-127`;
- `billing-jobs/handler.ts:1-20, 166-179`;
- `account/handler.ts:190-209`, `account/index.ts:74-101`;
- `_shared/stripe.ts` and its seven importers;
- `app/static/js/carried/account.js:76-143`, `app/static/js/pages/settings.js:174-187`;
- the tests: `app/tests/ui_wizard.rs:579-616`, `ui_settings.rs:53-94, 119-133`,
  `ui_account_privacy.rs:224-273`, `bundle_config.rs:134-157`;
- the DOM checks: `scripts/ui-fixture/checks/t21.py:16, 261-273`, `t16.py:225-254` and
  `scripts/ui-fixture/envelopes/t16-settings-answers.json:10`;
- `cloud/supabase/migrations/20261020000500_testers.sql`, `20261020000600_comp_guards.sql:1-25`;
- `.github/workflows/ci.yml:148-178`, `release.yml:294-298`;
- `HANDOFF.md:58-80, 166-174, 588-631`;
- the amendment, read in the `waitlist-spec` worktree (`:993-1232`).

These facts are from `feat/v0.4.0-ui` on 2026-11-19. T0 re-checks them on `main`.

## 1. VISION, the two rules, the contract list

- **VISION.** These hold:
  - one edition, an account required, no free tier (a trial is not a tier);
  - every refusal names its reason, and an unknown plan is counted in `billing-jobs`' `failed`;
  - the reminder goes only to the account's own address;
  - no telemetry is added.

  VISION's price line is Quinn's to rewrite (Q13), and that gates the tag.
- **Rule 1, no single-user assumption.**
  - Price ids come from each project's environment.
  - No account id, address or person appears in code, tests, fixtures, docs or reports.
  - Test data uses `example.invalid` addresses and invented ids.
  - The production steps that touch Quinn's account are one-off operations, recorded as counts.
- **Rule 2, no frozen reference regenerated.** Nothing under `engine/src/` or `engine/tests/fixtures/`
  changes. `engine/tests/site.rs` gains page pins; it is neither a fixture nor on the contract list.
- **The contract list.** Only `app/src/account.rs` is touched:
  - by `contract-engineer` in T5's one commit (and T5b's, if it runs);
  - reviewed by `contract-reviewer`.

  A task that needs any other file on the list stops (§9).
- **Commands.** No Tauri command changes. `open_checkout` passes `plan` through (`account.rs:1611-1621`).

## 2. Branches, worktrees, PR, red-first evidence

- **Before T0.** The note, this plan and the review reports sit on `docs/v041-pricing`, a separate branch.
  The main session merges it to `main`, or pushes it as a docs-only push, before T0 cuts the code branch.
  They do not ride the v0.4.0 merge.
- **Integration branch.** `feat/v0.4.1-pricing`, in the worktree `.claude/worktrees/v041-build`, cut by T0.
- **Task branches and merges.**
  - Each task has its own `v041/tNN-<slug>` worktree.
  - The task squash-merges into the integration branch as one commit (`tNN: <what>`), once green and
    reviewed. Merges are the main session's.
  - After each merge, the main session runs the workspace gate and deletes that worktree's `target/`.
- **Red first.**
  - The failing tests are committed first and tagged locally `v041-red/tNN`.
  - The squash body carries the red SHA and each failure reason, copied from the ledger.
- **The ledger.** `.superpowers/sdd/2026-11-19-v041-pricing/progress.md`, local and git-ignored.
- **The PR.** One Contract PR, `feat/v0.4.1-pricing` → `main`, merged as a merge commit. The pre-tag gate
  runs on that SHA.

## 3. Preconditions, and what T0 and T0r settle

**Quinn's items Q1-Q13, by what they block** (the note's §8 has each recommendation):

| Item | What | Blocks |
|---|---|---|
| Q1 | Amendment 2026-11-10 on `main`, plus an amendment recording the 2026-11-17 rulings and Q3, Q5, Q6, Q7 | T1 |
| Q2 | The production parity push, only if the testers and comp-guard stamps are the whole gap | Q9's grant; §8 C step 10 |
| Q3 | `academic_year` keeps its name and $69.99 | T2 |
| Q4 | Stripe and KNOWLU LLC: the same account's details, or a new account (deferred) | P7, the test-mode checklist, and so S1 |
| Q5 | The subscribe step shows P9's price lines | T4 |
| Q6 | The upgrade sheet sells both plans, Yearly first | T4 |
| Q7 | No gift paragraph in these terms | T5 |
| Q8 | The mailing address, or "email only" | T5 |
| Q9 | Per Quinn account that holds a subscription: comp grant, then cancel immediately | T5's old-subscriber paragraph; the act comes before §8 C step 7 |
| Q10 | Managed Payments in effect (reading (i) with the change, or (ii)), so Link on the pages; or the fallback | T5's scope (checkpoint B) |
| Q11 | The live reminder round trip | §8 C step 10b |
| Q12 | 10b's leftover production records | §8 C step 10b |
| Q13 | VISION's and CLAUDE.md's price lines in Quinn's words | the tag |

**Also Quinn's:** the test-mode half of the note's §4 checklist, before S1 (P7 below), after Q4.

**T0's production read** (read-only, on Quinn's word; counts only in the ledger, never an id or an
address):
- `supabase migration list --project-ref <prod>` and `supabase functions list --project-ref <prod>`.
  - If `20261020000500_testers.sql` or `20261020000600_comp_guards.sql` is missing, the main session opens
    precondition **"production parity for v0.4.1"** (Q2).
  - **Its scope.** The push runs from a **scratch copy of `cloud/supabase` whose `migrations/` ends at
    `20261020000600_comp_guards.sql`**, never from the integration worktree. `db push --include-all`
    pushes every pending local migration, and has no "through" option. L8's `entitlement` redeploy
    follows.
  - **If the list shows anything else missing below that stamp** that needs a secret, a Vault entry or a
    function redeploy beyond `entitlement` and `billing-checkout` (for example email's migrations), the
    precondition is L9 proper (`HANDOFF.md:598`). v0.4.1 waits for it, or Quinn rules.
  - The main session shows Quinn the exact list of files the push will apply before the go.
- **Whose accounts.** Quinn names which production accounts are Quinn's (`HANDOFF.md:58` records two). The
  ledger records two counts: Quinn's accounts and other accounts.
- The queries:
  - `select count(*) from public.accounts`;
  - `select plan, status, count(*) from public.entitlements group by 1, 2`;
  - if `public.comp_grants` exists: `select count(*) from public.comp_grants where ended_at is null and
    now() < ends_at`.
- Quinn reads any old-price subscription in the live Dashboard.

**T0r's questions** (the researcher writes `docs/reports/2026-11-19-v041-managed-payments-facts.md`, each
answer a quote with its URL and the date read, or "not found"):
1. **Every row of the note's §3.**
2. **The account setting.** What the account-level Managed Payments setting Quinn kept on in live mode on
   2026-09-14 is (`HANDOFF.md:80, 629`). Whether it applies to a session that does not ask for it, which
   settles D7's reading (i) or (ii).
3. **The session parameter.** The exact per-session parameter, and whether it needs a preview API version.
4. **What Managed Payments refuses.** Which form keys it refuses (`handler.ts:51-72`), with the
   load-bearing ones named first (note D7):
   - `consent_collection[terms_of_service]`, `payment_method_collection`,
     `subscription_data[trial_period_days]`;
   - `customer`, `subscription_data[metadata][account_id]`, `client_reference_id`, `custom_text`;
   - then the droppable ones.
5. **Subscriptions and events.** Whether its subscriptions live in Knowlu's account, keep their metadata and
   send `customer.subscription.*` events to the endpoint.
6. **The trial event.** When `trial_will_end` fires, and whether moving a trial end to within 3 days fires
   it at once, in test mode and in live mode.
7. **Display, portal and Link.**
   - How Checkout shows week × 4.
   - Whether the Customer Portal works for these subscriptions.
   - What Link sends, and when.
   - Whether a refund returns the tax and the fee.
   - Whether `tax_behavior: exclusive` is accepted.
8. **Moving Stripe to an LLC.** Whether a sole-proprietor Stripe account can become an LLC's by changing
   its business details, or needs a new account (Q4).

## 4. Tasks at a glance

Agents and efforts follow CLAUDE.md's table. Every `cloud-engineer`, `console-ui`, `test-writer` and
`docs-keeper` diff goes to `reviewer`. T5's commit goes to `contract-reviewer`, and its page text to
`reviewer`.

| Task | Note | Owns | Agent (model, effort) | After | Review |
|---|---|---|---|---|---|
| T0 baseline and production read | §4 step 0 | the ledger, the worktree | main session | §2's docs merge; Q1 | — |
| T0r Stripe facts | §3, D7 | `docs/reports/2026-11-19-v041-managed-payments-facts.md` | `researcher` (Opus, high), read-only, with web | — | checkpoint A |
| T1 checkout | D3, D4, D5, D7, D8 | `cloud/supabase/functions/billing-checkout/{handler,index,handler_test}.ts` | `cloud-engineer` (Opus, high) | T0, checkpoint A | `reviewer`, with T2 and T3 |
| T2 webhook and reminder | D2, D6 | `cloud/supabase/functions/stripe-webhook/{handler,index,handler_test}.ts` | `cloud-engineer`, same run | T0, checkpoint A, Q3 | as T1 |
| T3 billing mails | D10 | `cloud/supabase/functions/billing-jobs/{handler,handler_test}.ts` | `cloud-engineer`, same run | T0 | as T1 |
| S1 test-mode spike | D7 gate, D5, D6 | the ledger; `docs/reports/2026-11-2x-v041-s1-managed-payments.md` | main session, with Quinn at the test Dashboard | T1-T4 merged (S1 drives a T4 dev build); P7 | checkpoint B |
| T4 subscribe step and labels | D9 | `app/static/js/wizard.js`, `app/static/js/pages/settings.js`, `app/static/js/carried/account.js`, `app/tests/ui_wizard.rs`, `app/tests/ui_settings.rs`, `app/tests/ui_plans.rs` (new), `scripts/ui-fixture/checks/t21.py`, `scripts/ui-fixture/checks/t16.py` (one assertion), `scripts/ui-fixture/checks/t20.py` (one assertion; its `#up-subscribe` checks at `:94, 115` stay), `scripts/ui-fixture/envelopes/t16-settings-answers.json` (one value) | `console-ui` (Opus, high) | T1-T3 merged; Q5, Q6 | `reviewer` |
| T5 legal commit | D11, D12, D3's floor | red: `engine/tests/site.rs`, `app/tests/bundle_config.rs`, and with D12 `app/tests/ui_account_privacy.rs`. Draft: `site/terms.html`, `site/subscribed.html`, and with D12 `site/privacy.html` and `app/static/js/carried/reconsent.js`. The commit: those, plus `app/src/account.rs` (two constants) and the `CHECKOUT_TERMS_VERSION` line in `billing-checkout/index.ts` | `test-writer` (red), then `console-ui` (drafts), then `contract-engineer` (Opus, xhigh), who makes the one commit | checkpoint B; T0's count; Q7-Q9 | `contract-reviewer`; `reviewer` (text); Quinn reads the pages |
| T5b C1, only if T0 finds an account that is not Quinn's | D11 | the legal spec's C1: `account.rs`, `cloud/supabase/functions/account/{handler,handler_test}.ts`, `carried/reconsent.js` | `contract-engineer`, `cloud-engineer`, `console-ui` | T0 | `contract-reviewer`, `reviewer` |
| T6 docs | all | `HANDOFF.md`, `docs/reference/app.md`, `cloud/supabase/README.md`; marker lines in the legal spec and the v0.4.0 note | `docs-keeper` (Sonnet, medium) | T5 | `reviewer` |
| T7 version 0.4.1 | — | `Cargo.toml:9`, `app/tauri.conf.json:4`, `Cargo.lock` | main session | T5 | T8 |
| T8 whole-branch review | all | none | `reviewer` (Opus, high) | T4-T7 | — |
| T9 proof, PR, rollout, tag | §4 | none | main session, on Quinn's word at each production step | T8 PASS | Quinn |

**What can run at once.**
- T0r beside T0.
- T1-T3 as one `cloud-engineer` run, with disjoint files.
- T4 and T5's red dispatch after T1-T3 merge.
- T6 beside T7.

**What runs in order.**
- T1-T4, then S1. S1 drives a T4 dev build, whose wizard can reach a checkout. The floor is still
  2026-09-10 then, which that build sends.
- `billing-checkout/index.ts`: T1, then T5's one line.
- `account.rs`: T5, then T5b.
- **`site/`:** the controller's site port (branch `site/port-live-1119`) merges to `main` before the v0.4.0
  tag, so before T0. T5 composes its three pages on the ported `site/`. If the port has not landed when T5
  starts, stop: the port and T5 both write `site/*.html`.

## 5. Tasks

**Every brief carries:**
- the goal;
- the note's decisions, extracted by heading (for example, from "**D3.**" to "**D4.**");
- this task block;
- the owned files;
- §1's constraints.

Red first: commit the failing tests and record each failure reason.

**Acceptance.**
- **Cloud.** `deno test` over the function's folder, then `deno check`, `deno lint` and `deno test` exactly
  as `ci.yml:153-172` runs them. That run's `--allow-env` list has no `STRIPE_*` or `EMAIL_*` name, so those
  values reach the handler through `Deps`, and the `index.ts` wiring is checked by reading its text.
- **App and site.**
  - `cargo test -p knowlu --test <file>` (or `-p knowlu-engine --test site`);
  - then `cargo build --workspace` and `cargo test --workspace`, with 0 warnings except the `.rsrc` line;
  - then `pwsh scripts/ci/eol-check.ps1`.

### T0. Baseline and production read (main session)
- [ ] Confirm §2's docs merge, and that v0.4.0 has merged. Cut the branch and its worktree. Record:
      - `main`'s head;
      - the workspace counts and CI's warning line;
      - the deno count.
- [ ] Confirm Q1 on `main`: the cloud design carries Amendment 2026-11-10 and the 2026-11-17
      amendment. If not, stop: T1 does not start.
- [ ] Re-confirm each `path:line` the briefs rely on (the "Code read" list). Fix this plan first if one has
      moved.
- [ ] §3's production read, on Quinn's word. Record:
      - which of the two migrations production has, and the dates of its three billing functions;
      - the counts.

      The outcomes:
      - **A migration missing** opens "production parity for v0.4.1" (Q2), scoped as §3 says.
      - **Any account that is not Quinn's** adds T5b. Quinn's own accounts (`HANDOFF.md:58`) never do.
      - **No old-price subscription after Q9** means T5 leaves out the old-subscriber paragraph.

### T0r. Stripe facts (`researcher`)
- [ ] §3's eight questions. The researcher writes only the report.
- **Checkpoint A** (the main session reads the report):
  - Write into T1's brief the exact per-session parameter and the keys to drop. Only
    `automatic_tax[enabled]`, `customer_update[address]`, `billing_address_collection` and
    `allow_promotion_codes` may be dropped, each on a quoted refusal.
  - **If a load-bearing key is refused** (note D7), stop and go to Quinn at once. That is a checkpoint B
    failure brought forward (Q10). T1 builds the fallback form unless Quinn rules otherwise.
  - **If reading (ii) holds**, T1 makes no per-session change. Managed Payments is still in effect, so
    T5 carries D11's Link line and D12. The only fallback left is Quinn turning the account setting off in
    live mode before §8 C step 10 (Q10). Not recommended: that reverses R2 and U5.
  - **If the report contradicts the note**, amend the note within the open review round.

### T1. Checkout (`cloud-engineer`)

**Red** (`billing-checkout/handler_test.ts`):
- `the_signed_plans_reach_stripe_with_their_price`: each plan sends its own price id, and the consent row
  carries 1299 or 8437.
- `an_older_build_is_told_to_update`: `monthly` and `academic_year` answer 409 with D3's exact sentence.
  After the session check, the fake records no call: no account read, no tester read, no customer, no
  consent row, no session.
- `a_terms_version_below_the_floor_is_told_to_update`: "2026-09-09" answers 409 with the same sentence.
- `a_malformed_or_future_terms_version_is_refused`: each answers 400 with "a version must be a YYYY-MM-DD
  date no later than tomorrow":
  - "zzz" and "9999-99-99";
  - the day after tomorrow;
  - **"2026-11-19x"**, which sorts between the floor and tomorrow and is caught only by the pattern.
- `an_unknown_plan_is_still_400`.
- `the_trial_is_fourteen_days_on_a_first_subscription_only`:
  - no row gives "14";
  - a `canceled` row with a subscription id gives no trial key;
  - `payment_method_collection` is "always" in both cases.
- `an_account_with_a_live_subscription_is_refused`: a row with a subscription id whose status is `active`,
  `trialing`, `past_due` or `unpaid` answers 409 with D5's sentence before any customer or consent row.
- `the_price_is_checked_before_it_is_sold`: one case per check, each answering 500 with no consent row or
  session:
  - a wrong amount;
  - a wrong currency;
  - `active: false`;
  - `tax_behavior: "inclusive"`;
  - the interval `month`;
  - a count of 1 on the 4-week plan;
  - **`four_weekly` pointing at a `year` × 1 price at 1299**, and `yearly` pointing at a `week` × 4 price
    at 8437.
- `the_form_keeps_every_load_bearing_key`: D7's load-bearing keys are present (the trial key only on a
  first subscription, D5). The dropped keys equal checkpoint A's list exactly. If the fallback applies,
  today's tax keys are pinned instead.
- `stripes_page_states_the_renewal_terms`: D8's sentence, for each plan and trial combination.
- The `index.ts` text-read (`handler_test.ts:331-343`) checks:
  - the names `STRIPE_PRICE_FOUR_WEEKLY` and `STRIPE_PRICE_YEARLY`;
  - `{ four_weekly: 1299, yearly: 8437 }`;
  - `CHECKOUT_TERMS_VERSION` declared once, as a quoted `YYYY-MM-DD` literal. It does not check the
    value: `bundle_config` ties that to `TOS_VERSION`, so T5's commit changes no test file of T1's;
  - `isTester` unchanged.
- Existing tests move to the new plans. The 18+ and tester assertions are unchanged.

**Change.**
- `Plan` becomes `"four_weekly" | "yearly"`, with a separate list of the two old names.
- `Deps` gains `subscriptionState(accountId)`, which returns `stripe_subscription_id` and `status`, plus
  `getPrice(id)` (through `stripeGetFrom`) and `termsFloor`.
- The order is: method, user, parse, D3's three checks, account, 18+, tester, D5, D4, customer, consent,
  session.
- D3's shape check is the pattern `^\d{4}-\d{2}-\d{2}$` first, then the comparison with tomorrow (UTC).
  The account function has only the comparison (`account/handler.ts:199-201`); it is not copied.
- D4 compares the interval and the count to the plan's own pair: `four_weekly` is week × 4, `yearly` is
  year × 1.
- A different API version, if T0r requires one, goes on the session call only. `_shared/stripe.ts` is not
  edited.

**Stop rule.** If `_shared/stripe.ts` must change, stop: seven functions would redeploy.

### T2. Webhook and reminder (`cloud-engineer`)

**Red** (`stripe-webhook/handler_test.ts`):
- `the_plan_comes_from_the_price_id`:
  - the four ids map to their names;
  - an unknown id gives `null`;
  - a week × 4 price with an unknown id gives `null`.

  This replaces `:18-62`'s interval cases. The period and paused assertions stay.
- `a_missing_price_setting_matches_nothing`: a table with an absent old id still maps the others, and no
  event answers 500.
- `trial_will_end_sends_one_reminder`, for a trialing subscription with a known price and a future
  `trial_end`:
  - one `sendEmail` to the account's address;
  - the text has the plan words, the amount computed from the event's `unit_amount` and `recurring`
    ("$84.37 once a year" or "$12.99 every 4 weeks"), "plus tax where it applies", the UTC−10 date and
    "Manage subscription";
  - no "in 3 days" and no "per month";
  - the event is recorded, and `writeEntitlement` is not called.
- `an_unknown_price_still_gets_its_reminder`: no plan words, and the amount and interval from the event.
- `no_reminder_for_a_subscription_that_will_not_charge`: `cancel_at_period_end`, a status other than
  `trialing`, or a past `trial_end` each send nothing and record the event.
- `a_failed_reminder_is_retried_by_stripe`:
  - 502, and nothing recorded;
  - the captured `console.error` line holds the account id and no "@".
- `a_duplicate_trial_will_end_sends_nothing`.
- `the_date_is_never_later_than_the_students_own`: a pure helper, with a case one day before the UTC date.
- `index.ts` text-read: the four `STRIPE_PRICE_*` names, read as optional, and the three `EMAIL_*` names.

**Change.**
- `entitlementFromSubscription(sub, prices)`.
- A `trial_will_end` branch after the signature, duplicate and account checks, with deps `accountEmail`
  and `sendEmail` (the shape of `billing-jobs/index.ts:26-36`).
- The handled event types gain `trial_will_end`.

### T3. Billing mails (`cloud-engineer`)

**Red** (`billing-jobs/handler_test.ts`):
- `the_new_plans_are_never_paused` (2027-06-02, no Stripe call, `paused: 0`).
- `the_new_plans_reminder_says_every_4_weeks_or_once_a_year`: the wording includes "plus tax where it
  applies" and never "per month".
- `the_cancel_path_names_settings_and_membership`: "Settings", "Membership" and "Manage subscription"
  present, and no "gear".
- The existing `monthly` and `academic_year` tests stay, including `:149`.

**Change.** `priceOf` gains two entries. "click the gear" is replaced in both mails. The pause branch is
unchanged.

### S1. Test-mode spike (main session; §8 B; after T1-T4)

### T4. Subscribe step and labels (`console-ui`; built as Q5 and Q6 rule)

**Red:**
- `ui_wizard::the_subscribe_buttons_send_the_signed_plans`:
  - `#wiz-sub-year` ("Yearly", `ibtn`) comes before `#wiz-sub-month` ("Every 4 weeks", `wbtn`);
  - the mapping is `? "yearly" : "four_weekly"`;
  - billing has `const PLANS: readonly Plan[] = ["four_weekly", "yearly"];`.
- `ui_wizard::the_wizard_states_the_signed_prices_and_no_trial`, which replaces
  `the_wizard_states_no_price_or_trial`.
  - The subscribe panel's markup contains, verbatim:
    - "$6.49 every 4 weeks, billed $84.37 once a year. Renews yearly.";
    - "30-day money-back guarantee on each yearly charge.";
    - "$12.99 every 4 weeks.";
    - "Plus tax where it applies.".
  - The test keeps today's method (`ui_wizard.rs:585-597`): `string_literals` over **both** `js/wizard.js`
    and `js/picker.js`. The status word `"trialing"` (`wizard.js:162`) is the one exception to the trial
    ban. For every literal:
    - "$" and "a year" appear only inside one of the four lines above;
    - none of these appears: "trial", "7-day", "seven day", "14-day", "14 days", "fourteen", "per month",
      "/month", "a month", "/mo", "per year", "9.99", "69.99", "free first" or "pell".
- `ui_settings::membership_shows_no_price` gains "84.37", "6.49" and "14 days".
- `ui_settings::plan_words_cover_every_plan_billing_can_write`: the four words.
- New file `ui_plans.rs`:
  - `the_upgrade_sheet_sells_both_plans_yearly_first`: `carried/account.js` sends `{ plan: "yearly" }` and
    `{ plan: "four_weekly" }`, with the yearly button first, inside `#up-subscribe`.
  - `the_sheet_states_the_same_prices`: the sheet carries the same four lines as the wizard.
  - `only_the_two_plan_buttons_start_checkout`: the sheet's click handler starts `subscribe` only from
    `#up-sub-year` or `#up-sub-month`, never from `closest("#up-subscribe")`. Today's handler
    (`carried/account.js:214`) would start a checkout with no plan on a click on the price lines.
  - `every_plan_the_app_sends_is_one_billing_sells`.
  - `every_plan_the_webhook_can_name_has_a_word`.
  - `the_shown_prices_are_the_charged_cents`: "$12.99" is 1299, "$84.37" is 8437 and "$6.49" is 8437 / 13,
    against `billing-checkout/index.ts`'s map.
- **The DOM checks:**
  - `t21.py`: `:16`'s docstring changes. `:264` expects the new labels and the price lines and no "trial".
    `:273` expects `[{"plan": "yearly"}, {"plan": "four_weekly"}]`.
  - `t16.py::check_settings`: "Active, Every 4 weeks" (`settings.js:185`), with
    `t16-settings-answers.json:10` set to `four_weekly`.
  - `t20.py`: one assertion that, once shown, the sheet offers Yearly before Every 4 weeks.

**Change.**
- `wizard.js:32-36` and `:642`: the panel, the lines, the order, the mapping. The lede drops "and shows the
  price".
- `settings.js:32`: the word.
- `carried/account.js:83, 137`: `#up-subscribe` becomes the wrapper that is hidden and shown, holding the
  two buttons (`#up-sub-year`, `#up-sub-month`) and the lines. Its id stays, so
  `ui_account_privacy.rs:66, 105, 333` and `t20.py:94, 115` stay green unchanged.
- `carried/account.js:214`: the handler binds the two buttons. `subscribe(plan)` takes the plan from the
  button pressed, and a click elsewhere in the wrapper does nothing.

**Stop rule.** If any `ui_account_privacy.rs` pin must change, stop. That file is T5's, and the main session
sequences the two.

**If Q5 is "price-free":**
- no price lines are built;
- the wizard test keeps r1's shape, banning "12.99", "84.37", "6.49", "14-day", "14 days", "fourteen" and
  "/mo";
- `ui_plans` drops the figure checks.

**If Q6 is "one plan":** the sheet keeps one button, sending `four_weekly`.

The DOM runs use `scripts/ui-fixture/run.py`, at both sizes and in both themes.

### T5. The legal commit (`test-writer`, then `console-ui`, then `contract-engineer`)

**Red** (`test-writer`):
- `site::the_terms_state_the_signed_prices`: the note §6 strings, present and absent. "summer pause"
  appears only in "There is no summer pause." (and in the old-subscriber paragraph, if T0 kept it).
- `site::the_terms_carry_the_interim_terms_clauses`: the legal spec §7 item 8's strings, and no "Alabama".
- `site::the_terms_promise_no_code_this_release_lacks`: none of "When you subscribe:", "Before the yearly
  plan renews", "A year for someone else" or "does not count as paid".
- `site::the_subscribed_page_names_fourteen_days`: "14 days", "first subscription", "Settings", and no
  "seven-day" or "gear".
- `bundle_config::the_terms_version_constant_is_the_published_terms_date`: the privacy tie's shape, plus
  `CHECKOUT_TERMS_VERSION == TOS_VERSION`. This is green on arrival and is recorded as a guard.
- **If Managed Payments is in effect** (reading (ii), or reading (i) with the change; checkpoint B):
  - the terms pin requires "Sold through Link";
  - `site::the_privacy_page_names_link`;
  - `ui_account_privacy::the_panels_changes_name_link`, replacing `…name_gmail_and_amazon` (or naming all
    three, if bump A has already merged).
- **Otherwise:** the terms pin bans "Link".

**Draft** (`console-ui`, handed over as files):
- `site/terms.html`, composed by heading as note D11 lists. Markers are never published. `[MAILING
  ADDRESS]` takes Q8's answer.
- `site/subscribed.html`.
- If Managed Payments is in effect: `site/privacy.html` and one `PRIVACY_CHANGES` key in
  `carried/reconsent.js`.

**The terms date** is the commit's own date (UTC). It must be no later than the day after the staging
proof, because checkout's shape check (D3) and the production account function (`account/handler.ts:199-201`)
both refuse a version later than tomorrow.

**The commit** (`contract-engineer`, xhigh) moves, in one commit:
- `TOS_VERSION`, and `PRIVACY_VERSION` if Managed Payments is in effect;
- `CHECKOUT_TERMS_VERSION`;
- the pages and the key.

Message: `t05: terms <date> state the signed prices[; privacy <date> names Link]`. After it merges, the
main session redeploys `billing-checkout` to staging before the final staging proof (§8 B, part 2). The
commit touches no T1 test: T1's text-read checks only the constant's shape.

**Stop rule.** If bump A's PR is open, the main session decides which PR carries the privacy text, so that
only one key ships.

### T5b. Terms re-acceptance (only if T0 found an account that is not Quinn's)
- The legal spec's C1, to its §7 item 10. It must land before the tag. Production's `account` is redeployed
  at §8 C step 10 with the others.

### T6. Docs (`docs-keeper`)
- **`HANDOFF.md`:** v0.4.1 is billing at the signed prices; the rest stays in v0.5.0; production parity
  as found by T0.
- **`docs/reference/app.md:309-311`:** the price line, as worded in Q1's amendment.
- **`cloud/supabase/README.md`:** a "Billing" section covering the plan keys, the four optional
  `STRIPE_PRICE_*` names, the six handled event types, the 409s and the price check.
- **Marker lines** in the legal spec (§5's "Terms + Privacy B" row and the interim block) and in the v0.4.0
  note (A2's wizard clause, ended by this lane, and §13 item 1).

### T7. Version (main session)
- `Cargo.toml:9`, `app/tauri.conf.json:4`, `Cargo.lock`: 0.4.1.

### T8. Whole-branch review (`reviewer`)
- The diff against the note and §6, row by row.
- The red tags are spot-checked.
- No file outside each task's Owns column changed.
- The report: `docs/reports/2026-11-2x-v041-whole-branch-review.md`.

### T9. Proof, PR, rollout, tag (main session)
- §8, in order.

## 6. Fidelity ledger

"Pin" means a deno or cargo test, written `file::name`. **⚑** marks a row with no automated test, which
rests on S1, a proof, a review or Quinn.

| Note | Requirement | Task | Proof |
|---|---|---|---|
| §1 | the student journey | T1-T5 | §8 B proof steps, and §8 C step 10b |
| D1 | new keys; old keys never sold; no migration | T1, T2, T4 | `billing-checkout::the_signed_plans_reach_stripe_with_their_price`, `::an_older_build_is_told_to_update`; `ui_plans::every_plan_the_app_sends_is_one_billing_sells` |
| D1 ⚑ | `academic_year` keeps its name and price | T2, T3, Quinn | `stripe-webhook::the_plan_comes_from_the_price_id`; the existing `billing-jobs` tests; Q3 |
| D2 | plan from the price id; unknown gives `null`; values optional | T2 | `stripe-webhook::the_plan_comes_from_the_price_id`, `::a_missing_price_setting_matches_nothing`; the text-read; S1 (a) |
| D3 | old names, old version and bad shape refused first; unknown is 400 | T1 | `billing-checkout::an_older_build_is_told_to_update`, `::a_terms_version_below_the_floor_is_told_to_update`, `::a_malformed_or_future_terms_version_is_refused`, `::an_unknown_plan_is_still_400`; §8 C step 10b (body) |
| D3 | the floor moves with the terms | T1, T5 | `bundle_config::the_terms_version_constant_is_the_published_terms_date` |
| D4 | six price checks before the consent row, with the interval and the count tied to the plan | T1 | `billing-checkout::the_price_is_checked_before_it_is_sold` (including a `four_weekly` id on a `year` × 1 price) |
| D5 | the trial on a first subscription only; card always; one live subscription | T1 | `billing-checkout::the_trial_is_fourteen_days_on_a_first_subscription_only`, `::an_account_with_a_live_subscription_is_refused`; S1 (b) |
| D6 | the reminder's conditions, wording, unknown price, retry, log, no entitlement write | T2 | the six `stripe-webhook` reminder pins; S1 (c) |
| D6 ⚑ | the event fires, and the reminder arrives in live mode | T0r, T9 | T0r's quote; §8 C step 9 (events read back); step 10b (round trip, Q11) |
| D7 | load-bearing keys kept; only allowed keys dropped | T1, checkpoint A | `billing-checkout::the_form_keeps_every_load_bearing_key` |
| D7 ⚑ | reading (i) or (ii); the test-mode gates (a)-(e) | T0r, S1 | T0r's report; the S1 report |
| D7 | `_shared/stripe.ts` untouched | T1 | T8's diff check |
| D8 | the renewal sentence | T1 | `billing-checkout::stripes_page_states_the_renewal_terms`; S1 (a); step 10b |
| D9 | the wizard's prices, order and values; no trial words | T4 | `ui_wizard::the_subscribe_buttons_send_the_signed_plans`, `::the_wizard_states_the_signed_prices_and_no_trial`; DOM t21 |
| D9 | the shown figures are the charged cents | T4 | `ui_plans::the_shown_prices_are_the_charged_cents` |
| D9 | the sheet sells both plans with the same lines; only the buttons start checkout | T4 | `ui_plans::the_upgrade_sheet_sells_both_plans_yearly_first`, `::the_sheet_states_the_same_prices`, `::only_the_two_plan_buttons_start_checkout`; DOM t20 |
| D9 | Settings words, and no price | T4 | `ui_settings::plan_words_cover_every_plan_billing_can_write`, `::membership_shows_no_price`; `ui_plans::every_plan_the_webhook_can_name_has_a_word`; DOM t16 |
| D9 ⚑ | the portal has no switching | Quinn | the checklist; S1 (d) |
| D10 | `priceOf`; no pause for the new plans; the cancel path | T3 | the three new `billing-jobs` pins |
| D11 | the terms text; no gift paragraph | T5 | `site::the_terms_state_the_signed_prices`, `::the_terms_carry_the_interim_terms_clauses`, `::the_terms_promise_no_code_this_release_lacks`; `reviewer`; Quinn reads the terms |
| D11 | the version tie; `subscribed.html` | T5 | `bundle_config::the_terms_version_constant_is_the_published_terms_date`; `site::the_subscribed_page_names_fourteen_days` |
| D11 ⚑ | existing accounts | T0, T5b, T9 | the ledger's count; §8 C step 12, or T5b's tests |
| D12 | Link on the privacy page; one key; the tie | T5 | `site::the_privacy_page_names_link`; `ui_account_privacy::the_panels_changes_name_link`, `::the_panels_line_one_names_privacy_version_and_the_table_has_one_key`; `bundle_config::the_privacy_version_constant_is_the_published_pages_date` |
| §4 ⚑ | production parity before step 10 | T0, T9 | the ledger's migration list; Q2 done |
| §4 ⚑ | rollout order; the deploy proven live by its body and deliveries; 10b's card always cancelled | T9 | §8 C steps 7-13, each recorded; 10b's `canceled` read-back |
| §4 ⚑ | the tag publishes the ported site plus T5's pages | T9 | §8 D's `site/` line |
| §5 | 18+, tester and gate unchanged | T1 | the existing tests, unchanged |
| §5 | the webhook's guards | T2 | the existing tests, unchanged |
| §5 | P10's pause for `monthly` alone | T3 | the existing pause tests; `billing-jobs::the_new_plans_are_never_paused` |
| §5 | the wording rule | T1, T3, T4, T5 | the `custom_text`, `priceOf`, wizard and terms pins |
| §5 ⚑ | rule 1; rule 2; no ids in reports | all | T8; the S1 report's rule (§8 B) |

## 7. Review checkpoints

| Checkpoint | When | Who | Passes when |
|---|---|---|---|
| Note and plan review | done: r1 FAIL, r2 PASS with changes, r3 applies them | `reviewer`; the controller checks r3 against r2's list before T0; then Quinn | the note's §8 settled by Quinn |
| **A** | after T0r | main session | every question answered; T1's brief written; a refused load-bearing key goes to Quinn |
| Cloud review | T1-T3 green | `reviewer` | PASS, with red-first visible |
| **B** | after S1 | main session; **Quinn** if a gate failed | the gates recorded; Q10 settled |
| App review | T4 green | `reviewer` | PASS |
| Legal review | T5 committed | `contract-reviewer`; `reviewer` (text, with T6) | PASS; **Quinn reads the pages** |
| **C** | T4-T7 merged | `reviewer` (T8) | PASS |
| **D** | before the tag | main session, then **Quinn's word** | §8 D recorded, step 10b included |

## 8. S1, the staging proof, production, the pre-tag gate (main session)

**Rules for every step.**
- No subagent holds a session or a token. Staging sessions come by emailed code, at most two an hour.
- The DOM only, through CDP. No synthetic input. Screenshots by `PrintWindow`. Scratch profiles and vaults
  only.
- **The ledger and every report in `docs/reports/` hold counts, statuses and wording only.** That means no
  address, no customer or subscription id, no session id, and no receipt text that carries one (the
  repository is public).

**A. Staging setup**
- [ ] Q4 settled. Then P7: Quinn's test-mode checklist (note §4).
- [ ] Set `STRIPE_PRICE_FOUR_WEEKLY` and `STRIPE_PRICE_YEARLY` on staging.
- [ ] Deploy the three functions from the integration branch after T1-T4 (`--use-api`).
- [ ] **B1.** Build the dev app from the integration branch after T4 (staging environment, a new scratch
      profile). It sends `terms_version` 2026-09-10, which equals the floor until T5.

**B. Part 1: S1 and the staging proof, on the T4 build**
- [ ] **S1 (a) and (b), account A** (a fresh scratch account, signed in through the T4 build):
  1. Open `yearly` and `four_weekly` checkouts. A headless read shows:
     - the price, and "every 4 weeks" or the yearly wording;
     - "14 days", the tax line and D8's sentence;
     - "Sold through Link" if Managed Payments is in effect.
  2. Quinn completes `four_weekly` with a test card.
  3. Then, read-only:
     - `four_weekly`, `trialing`, a period end about 14 days out;
     - the consent row carries 1299 and the `terms_version` sent.
- [ ] **B4.** Account A in the T4 build: Settings reads "Active, Every 4 weeks". This comes before
      S1 (d)'s cancel.
- [ ] **S1 (c).** Quinn moves account A's trial end to two days out. The reminder arrives (through the
      connector) and its wording is recorded against D6.
- [ ] **S1 (d).** In the app, Manage subscription cancels in one click, `entitlements` follows, and the
      portal has no switch. Then open a new checkout on account A: the page shows **no** trial.
- [ ] **S1 (e), account B, with B2 and B3.**
  1. **B2.** Account B walks the T4 build's wizard to Subscribe:
     - Yearly comes before Every 4 weeks, with the four lines, and no "trial" or "14" in the panel;
     - clicking each button opens Stripe's test page, and `PrintWindow` shots show figures matching the
       panel.
  2. Quinn completes **`yearly`** from that page. The webhook writes `yearly`, `trialing`.
  3. **B3.** Finish the wizard to Home.
  4. Delete account B through Settings → Delete my data. The subscription is **set to cancel at period end
     with no invoice** (`account/handler.ts:88-94`), and later events are ignored (no such account).
- [ ] **B5.** `POST /billing-checkout` with `monthly` and "2026-09-10" answers 409 with D3's exact body.
      No consent row is added.
- [ ] **B6.** The upgrade sheet on a signed-out scratch install:
      - after sign-in, both plans appear with their lines;
      - a click on the lines starts nothing.
- [ ] **The S1 report** (`docs/reports/2026-11-2x-v041-s1-managed-payments.md`, under the rule above):
  - what Link sends, and when;
  - the receipt's sender;
  - whether a test refund returns the tax;
  - what the portal offers.

  Then checkpoint B.

**B. Part 2: after T5, on the PR head**
- [ ] Redeploy `billing-checkout`, so the floor is the new terms date. Build the dev app from the PR head.
- [ ] **B8.** A fresh scratch account opens both checkouts. The consent row carries the new terms version.
      B5 again: now "2026-09-10" is refused by the floor, with D3's sentence.
- [ ] **B7 (only if Managed Payments is in effect).** The re-consent panel shows the Link lines, and
      acceptance records the new version.
- [ ] **Clean up** the scratch profiles, vaults, credentials, autostart entries and staging accounts.
- [ ] **The PR body** says what was run and what was seen.

**C. Production (note §4; each step on Quinn's word)**
- [ ] **P-parity**, if T0 opened it and Q2 says yes:
  1. A scratch copy of `cloud/supabase`, with `migrations/` ending at `20261020000600_comp_guards.sql`.
  2. Show Quinn the file list the push will apply.
  3. `db push --include-all` from that copy.
  4. The `entitlement` redeploy.
  5. Re-read `migration list`.
- [ ] **Q9**, if agreed, for **each of Quinn's accounts that holds a subscription**:
  1. `admin.comp_grant` (an `ends_at` within a year), one production write per account.
  2. Quinn cancels its old-price subscription **immediately**, in both modes.
  3. Read back the account's view row: `tester`.
- [ ] **7.** Re-run T0's queries. The counts must equal T0's, except:
      - no `active` or `trialing` `monthly` or `academic_year` row remains;
      - Q9's grants are added.

      Anything else stops the rollout and goes to Quinn. A new account that is not Quinn's makes T5b
      required.
- [ ] **8.** The PR merges. Record the SHA.
- [ ] **9.** Quinn's live-mode checklist. Quinn reads back the live endpoint's enabled events, and the main
      session records whether `customer.subscription.trial_will_end` is present.
- [ ] **10.** Set production's two price ids, and deploy the three functions (and `account`, if T5b ran)
      from the merge SHA. Quinn reads the endpoint's deliveries since the deploy: every one is 2xx.
- [ ] **10b. The pre-tag live proof**, on a fresh production scratch account (an address Quinn supplies;
      sign-in by emailed code; `POST /account/consent` with the 18+ attestation and the new versions):
  - The `monthly` and "2026-09-10" POST answers **409 with D3's exact sentence** in `error`. The tester
    sentence fails this step.
  - `yearly` and `four_weekly` open live pages. A headless read shows:
    - the figures, "14 days", the tax line and D8's sentence;
    - "Sold through Link" if Managed Payments is in effect;
    - **its absence** if Q10 took the switch-off fallback under reading (ii).

    Nothing is paid.
  - **If Q11 is agreed**, the reminder round trip:
    1. Quinn completes `four_weekly` under the trial with their own card. There is no charge at checkout.
    2. `entitlements` shows `four_weekly`, `trialing`.
    3. Quinn moves the trial end to **no closer than 24 hours away**, and within 3 days.
    4. Wait for the reminder at the scratch address, for **at most 30 minutes**.
    5. **Quinn cancels immediately, whether or not the reminder came.** A reminder that did not arrive is
       recorded as a failed gate, never waited out.
    6. Read back `canceled` from Stripe and from `entitlements` before going on. If the read-back is not
       `canceled`, stop: nothing else runs until it is.
  - **If not, or if the event does not fire on the move:**
    - record the gap;
    - make one `mail-canary` send through production's `EMAIL_*`, if that function is deployed there;
    - Quinn accepts the gap at D.
  - `DELETE /account`, only after the `canceled` read-back. Then read-only: the account's rows are gone.
  - **What stays** (Q12): the `auto_renew` consent rows and the account's consent row carry the new terms
    version (and privacy version) before they are published. Deletion keeps them, with the account id
    nulled and the hash kept (`account/index.ts:114`). The live Stripe customer stays, and Link may keep a
    record. The ledger records these as counts.
- [ ] **11.** §8 D, then tag `v0.4.1` on the merge SHA. Watch `release.yml` through the Pages deploy.
- [ ] **12.** If every production account is Quinn's: Quinn's written acceptance, then the legal spec §4's
      insert and update **on each of Quinn's accounts**, one production write per account. This step does
      not apply if T5b ran.
- [ ] **13.** knowlu.com/terms.html shows the new version, `subscribed.html` says 14 days, and the updater
      offers 0.4.1 to a v0.4.0 install.

**D. Pre-tag gate (before step 11)**
- [ ] The merge tree equals the proved tree, or the proof restarts.
- [ ] CI on the merge SHA is green, with `warnings: N accepted (.rsrc), 0 tallies, 0 other`.
- [ ] `pwsh scripts\release.ps1 -DryRun` passes on a clean checkout.
- [ ] `pwsh scripts\ci\privacy-guard.ps1` passes.
- [ ] Steps 9, 10 and 10b are recorded, or their gaps are accepted by Quinn.
- [ ] **`site/` at the merge SHA equals the live site plus T5's pages.** The site port
      (`site/port-live-1119`) merged before the v0.4.0 tag, so the tag's Pages deploy (`release.yml:294-297`
      replaces the whole site) restores nothing old. Check it by comparing `site/` at the SHA with the live
      pages, T5's three excepted.
- [ ] Q13 is done.
- [ ] Ask Quinn, with these records.

## 9. Risks and stop rules

| Risk | Sign | What happens |
|---|---|---|
| Production lacks the testers or comp-guard migrations | T0's list | Q2's precondition. Step 10 does not run until it is done, because today's checkout reads `effective_entitlements`. |
| The parity push brings more than this lane | T0's list | The push runs only from a scratch copy ending at `20261020000600` (§3). It never runs from the integration worktree, which would also push `20261023000100`, `20261109000100` and `20261111000100`. If anything missing below the stamp needs a secret, a Vault entry or more redeploys, it is L9 proper, and v0.4.1 waits (Q2). |
| Managed Payments refuses a load-bearing key | T0r, or S1 | Quinn, at once (Q10). The fallback form ships, or the release holds. Never a silent drop. |
| Managed Payments needs `_shared/stripe.ts` changed | T1 | Stop. Seven functions would redeploy. |
| `trial_will_end` is not delivered in test mode | S1 (c) | Checkpoint B. Recommend holding: a missing reminder makes the first charge refundable. |
| The portal cannot cancel these subscriptions in one click | S1 (d) | Checkpoint B, then the fallback (D7). |
| A Dashboard price is wrong | D4's 500, on staging or at step 10b | Quinn corrects it, and a new id is set. Nothing was sold. |
| An unset price value on the webhook | the deliveries at step 10 | It cannot answer 500 (D2). An unknown id shows as `plan: null` and as a `billing-jobs` failure. |
| Counts change between T0 and step 7 | step 7 | Stop and tell Quinn. A new account that is not Quinn's makes T5b required. |
| The reminder does not arrive in 10b | 10b step 4 | Quinn cancels at 30 minutes anyway, and the `canceled` read-back comes before `DELETE`. The gate is recorded as failed, and Quinn decides at D whether to hold the tag. |
| The site port has not landed | T5 start, or §8 D | Stop. The tag would replace the live site with an unported `site/`, and T5 and the port both write `site/*.html`. |
| Bump A is open | T5 | T5's stop rule: one privacy key. |
| A task needs another contract-list file | any | Stop, amend the plan, and `contract-engineer` makes the change. |
| The window between step 10 and the tag | by design | Accepted. Sales are closed, and the only install is Quinn's. The tag follows the same day. |
| `release.yml` fails after step 10 | the tag's run | Fix forward on a new tag. Quinn may roll the three functions back to the pre-merge SHA, because the old functions and the old published terms are a coherent pair. |
| A Stripe fact proves false after use | any | Amend the note within the open round, never a third round. |
| The Stripe account changes owner mid-lane | Q4 | The new-account branch adds new secrets, a new endpoint, product, prices and portal, cleared customer ids, and S1 again. Recommend deferring it. |

**Two failed attempts** move one level up, effort before model. `max`, and Fable for a building role, are
Quinn's call.

## 10. Budget

- **Agent runs.** About twelve, plus fixes:
  - one `researcher`;
  - one or two `cloud-engineer`;
  - two `console-ui` (T4, and T5's drafts);
  - one `test-writer`;
  - one `contract-engineer`;
  - one `docs-keeper`;
  - four `reviewer` passes (cloud, app, text with docs, and the whole branch) and one `contract-reviewer`;
  - the note and plan re-review.

  T5b adds about three.
- **Fable** is used only by the planner.
- **Quinn's time.**
  - The twelve items of the note's §8, most of them one word each.
  - The test-mode checklist, about 30 minutes.
  - S1, about an hour.
  - Reading the pages.
  - The live-mode checklist, about 20 minutes.
  - Step 10b, about 30 minutes with the round trip.
  - About eight production words: T0's read, parity, Q9, step 7, the merge, step 10, step 10b and the tag.
- **Emailed codes on staging.** Three sign-ins (accounts A and B, and the sheet's scratch install). Plan
  across two hours. Production's step 10b needs one more.
- **Disk.** At most four live builds. Delete `target/` after each merge.

## 11. Ambiguities, and how this plan reads them

1. **"Existing `academic_year` rows map to the yearly plan."** The row keeps its name and $69.99 (D1). Quinn
   confirms (Q3), and Q1's amendment records it.
2. **When billing moves.** In v0.4.1 (Quinn, 2026-11-19), against v0.5.0 in the v0.4.0 note and the legal
   spec. The rest of the billing lane stays in v0.5.0 (note §7).
3. **A2 against P9.** A2's wizard clause ends with this lane, so P9 governs, and the subscribe step shows the
   prices (D9). Quinn confirms (Q5). If the app is to stay price-free, Q1's amendment narrows P9 in
   writing.
4. **The upgrade sheet.** It shows both plans, Yearly first, replacing the 2026-09-14 one-plan ruling (Q6).
5. **The gift year.** R3 says to keep it if the amendment has it, and it does. These terms still leave it out
   until the gift lane ships (Q7).
6. **Where the reminder lives.** The amendment's consequences list it under `billing-jobs`. That list is
   marked "not done by this amendment" and leaves mechanisms to the plan. The webhook sends it (D6), for
   the reasons D6 gives. P14's text is met either way.
7. **"Monthly" as a plan word.** The wording rule governs words beside a price. Settings shows no price, and
   an old `monthly` subscription is billed monthly, so the word stays.
8. **"One trial per account."** It means no `entitlements` row with a subscription id. The tombstone is not
   read (D5).
9. **The trial in the app.** P9 does not list it, and only the server knows whether it applies, so the app
   leaves it out and Stripe's page states it.
10. **"Plus tax where it applies."** This is P9's phrase, used in the app, the emails and `custom_text`. The
    terms keep the legal spec's "Sales tax is added at checkout where it applies".
11. **Managed Payments "already on."** This is the live account setting of 2026-09-14. Whether it governs
    sessions that do not ask for it is T0r's question 2 (D7's readings). Under reading (ii), Link is
    already the seller, so the Link texts ship (Q10). The only fallback is switching the setting off.
12. **"Quinn's account."** Production has two accounts that are Quinn's (`HANDOFF.md:58`). Every rule that
    names Quinn's account means each of them: the T5b trigger, Q9's grant and cancel, and step 12's
    acceptance. Quinn names them, and the ledger records counts.
13. **The terms floor equals `TOS_VERSION`.** So every later terms bump refuses every older build at
    checkout, typo fixes included (note D3). This is intended: a build that shows terms must send the
    version it shows, and the updater offers the new build.
