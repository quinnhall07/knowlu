# v0.4.1 pricing: build plan

> **For agentic workers:** REQUIRED SUB-SKILL: use superpowers:subagent-driven-development (recommended)
> or superpowers:executing-plans to run this plan task by task. Steps use checkbox (`- [ ]`) syntax.

**Status:** r4. r2 answered review r1. r3 applied review r2's fixes
(`docs/reports/2026-11-19-v041-pricing-spec-review-r2.md`, PASS with changes). **r4 revises r3 for
Quinn's ruling of 2026-11-20** (zero customers; prices that can change later), as the note's r4 does. §12
lists every task r4 changed, for a delta review.

**Size:** Contract. It moves money; it changes `app/src/account.rs` (`TOS_VERSION`, with D12
`PRIVACY_VERSION`, and the checkout command); it is a release; it adds a publishing workflow (T10); it
changes the terms; and with D12 it changes the privacy page.

**Spec:** `docs/specs/2026-11-19-v041-pricing-design.md` r4 ("the note"), cited by section (§n), by
decision (D1-D16) and by Quinn item: **Qn is item n of the note's §8 list** (the plan's own §8 is the proof and
rollout). Where this plan and the note disagree, the note wins. §11
lists where the note left room.

**Goal.** Ship v0.4.1 with four things true:
- **Billing charges the signed prices:** $12.99 every 4 weeks, or $84.37 once a year, with 14 days free
  once per account and a reminder before the first charge.
- **The app's subscribe step states those prices** (P9; Q5), as the server sends them.
- **The terms state them.**
- **A later price change, up or down, is a catalog change and a terms bump**, with no app release, by the
  runbook (ruling 7; D13-D16).

Managed Payments' per-session change is in if S1 passes. Every production step and the tag wait for
Quinn's word.

**Architecture.** One integration branch, `feat/v0.4.1-pricing`, cut from `main` after v0.4.0 has merged.
- **Cloud.** One new shared module, `_shared/price_catalog.ts` (D13), and three functions change
  (`billing-checkout`, which also answers `GET` with the offer; `stripe-webhook`; `billing-jobs`, which
  loses the pause and gains the move). There is no migration and no new function.
- **App.** `account.rs` gains `checkout_offer` and sends the offer back at checkout (T4a). Two app scripts
  render the offer (`wizard.js`, `carried/account.js`) and hold no price; one changes its plan words
  (`settings.js`).
- **Legal.** One legal commit moves the terms, `TOS_VERSION` and the catalog's `TERMS_VERSION` together.
- **Release and docs.** A site-only publishing workflow (T10) and the price-change runbook (T11).
- **Order.** Production parity is checked first (note §4 step 0). The functions then go to staging for S1,
  the rehearsal S2 and the proof, and to production before the tag, with a pre-tag live proof between.
  T10's first live run follows the tag.

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

Read for r4 on 2026-11-20, in this worktree:
- `billing-checkout/index.ts:1-65` and `handler_test.ts:1-343`, whole;
- `stripe-webhook/handler.ts:1-168`, `index.ts:1-73`;
- `billing-jobs/handler.ts:1-181`, `index.ts:1-48`, `handler_test.ts:1-363`, all whole;
- `_shared/stripe.ts:73-117` (the exports);
- `migrations/20260910000200_billing_jobs.sql:1-61`, `20260910000600_billing_jobs_vault.sql`, whole;
- `app/tests/account.rs:12-42, 752-757, 1415-1459`; `engine/tests/workflows.rs`, by its test names and its
  `release.yml` self-hosted pin (`:13-21, 47, 207-217`);
- `app/src/account.rs:1-60, 205-217, 875-904, 1160-1206, 1250-1310, 1570-1636`; `app/src/main.rs:181, 270`;
- `app/static/js/wizard.js:32-34, 205-222, 380-395, 636-655`; `carried/account.js`, whole;
  `pages/settings.js:20-44, 170-187`;
- `app/tests/ui_wizard.rs:579-616`, `bundle_config.rs:130-157`;
- `.github/workflows/release.yml:1-40, 240-312`, `ci.yml:145-178`; `scripts/release.ps1:293-297, 326`;
  `site/releases/` (only `.gitkeep`); `cloud/supabase/config.toml:101-165`;
- the legal spec `:185-193, 560-640, 755-794, 850, 908-914`;
- the draft Amendment 2026-11-17, read in the `price-amend` worktree (`:1234-1511`).

These facts are from `feat/v0.4.0-ui` on 2026-11-19, and this worktree on 2026-11-20. T0 re-checks them on
`main`.

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
  - by `contract-engineer` in T4a (the offer command and the checkout body), in T5's one commit, and in
    T5b's, if it runs;
  - reviewed by `contract-reviewer`.

  A task that needs any other file on the list stops (§9).
- **Commands.** One new Tauri command, `checkout_offer`, beside `open_checkout` in `account.rs`, listed in
  both `generate_handler!` lists (`main.rs:181, 270`). `open_checkout` (`account.rs:1610-1621`) gains two
  arguments, the offer's `amount_cents` and `terms_version`.

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

**Quinn's items Q1-Q14, by what they block** (the note's §8 has each recommendation):

| Item | What | Blocks |
|---|---|---|
| Q1 | **Answered 2026-11-20: yes.** Amendment 2026-11-10 on `main`, plus Amendment 2026-11-17 (drafted on `docs/price-amendments`), signed | T1a (both on `main` first) |
| Q2 | The production parity push, only if the testers and comp-guard stamps are the whole gap | the comp grants on Quinn's accounts; §8 C step 10 |
| Q3 | **Answered 2026-11-20 by ruling (6):** no `academic_year` subscription is kept | — |
| Q4 | Stripe and KNOWLU LLC: the same account's details, or a new account (deferred) | P7, the test-mode checklist, and so S1 |
| Q5 | The subscribe step shows P9's price lines (the amendment's (8), signed with it) | T4 |
| Q6 | The upgrade sheet sells both plans, Yearly first (the amendment's (9)) | T4 |
| Q7 | No gift paragraph in these terms (the amendment's (10)) | T5 |
| Q8 | The mailing address, or "email only" | T5 |
| Q9 | **Answered 2026-11-20 by ruling (6):** per Quinn account that holds an old-price subscription, comp grant, then cancel immediately. Each act still waits for Quinn's word | §8 C's own-subscriptions step, before step 7 |
| Q10 | Managed Payments in effect (reading (i) with the change, or (ii)), so Link on the pages; or the fallback | T5's scope (checkpoint B) |
| Q11 | The live reminder round trip | §8 C step 10b |
| Q12 | 10b's leftover production records | §8 C step 10b |
| Q13 | VISION's and CLAUDE.md's price lines in Quinn's words | the tag |
| Q14 | **New in r4.** The price-change design: server-named checkout terms version; sign-up keeps the compiled one; terms publish via `site.yml`, narrowing C25 to the privacy page (note D15) | T5, T10 |

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
- **The ruling's premise (r4).** From the counts and Quinn's naming: no `active`, `trialing`, `past_due` or
  `unpaid` subscription belongs to an account that is not Quinn's. If one does, stop and go to Quinn:
  ruling (6) assumes zero customers.

**T0r's questions** (the researcher writes `docs/reports/2026-11-19-v041-managed-payments-facts.md`, each
answer a quote with its URL and the date read, or "not found"; questions 9-11 are new in r4):
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
9. **Changing a subscription's price (r4, D14 `move`).** Whether updating a subscription item's price with
   `proration_behavior=none` leaves the current period as paid, bills the new price from the next renewal,
   keeps the billing date when the cadence is the same, and behaves the same on a `trialing` subscription
   and under Managed Payments.
10. **An archived price (r4, D14 `stay`).** Whether a subscription on an archived (inactive) price keeps
    renewing, and whether the portal still shows it.
11. **Subscription metadata (r4, D14).** Whether a metadata write on a subscription is kept under Managed
    Payments and fires `customer.subscription.updated` (the webhook must stay 2xx on it).

## 4. Tasks at a glance

Agents and efforts follow CLAUDE.md's table. Every `cloud-engineer`, `console-ui`, `test-writer`,
`implementer` and `docs-keeper` diff goes to `reviewer` (T10 to `contract-reviewer`). T4a's and T5's
commits go to `contract-reviewer`, and T5's page text to `reviewer`.

| Task | Note | Owns | Agent (model, effort) | After | Review |
|---|---|---|---|---|---|
| T0 baseline and production read | §4 step 0 | the ledger, the worktree | main session | §2's docs merge; Q1 | — |
| T0r Stripe facts | §3, D7 | `docs/reports/2026-11-19-v041-managed-payments-facts.md` | `researcher` (Opus, high), read-only, with web | — | checkpoint A |
| T1a price catalog (r4) | D13 | `cloud/supabase/functions/_shared/price_catalog.ts` (new), `cloud/supabase/functions/_shared/price_catalog_test.ts` (new) | `cloud-engineer` (Opus, high), first in the cloud run | T0 (Q1 on `main`) | `reviewer`, with T1-T3 |
| T1 checkout | D3, D4, D5, D7, D8, D9's offer | `cloud/supabase/functions/billing-checkout/{handler,index,handler_test}.ts` | `cloud-engineer`, same run | T1a, checkpoint A | `reviewer`, with T1a, T2 and T3 |
| T2 webhook and reminder | D2, D6 | `cloud/supabase/functions/stripe-webhook/{handler,index,handler_test}.ts` | `cloud-engineer`, same run | T1a, checkpoint A | as T1 |
| T3 billing jobs | D10, D14, D13's source scan | `cloud/supabase/functions/billing-jobs/{handler,index,handler_test}.ts`, `cloud/supabase/functions/_shared/price_single_source_test.ts` (new) | `cloud-engineer`, same run, last | T1a, T1, checkpoint A (T0r question 9) | as T1 |
| T4a offer command (r4) | D9, D15 | `app/src/account.rs` (`checkout_offer`, `checkout_offer_at`, `open_checkout`'s arguments, `checkout_url_at`'s body), `app/src/main.rs` (the command in both lists, `:181, 270`), `app/tests/account.rs` (two loopback tests) | `contract-engineer` (Opus, xhigh) | T1 merged (the offer's shape) | `contract-reviewer` |
| S1 test-mode spike | D7 gate, D5, D6 | the ledger; `docs/reports/2026-11-2x-v041-s1-managed-payments.md` | main session, with Quinn at the test Dashboard | T1a-T4a merged (S1 drives a T4 dev build); P7 | checkpoint B |
| S2 price-change rehearsal (r4) | D13-D16 | the ledger; a section of the S1 report; a scratch branch never merged | main session, with Quinn at the test Dashboard | S1; T0r answers 9-11 | checkpoint B |
| T4 subscribe step and labels | D9 | `app/static/js/wizard.js`, `app/static/js/pages/settings.js`, `app/static/js/carried/account.js`, `app/tests/ui_wizard.rs`, `app/tests/ui_settings.rs`, `app/tests/ui_plans.rs` (new), `scripts/ui-fixture/checks/t21.py`, `scripts/ui-fixture/checks/t16.py` (one assertion), `scripts/ui-fixture/checks/t20.py` (sheet assertions; its `#up-subscribe` checks at `:94, 115` stay), `scripts/ui-fixture/envelopes/t16-settings-answers.json` (one value), new `checkout_offer` envelopes under `scripts/ui-fixture/envelopes/` (an invented offer, a failure) | `console-ui` (Opus, high) | T1 merged; the command's name and shape from T4a's brief; Q5, Q6 | `reviewer` |
| T5 legal commit | D11, D12, D15's tie | red: `engine/tests/site.rs`, `app/tests/bundle_config.rs`, one appended test in `_shared/price_catalog_test.ts`, and with D12 `app/tests/ui_account_privacy.rs`. Draft: `site/terms.html`, `site/subscribed.html`, and with D12 `site/privacy.html` and `app/static/js/carried/reconsent.js`. The commit: those, plus `app/src/account.rs` (`TOS_VERSION`, and `PRIVACY_VERSION` with D12) and the `TERMS_VERSION` line in `_shared/price_catalog.ts` | `test-writer` (red), then `console-ui` (drafts), then `contract-engineer` (Opus, xhigh), who makes the one commit | checkpoint B; T0's count; Q7, Q8, Q14 | `contract-reviewer`; `reviewer` (text); Quinn reads the pages |
| T5b C1, only if T0 finds an account that is not Quinn's | D11 | the legal spec's C1: `account.rs`, `cloud/supabase/functions/account/{handler,handler_test}.ts`, `carried/reconsent.js` | `contract-engineer`, `cloud-engineer`, `console-ui` | T0 | `contract-reviewer`, `reviewer` |
| T6 docs | all | `HANDOFF.md`, `docs/reference/app.md`, `cloud/supabase/README.md`; marker lines in the legal spec and the v0.4.0 note | `docs-keeper` (Sonnet, medium) | T5 | `reviewer` |
| T7 version 0.4.1 | — | `Cargo.toml:9`, `app/tauri.conf.json:4`, `Cargo.lock` | main session | T5 | T8 |
| T8 whole-branch review | all | none | `reviewer` (Opus, high) | T4-T7; T10 and T11 if merged (they do not gate the tag: if late, they go in a follow-up PR, reviewed the same way, before the first price change) | — |
| T9 proof, PR, rollout, tag | §4 | none | main session, on Quinn's word at each production step | T8 PASS | Quinn |
| T10 site-only publishing (r4) | D15 | `.github/workflows/site.yml` (new), `engine/tests/workflows.rs` (its pins) | `implementer` (Opus, high): a release workflow off the contract list | T0; Q14 | `contract-reviewer` (a release workflow, as the legal spec's C25 asks) |
| T11 price-change runbook (r4) | D16 | `docs/reference/price-change.md` (new) | `docs-keeper` (Sonnet, medium), transcribing D16 with the final names | T1a-T4a, T10 merged; S2 run (its lessons go in) | `reviewer`, against D16 row by row |

**Why these agents.**
- T1a-T3: `cloud-engineer` owns `cloud/`; one run keeps the catalog and its three readers consistent.
- T4a and T5's commit: `contract-engineer`, because `app/src/account.rs` is on the contract list.
- T4 and T5's drafts: `console-ui` owns `app/static` and `site/`.
- T5's red: `test-writer`, because the pins are written before the pages.
- T10: `implementer`. It is fully specified and checked by `workflows.rs` pins, off the contract list, but it
  can break a release, so Opus at high, and `contract-reviewer` reads it.
- T6 and T11: `docs-keeper`. T11 transcribes D16 with the final names; it designs nothing.
- T0, S1, S2, T7, T9: the main session, because they hold sessions, set secrets or need Quinn's word.

**What can run at once.**
- T0r beside T0.
- T1a, then T1-T3, as one `cloud-engineer` run, with disjoint files.
- T4a, T4 and T5's red dispatch after T1 merges.
- T10 beside the cloud run, after Q14.
- T6 beside T7 and T11.

**What runs in order.**
- T1a, T1-T3, T4a and T4, then S1, then S2. S1 drives a T4 dev build, whose wizard can reach a checkout. The
  catalog's `TERMS_VERSION` is still 2026-09-10 then, and the offer carries it.
- `_shared/price_catalog.ts`: T1a, then T5's one line. `_shared/price_catalog_test.ts`: T1a, then T5's one
  appended test.
- `account.rs`: T4a, then T5, then T5b.
- **T10 and the waitlist lane's site-only deploy (W-D3):** if W-D3 has merged by T10's start, T10 narrows
  its legal-page guard to `site/privacy.html` and adds the release-assets restore if it lacks one, instead
  of adding `site.yml`. If both are open, the main session sequences them; they both write a site-deploy
  workflow.
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
- [ ] Confirm Q1 on `main`: the cloud design carries Amendment 2026-11-10 and Amendment 2026-11-17, signed.
      If not, stop: T1a does not start.
- [ ] Re-confirm each `path:line` the briefs rely on (the "Code read" list). Fix this plan first if one has
      moved.
- [ ] §3's production read, on Quinn's word. Record:
      - which of the two migrations production has, and the dates of its three billing functions;
      - the counts.

      The outcomes:
      - **A migration missing** opens "production parity for v0.4.1" (Q2), scoped as §3 says.
      - **Any account that is not Quinn's** adds T5b, a tester's included. Quinn's own accounts
        (`HANDOFF.md:58`) never do.
      - **A paying subscription on an account that is not Quinn's** stops the lane and goes to Quinn (the
        ruling's premise, §3).
      - *Removed in r4:* the old-subscriber paragraph's condition. Ruling (6) leaves it out always.

### T0r. Stripe facts (`researcher`)
- [ ] §3's eleven questions. The researcher writes only the report.
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
  - **Questions 9-11 (r4)** go into T3's brief. If question 9's answer is that a no-proration price change
    bills or credits the current period, stop T3's move step and go to Quinn: D14's "never on a paid
    period" would not hold.

### T1a. The price catalog (`cloud-engineer`; new in r4)

The one module every billing function reads (note D13). It writes no file and calls nothing.

**Red** (`_shared/price_catalog_test.ts`):
- `the_catalog_sells_two_plans_yearly_first`: `SELLING` holds `yearly` then `four_weekly`, at 8437 and
  1299; `KEPT` is empty; `TERMS_VERSION` is a quoted `YYYY-MM-DD`.
- `every_entry_keeps_its_keys_cadence`: `four_weekly` is week × 4 and `yearly` is year × 1 for every
  entry; no key and cents appear twice.
- `the_yearly_amount_is_thirteen_whole_cent_charges`: 8437 / 13 = 649 exactly; an invented 8999 fails.
- `a_move_entry_notices_inside_seven_to_thirty_days`: N from 7 to 24 passes; 6 and 25 fail.
- `the_variable_names_are_derived`: `STRIPE_PRICE_FOUR_WEEKLY_1299` and `STRIPE_PRICE_YEARLY_8437`;
  resolving with one variable unset leaves that entry matching no id and the other resolved.
- `the_formatter_writes_every_4_weeks_and_once_a_year`: the two phrases, the P9 lines word for word (the
  note's §1), the tax line, the money-back line, and an invented entry at 1399 giving "$13.99 every 4
  weeks". Generic cadences for unknown prices: "every 2 months", "every week", "every year".
- Two catalog tests live elsewhere, because each task merges green: the source scan is T3's red, in its
  own file (the last cloud task, when every literal is gone), and the terms tie is T5's red (when the
  terms change).
- **Which tests name the selling amounts.** Only `the_catalog_sells_two_plans_yearly_first`, so a price
  change edits one test, on purpose and in view (note D16 step 2). Every handler test in T1-T3 builds its
  own catalog from a table in the test, through the same resolve helper; none imports the real `SELLING`.

**Change.** The module, its types and its pure helpers (resolve from an environment getter, look up by
price id, the formatter, the offer). Nothing imports it yet. `TERMS_VERSION` is 2026-09-10 until T5.

### T1. Checkout (`cloud-engineer`)

**Red** (`billing-checkout/handler_test.ts`; every handler test is given a catalog through `Deps`):
- `the_signed_plans_reach_stripe_with_their_price`: each plan sends its own price id, and the consent row
  carries 1299 or 8437 and the catalog's `TERMS_VERSION`.
- `the_offer_is_the_catalogs` (r4): `GET` with a session answers `{ terms_version, plans, note }`, plans in
  the catalog's order, each `{ plan, amount_cents, lines }`; with an invented catalog at 1399 the answer
  says "$13.99". The fake records no account read and no Stripe call. No session is 401; `PUT` is 405
  naming GET and POST.
- `an_older_build_is_told_to_update`: `monthly` and `academic_year` answer 409 with D3's exact sentence.
  After the session check, the fake records no call: no account read, no tester read, no customer, no
  consent row, no session.
- `an_unknown_plan_is_still_400`.
- `an_offer_with_a_missing_field_is_400` (r4): no `amount_cents`, or no `terms_version`.
- `a_changed_offer_is_refused` (r4): `amount_cents` 1298 on `four_weekly`, 8437 on `four_weekly`, and a
  `terms_version` of "2026-09-09" each answer 409 with D3's "changed" sentence, with no account read.
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
    at 8437;
  - (r4) a selling variable left unset in the resolved catalog.
- `the_form_keeps_every_load_bearing_key`: D7's load-bearing keys are present (the trial key only on a
  first subscription, D5). The dropped keys equal checkpoint A's list exactly. If the fallback applies,
  today's tax keys are pinned instead.
- `stripes_page_states_the_renewal_terms`: D8's sentence, for each plan and trial combination.
- The `index.ts` text-read (`handler_test.ts:331-343`) checks:
  - that it imports the catalog and resolves it from `Deno.env.get`, and holds no cents map (the map at
    `index.ts:16-20` goes);
  - `isTester` unchanged.

  Nothing in T1's tests names a terms date, so T5's commit changes no test file of T1's.
- Existing tests move to the new plans and carry the offer's two fields. The 18+ and tester assertions are
  unchanged.

**Change.**
- `Plan` becomes `"four_weekly" | "yearly"`, with a separate list of the two old names.
- `Deps` gains `catalog` (resolved), `subscriptionState(accountId)`, which returns `stripe_subscription_id`
  and `status`, and `getPrice(id)` (through `stripeGetFrom`). `priceFor` and `priceCentsFor` go: the
  catalog answers both.
- `GET` answers the offer from the catalog after `requireUser`; `POST` is the purchase. Any other method is
  405 naming both.
- The purchase's order is: method, user, parse, D3's three checks, account, 18+, tester, D5, D4, customer,
  consent, session.
- D3's offer check is equality with the catalog's cents for the plan and its `TERMS_VERSION`. r3's floor,
  pattern and date rule are not built.
- D4 compares the interval and the count to the key's cadence in the catalog.
- A different API version, if T0r requires one, goes on the session call only. `_shared/stripe.ts` is not
  edited.

**Stop rule.** If `_shared/stripe.ts` must change, stop: seven functions would redeploy.

### T2. Webhook and reminder (`cloud-engineer`)

**Red** (`stripe-webhook/handler_test.ts`; the catalog comes through `Deps`):
- `the_plan_comes_from_the_price_id`:
  - both selling ids map to their keys;
  - (r4) a `KEPT` id, same key at other cents, maps to that key;
  - an unknown id gives `null`;
  - a week × 4 price with an unknown id gives `null`.

  This replaces `:18-62`'s interval cases. The period and paused assertions stay.
- `a_missing_price_setting_matches_nothing`: a catalog with one variable unset still maps the others, and
  no event answers 500.
- `trial_will_end_sends_one_reminder`, for a trialing subscription with a known price and a future
  `trial_end`:
  - one `sendEmail` to the account's address;
  - the text has the plan words and the amount from the catalog entry the id matches ("$84.37 once a year"
    or "$12.99 every 4 weeks"), "plus tax where it applies", the UTC−10 date and "Manage subscription";
  - (r4) with an invented catalog entry at 1399 for that id, the text says "$13.99 every 4 weeks";
  - no "in 3 days" and no "per month";
  - the event is recorded, and `writeEntitlement` is not called.
- `an_unknown_price_still_gets_its_reminder`: no plan words, and the amount and interval from the event,
  through the catalog's formatter.
- `no_reminder_for_a_subscription_that_will_not_charge`: `cancel_at_period_end`, a status other than
  `trialing`, or a past `trial_end` each send nothing and record the event.
- `a_failed_reminder_is_retried_by_stripe`:
  - 502, and nothing recorded;
  - the captured `console.error` line holds the account id and no "@".
- `a_duplicate_trial_will_end_sends_nothing`.
- `the_date_is_never_later_than_the_students_own`: a pure helper, with a case one day before the UTC date.
- `index.ts` text-read: the catalog resolved with every variable optional, through `Deno.env.get` and never
  the file's throwing `env` (`stripe-webhook/index.ts:6-10`); no `STRIPE_PRICE_` literal; and the three
  `EMAIL_*` names.

**Change.**
- `entitlementFromSubscription(sub, catalog)`.
- A `trial_will_end` branch after the signature, duplicate and account checks, with deps `accountEmail`
  and `sendEmail` (the shape of `billing-jobs/index.ts:26-36`).
- The handled event types gain `trial_will_end`.

### T3. Billing jobs (`cloud-engineer`; revised r4)

**Red** (`billing-jobs/handler_test.ts`; the catalog and a subscription read come through `Deps`):
- `no_subscriber_is_ever_paused` (r4): on 2027-06-02, a `four_weekly` and a `yearly` subscriber; no
  `pause_collection` call is made, and the reply has no `paused` or `resumed` count.
- `the_annual_reminder_states_the_subscribers_own_price` (r4): the subscription is read; with an invented
  catalog entry at 1399 for its price id the mail says "$13.99 every 4 weeks, plus tax where it applies",
  never "per month"; a `yearly` id says "$84.37 once a year". It replaces `:291-330` (which pins "$69.99
  per academic year") and keeps that test's "Manage subscription" pins.
- `a_kept_price_is_reminded_at_its_own_amount` (r4): a `stay` entry's id gets its own cents, not the
  selling price's.
- `an_unknown_price_is_never_mailed` (r4): an id the catalog lacks, and a failed subscription read, each
  count in `failed`, send nothing, and the next subscriber is still reached. Replaces `:332-363`.
- `the_cancel_path_names_settings_and_membership`: "Settings", "Membership" and "Manage subscription"
  present, and no "gear".
- **The move** (r4, D14), with a `KEPT` entry at `move` N = 10:
  - `a_move_notices_then_marks_then_changes`: a subscriber on that price renewing in 12 days gets one mail
    (the old and the new amount, the UTC−10 date, the tax line, "Manage subscription"), then one metadata
    write, then one price change to the selling id with `proration_behavior` "none", in that order;
  - `no_move_outside_the_window`: 9 days and 17 days away, another price, a `stay` entry, and
    `cancel_at_period_end` each make no call and send nothing;
  - `a_failed_notice_changes_nothing`: no metadata write and no price change;
  - `a_noticed_subscription_is_changed_without_a_second_notice`: metadata already names this renewal;
  - `one_move_failure_does_not_stop_the_run`.
- `no_amount_is_written_outside_the_catalog` (r4; its own new file, `_shared/price_single_source_test.ts`,
  so T3's files stay disjoint from T1a's): reads every non-test `.ts` file under `billing-checkout/`, `stripe-webhook/`,
  `billing-jobs/` and `billing-portal/` (deno runs with `--allow-read`, `ci.yml:172`) and fails on a `$`
  before a digit, a decimal money figure such as `12.99`, any catalog entry's cents as a number token, a
  `STRIPE_PRICE_` name, "per month" or "a month". Red against `billing-jobs/handler.ts:72-73` until T3's
  change.
- The pause and resume tests (`:4-17`, `:70-245`) go with the code. `reminderDue`'s test (`:19-50`), the
  token test (`:52-68`) and the one-failure test (`:247-289`, moved to the new plans) stay.

**Change.**
- Remove `shouldBePaused`, `pauseDecision`, `resumeNotice`, `priceOf` and the `monthly` branch
  (`handler.ts:46-58, 69-75, 92-105, 126-166`). The reply becomes `{ reminded, moved, failed }`.
- `Deps` gains `catalog`, `getSubscription(id)` (through `stripeGetFrom`, `_shared/stripe.ts:117`) and keeps
  `stripe` (POST) for the move. `index.ts` wires both.
- The annual reminder and the move run per subscriber inside today's `try/catch` (`:117-178`).
- "click the gear" (`:86`) becomes "go to Settings, then Membership".

**Stop rule.** If T0r's question 9 says a no-proration change touches the current period, the move step is
not built; go to Quinn (checkpoint A's rule).

### S1. Test-mode spike (main session; §8 B; after T1a-T4a)

### T4. Subscribe step and labels (`console-ui`; built as Q5 and Q6 rule)

The app renders the server's offer and holds no price (note D9, revised r4). The offer's shape, from T1:
`{ ok, error, terms_version, plans: [{ plan, amount_cents, lines }], note }`, `plans` Yearly first. The
command is `checkout_offer` (T4a); `open_checkout` takes `{ plan, amountCents, termsVersion }`.

**Red:**
- `ui_wizard::the_subscribe_buttons_send_the_signed_plans`:
  - `#wiz-sub-year` ("Yearly", `ibtn`) comes before `#wiz-sub-month` ("Every 4 weeks", `wbtn`);
  - the mapping is `? "yearly" : "four_weekly"`;
  - `open_checkout` is invoked with the plan, that plan's `amount_cents` from the offer and the offer's
    `terms_version`.
- `ui_wizard::the_wizard_holds_no_price_and_renders_the_offer` (r4), which replaces
  `the_wizard_states_no_price_or_trial` (`ui_wizard.rs:584-602`).
  - The test keeps today's method (`ui_wizard.rs:585-597`): `string_literals` over **both** `js/wizard.js`
    and `js/picker.js`. The status word `"trialing"` (`wizard.js:162`) is the one exception to the trial
    ban. For every literal, none of these appears: "$", a decimal money figure, "9.99", "12.99", "84.37",
    "6.49", "69.99", "every 4 weeks" (the button label "Every 4 weeks" is a plan word; the test compares
    case-sensitively and allows only that exact label), "a year", "per year", "trial", "7-day", "seven day",
    "14-day", "14 days", "fourteen", "per month", "/month", "a month", "/mo", "free first" or "pell".
  - The code asks `invoke("checkout_offer"` on entering the step (after `:384-393`'s entitlement check),
    and writes each line with `textContent`; no `innerHTML` takes an offer field.
  - The subscribe panel's markup carries no price line; it keeps its heading and lede.
- `ui_wizard::without_an_offer_the_plan_buttons_wait` (r4): until an offer is held, both plan buttons are
  rendered `disabled`; on a failed or incomplete answer the step shows "Prices could not be loaded. Check
  your connection, then try again." and a Try again button that asks again; nothing is written to
  `localStorage`, `sessionStorage` or a command that saves.
- `ui_settings::membership_shows_no_price` gains "84.37", "6.49" and "14 days".
- `ui_settings::plan_words_cover_every_plan_billing_can_write`: `four_weekly` and `yearly` have words;
  `monthly` and `academic_year` have none.
- New file `ui_plans.rs`:
  - `the_upgrade_sheet_sells_both_plans_yearly_first`: `carried/account.js` sends `yearly` and
    `four_weekly` with the offer's two fields, with the yearly button first, inside `#up-subscribe`.
  - `the_sheet_holds_no_price`: the wizard test's literal rule over `carried/account.js`.
  - `only_the_two_plan_buttons_start_checkout`: the sheet's click handler starts `subscribe` only from
    `#up-sub-year` or `#up-sub-month`, never from `closest("#up-subscribe")`. Today's handler
    (`carried/account.js:214`) would start a checkout with no plan on a click on the price lines.
  - `every_plan_the_app_sends_is_one_the_catalog_sells`: the two keys against `SELLING` in
    `_shared/price_catalog.ts`, read as text.
  - `every_plan_the_catalog_can_name_has_a_word`: every key in the catalog has a word in `settings.js`.
- **The DOM checks:**
  - Two new envelopes for `checkout_offer`: an invented offer (Yearly "$9.87 every 4 weeks, billed $128.31
    once a year. Renews yearly."; Every 4 weeks "$1.23 every 4 weeks."; note "Plus tax where it applies."),
    and a failure.
  - `t21.py`: `:16`'s docstring changes. `:264` expects the new labels, exactly the invented envelope's
    lines and no "trial"; with the failing envelope, both buttons disabled and the retry sentence.
    `:273` expects `[{"plan": "yearly", ...}, {"plan": "four_weekly", ...}]` with the envelope's
    `amount_cents` and `terms_version`.
  - `t16.py::check_settings`: "Active, Every 4 weeks" (`settings.js:185`), with
    `t16-settings-answers.json:10` set to `four_weekly`.
  - `t20.py`: once shown, the sheet offers Yearly before Every 4 weeks, with the invented envelope's lines.

**Change.**
- `wizard.js:32-34` and `:641-646`: the panel without lines, the order, the mapping, the offer request after
  the entry check, the lines drawn as text, the disabled state and the retry. The lede drops "and shows the
  price". `:211-219`'s visibility rules stay.
- `settings.js:32`: the two words.
- `carried/account.js:83, 129, 134-137`: `#up-subscribe` becomes the wrapper that is hidden and shown,
  holding the two buttons (`#up-sub-year`, `#up-sub-month`), the lines and the retry. Its id stays, so
  `ui_account_privacy.rs:66, 105, 333` and `t20.py:94, 115` stay green unchanged. Where `:129` shows
  Subscribe today, the sheet asks for the offer.
- `carried/account.js:214`: the handler binds the two buttons. `subscribe(plan)` takes the plan from the
  button pressed and that plan's offer, and a click elsewhere in the wrapper does nothing.
- A checkout refusal shows the server's sentence and asks for the offer again.

**Stop rule.** If any `ui_account_privacy.rs` pin must change, stop. That file is T5's, and the main session
sequences the two.

**If Q5 is "price-free":**
- the lines are not drawn; the offer is still asked for, for its two fields;
- the DOM checks expect no lines.

**If Q6 is "one plan":** the sheet keeps one button, sending `four_weekly`.

The DOM runs use `scripts/ui-fixture/run.py`, at both sizes and in both themes.

### T4a. The offer command and the checkout body (`contract-engineer`, xhigh; new in r4)

`app/src/account.rs` is on the contract list (note D9, D15).

**Red** (`app/tests/account.rs`, loopback on `127.0.0.1:0`, no real credential):
- `checkout_offer_at_reads_the_offer`: `checkout_offer_at(api_base, token)` sends `GET /billing-checkout`
  with `authorization: Bearer <token>` and returns the JSON body unchanged; a non-2xx gives the server's
  sentence through `provider_error` (`account.rs:210-217`); an unreachable server gives the `UNREACHABLE`
  sentence.
- `checkout_url_at_sends_the_offer_it_was_given`: the POST body is exactly
  `{"plan", "amount_cents", "terms_version"}` with the given values, and carries no `TOS_VERSION` unless it
  was the given value. The consent retry before it (`:1297`) still sends `TOS_VERSION` (sign-up's, D15).
- Both use the file's `loopback` helper (`app/tests/account.rs:12`), which reads each request's body by
  `Content-Length` (`:36-37`) and returns what it saw. Neither touches Credential Manager, so neither takes
  `CREDMAN_LOCK`.
- The existing `the_checkout_retries_the_consent_first_and_a_failed_retry_still_reaches_stripe`
  (`:1430-1442`, which calls `checkout_url_at` with `"monthly"`) moves to the new signature with
  `four_weekly`, 1299 and a version; its assertions stay.

**Change.**
- `checkout_offer_at` and the command `checkout_offer`, which reads the pending session as
  `entitlement_now` does (`account.rs:1194-1206`) and answers `{ ok, error, terms_version, plans, note }`.
- `checkout_url_at(api_base, token, plan, amount_cents, terms_version)` (`:1292-1301`), and
  `open_checkout(plan, amount_cents, terms_version)` (`:1610-1621`) passing them through.
- `main.rs:181` and `:270`: `account::checkout_offer` beside `account::open_checkout` in each list.
  Recount each list before quoting a number.

**Stop rule.** If the change needs `credentials.rs` or any other contract-list file, stop and amend the plan.

### S2. The price-change rehearsal (main session; §8 B; after S1; new in r4)
- §8 B's S2 steps. It proves D13-D16 end to end in test mode, and the runbook's staging half.

### T5. The legal commit (`test-writer`, then `console-ui`, then `contract-engineer`)

**Red** (`test-writer`):
- `site::the_terms_state_the_signed_prices`: the note §6 strings, present and absent. "summer pause"
  appears only in "There is no summer pause." (r4: no old-subscriber paragraph, ever; "If you subscribed
  before" is absent).
- `site::the_terms_carry_the_price_change_paragraph` (r4): the legal spec `:625` paragraph, with "between 7
  and 30 days ahead" and "never applies to a period you have already paid for".
- `price_catalog_test::the_terms_state_the_catalogs_prices` (r4; deno, in T1a's file, which this task may
  append to): every `$` figure in `site/terms.html` is some catalog entry's; every `SELLING` figure is
  there; the page's version is `TERMS_VERSION`; and the price-change paragraph names the catalog's 7 and
  30. Red until the commit below.
- `site::the_terms_carry_the_interim_terms_clauses`: the legal spec §7 item 8's strings, and no "Alabama".
- `site::the_terms_promise_no_code_this_release_lacks`: none of "When you subscribe:", "Before the yearly
  plan renews", "A year for someone else" or "does not count as paid".
- `site::the_subscribed_page_names_fourteen_days`: "14 days", "first subscription", "Settings", and no
  "seven-day" or "gear".
- `bundle_config::the_terms_version_constant_is_the_published_terms_date`: the privacy tie's shape, plus
  the catalog's `TERMS_VERSION` (read as text from `_shared/price_catalog.ts`) equal to `TOS_VERSION`. This
  is green on arrival (both 2026-09-10, the page's date) and is recorded as a guard.
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
proof, because the production account function refuses a sign-up version later than tomorrow
(`account/handler.ts:199-201`). Checkout has no date rule (note D3).

**The commit** (`contract-engineer`, xhigh) moves, in one commit:
- `TOS_VERSION`, and `PRIVACY_VERSION` if Managed Payments is in effect;
- the catalog's `TERMS_VERSION` (one line in `_shared/price_catalog.ts`);
- the pages and the key.

Message: `t05: terms <date> state the signed prices[; privacy <date> names Link]`. After it merges, the
main session redeploys the three billing functions to staging before the final staging proof (§8 B,
part 2): each bundles the catalog. The commit touches no T1 test: nothing in T1's tests names a terms
date.

**Stop rule.** If bump A's PR is open, the main session decides which PR carries the privacy text, so that
only one key ships.

### T5b. Terms re-acceptance (only if T0 found an account that is not Quinn's)
- The legal spec's C1, to its §7 item 10. It must land before the tag. Production's `account` is redeployed
  at §8 C step 10 with the others.

### T6. Docs (`docs-keeper`)
- **`HANDOFF.md`:** v0.4.1 is billing at the signed prices; the rest stays in v0.5.0; production parity
  as found by T0.
- **`docs/reference/app.md:309-311`:** the price line, as worded in Q1's amendment.
- **`cloud/supabase/README.md`:** a "Billing" section covering the plan keys, the catalog (D13) and its
  derived `STRIPE_PRICE_<KEY>_<CENTS>` names, the offer (`GET /billing-checkout`), the six handled event
  types, the 409s, the price check, the move (D14), and a pointer to `docs/reference/price-change.md`.
- **`docs/reference/app.md`'s release section:** `site.yml` (T10), and the warning that a hand
  `wrangler pages deploy site` takes the installer and `latest.json` off knowlu.com (note D15).
- **Marker lines** in the legal spec (§5's "Terms + Privacy B" row and the interim block; C25 narrowed to
  the privacy page, if Q14 is yes) and in the v0.4.0 note (A2's wizard clause, ended by this lane, and §13
  item 1).

### T10. Site-only publishing (`implementer`, Opus high; new in r4)

Note D15. It does not gate the v0.4.1 tag; it must merge before the first price change.

**Red** (`engine/tests/workflows.rs`, beside today's pins):
- `site_yml_is_started_by_hand_only`: `on:` is `workflow_dispatch` and nothing else.
- `site_yml_runs_from_main_only`: a step refuses any `github.ref` but `refs/heads/main`.
- `site_yml_restores_the_release_files_first`: before the deploy step, it downloads the latest release's
  versioned installer and, when the release has one, `latest.json` (`release.yml:307-311` names them) into
  `site/releases/`, copies the installer to `Knowlu-setup.exe` (`scripts/release.ps1:297`), and fails if
  the installer is missing.
- `site_yml_refuses_a_changed_privacy_page`: it compares `site/privacy.html` with the latest `v*` tag's
  and stops if they differ.
- `site_yml_deploys_as_the_release_does`: the same pinned `wrangler-action` SHA and `pages deploy site
  --project-name knowlu --branch main` (`release.yml:294-298`), and a `deploy_branch` input that defaults to
  `main` and allows a preview branch.
- `site_yml_never_runs_self_hosted`: no `self-hosted` and no `CI_SELF_HOSTED`, as `release.yml` is pinned
  (`workflows.rs:216-217`; that pin names only `release.yml`). The SHA pin (`:47`) already reads every
  workflow file (`:13-21`), so it covers `site.yml` unchanged.

**Change.** `.github/workflows/site.yml`, GitHub-hosted, `permissions: contents: read`, the same
`CLOUDFLARE_*` secrets as `release.yml:296-297`.

**Stop rule.** If the waitlist lane's site-only deploy (W-D3) has merged, do not add a second workflow:
narrow its legal-page guard to `site/privacy.html` and add the restore if it lacks one. If it is open,
stop; the main session sequences the two.

**Its proof** (main session): first to a preview branch, reading the preview's pages, installer and
`latest.json`; then the live run at §8 C step 14, on Quinn's word.

### T11. The price-change runbook (`docs-keeper`; new in r4)
- Write `docs/reference/price-change.md` from note D16, step for step, with the final names from T1a-T4a
  and T10: the variable names, the catalog's fields, the commands to deploy the three functions, the
  `site.yml` dispatch, and each check. Each live step says "on Quinn's word".
- Add the S2 lessons the main session records.
- A pointer from `cloud/supabase/README.md`'s Billing section (T6).
- `reviewer` checks it against D16 row by row; no step may be missing or reordered.

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
| D1 | new keys; old keys never sold or written; no migration | T1, T2, T4 | `billing-checkout::the_signed_plans_reach_stripe_with_their_price`, `::an_older_build_is_told_to_update`; `ui_plans::every_plan_the_app_sends_is_one_the_catalog_sells`; `ui_settings::plan_words_cover_every_plan_billing_can_write` (no old words) |
| D1 | a key outlives its price (r4) | T1a, T2 | `price_catalog_test::every_entry_keeps_its_keys_cadence`; `stripe-webhook::the_plan_comes_from_the_price_id` (the `KEPT` id) |
| D2 | plan from the price id through the catalog; unknown gives `null`; values optional; no old value read | T2 | `stripe-webhook::the_plan_comes_from_the_price_id`, `::a_missing_price_setting_matches_nothing`; the text-read; S1 (a) |
| D3 | old names refused first; unknown is 400; the offer must match the catalog | T1 | `billing-checkout::an_older_build_is_told_to_update`, `::an_unknown_plan_is_still_400`, `::an_offer_with_a_missing_field_is_400`, `::a_changed_offer_is_refused`; §8 C step 10b (both bodies) |
| D3 | the consent row records the catalog's cents and terms version | T1 | `billing-checkout::the_signed_plans_reach_stripe_with_their_price` |
| D4 | six price checks before the consent row, with the interval and the count tied to the key; an unset variable | T1 | `billing-checkout::the_price_is_checked_before_it_is_sold` (including a `four_weekly` id on a `year` × 1 price, and an unset variable) |
| D5 | the trial on a first subscription only; card always; one live subscription | T1 | `billing-checkout::the_trial_is_fourteen_days_on_a_first_subscription_only`, `::an_account_with_a_live_subscription_is_refused`; S1 (b) |
| D6 | the reminder's conditions, wording from the catalog, unknown price, retry, log, no entitlement write | T2 | the six `stripe-webhook` reminder pins (the invented 1399 case included); S1 (c) |
| D6 ⚑ | the event fires, and the reminder arrives in live mode | T0r, T9 | T0r's quote; §8 C step 9 (events read back); step 10b (round trip, Q11) |
| D7 | load-bearing keys kept; only allowed keys dropped | T1, checkpoint A | `billing-checkout::the_form_keeps_every_load_bearing_key` |
| D7 ⚑ | reading (i) or (ii); the test-mode gates (a)-(e) | T0r, S1 | T0r's report; the S1 report |
| D7 | `_shared/stripe.ts` untouched | T1 | T8's diff check |
| D8 | the renewal sentence | T1 | `billing-checkout::stripes_page_states_the_renewal_terms`; S1 (a); step 10b |
| D9 | the offer from the server: one catalog, one bundle | T1, T4a | `billing-checkout::the_offer_is_the_catalogs`; `account::checkout_offer_at_reads_the_offer` |
| D9 | the purchase sends the offer back | T4a, T4 | `account::checkout_url_at_sends_the_offer_it_was_given`; `ui_wizard::the_subscribe_buttons_send_the_signed_plans`; DOM t21 (`:273`) |
| D9 | the app holds no price and renders what it is sent; order; no trial words | T4 | `ui_wizard::the_wizard_holds_no_price_and_renders_the_offer`; `ui_plans::the_sheet_holds_no_price`; DOM t21 and t20 with the invented envelope |
| D9 | no offer, no figure; nothing cached | T4 | `ui_wizard::without_an_offer_the_plan_buttons_wait`; DOM t21 with the failing envelope |
| D9 | the sheet sells both plans; only the buttons start checkout | T4 | `ui_plans::the_upgrade_sheet_sells_both_plans_yearly_first`, `::only_the_two_plan_buttons_start_checkout`; DOM t20 |
| D9 | Settings words, and no price | T4 | `ui_settings::plan_words_cover_every_plan_billing_can_write`, `::membership_shows_no_price`; `ui_plans::every_plan_the_catalog_can_name_has_a_word`; DOM t16 |
| D9 ⚑ | the portal has no switching | Quinn | the checklist; S1 (d) |
| D10 | the pause removed; the annual reminder states the subscriber's own price from the catalog; the cancel path | T3 | `billing-jobs::no_subscriber_is_ever_paused`, `::the_annual_reminder_states_the_subscribers_own_price`, `::a_kept_price_is_reminded_at_its_own_amount`, `::an_unknown_price_is_never_mailed`, `::the_cancel_path_names_settings_and_membership` |
| D13 | one catalog; its rules; derived variable names; one formatter | T1a | the six `price_catalog_test` pins |
| D13 | no amount outside the catalog on the server | T3 | `price_single_source_test::no_amount_is_written_outside_the_catalog` |
| D13 | the terms state the catalog's prices and version | T5 | `price_catalog_test::the_terms_state_the_catalogs_prices`; `bundle_config::the_terms_version_constant_is_the_published_terms_date` |
| D14 | `stay` and `move`: notice first, then mark, then change; the window; retries; no second notice | T3 | the five move pins; `price_catalog_test::a_move_entry_notices_inside_seven_to_thirty_days` |
| D14 ⚑ | a no-proration change leaves the paid period alone, in Stripe's test mode | T0r, S2 | T0r's answer 9; S2 (Quinn reads the upcoming invoice) |
| D15 | checkout records the server's terms version; no build refused for its age | T1, T4a | `billing-checkout::a_changed_offer_is_refused`; `account::checkout_url_at_sends_the_offer_it_was_given` |
| D15 | `site.yml`: by hand, `main` only, restore first, privacy refused, never self-hosted | T10 | the six `workflows` pins |
| D15 ⚑ | a site-only run keeps the installer and `latest.json` live | T10, T9 | T10's preview run; §8 C step 14 |
| D15 ⚑ | a catalog change reaches an unchanged build | S2 | S2 (b) |
| D16 | the runbook covers every step of D16 | T11 | `reviewer`, row by row; S2's lessons in it |
| D11 | the terms text; no gift paragraph; no old-subscriber paragraph; the price-change paragraph | T5 | `site::the_terms_state_the_signed_prices`, `::the_terms_carry_the_interim_terms_clauses`, `::the_terms_promise_no_code_this_release_lacks`, `::the_terms_carry_the_price_change_paragraph`; `reviewer`; Quinn reads the terms |
| D11 | the version tie; `subscribed.html` | T5 | `bundle_config::the_terms_version_constant_is_the_published_terms_date`; `site::the_subscribed_page_names_fourteen_days` |
| D11 ⚑ | existing accounts | T0, T5b, T9 | the ledger's count; §8 C step 12, or T5b's tests |
| D12 | Link on the privacy page; one key; the tie | T5 | `site::the_privacy_page_names_link`; `ui_account_privacy::the_panels_changes_name_link`, `::the_panels_line_one_names_privacy_version_and_the_table_has_one_key`; `bundle_config::the_privacy_version_constant_is_the_published_pages_date` |
| §4 ⚑ | the ruling's premise: no paying subscription that is not Quinn's | T0 | the ledger's counts |
| §4 ⚑ | production parity before step 10 | T0, T9 | the ledger's migration list; Q2 done |
| §4 ⚑ | Quinn's own old-price subscriptions ended, after their comp grants | T9 | §8 C's own-subscriptions step; step 7's counts |
| §4 ⚑ | rollout order; the deploy proven live by its bodies and deliveries; 10b's card always cancelled | T9 | §8 C steps 7-14, each recorded; 10b's `canceled` read-back |
| §4 ⚑ | the tag publishes the ported site plus T5's pages | T9 | §8 D's `site/` line |
| §5 | 18+, tester and gate unchanged | T1 | the existing tests, unchanged |
| §5 | the webhook's guards | T2 | the existing tests, unchanged |
| §5 | the wording rule | T1a, T1, T3, T4, T5 | the formatter, `custom_text`, reminder, wizard and terms pins; the source scan |
| §5 ⚑ | rule 1; rule 2; no ids in reports | all | T8; the S1 report's rule (§8 B) |

## 7. Review checkpoints

| Checkpoint | When | Who | Passes when |
|---|---|---|---|
| Note and plan review | done: r1 FAIL, r2 PASS with changes, r3 applies them | `reviewer`; the controller checks r3 against r2's list before T0; then Quinn | the note's §8 settled by Quinn |
| r4 delta review | after r4 | `reviewer`, on the note's §10 rows and the plan's §12 only | PASS; what stays open goes to Quinn as a short list |
| **A** | after T0r | main session | every question answered; T1's and T3's briefs written; a refused load-bearing key, or a no-proration change that touches the paid period, goes to Quinn |
| Cloud review | T1a-T3 green | `reviewer` | PASS, with red-first visible |
| Offer command | T4a green | `contract-reviewer` | PASS |
| **B** | after S1 and S2 | main session; **Quinn** if a gate failed | the gates recorded; Q10 settled; S2 recorded |
| App review | T4 green | `reviewer` | PASS |
| Publishing | T10 green | `contract-reviewer` | PASS; the preview run recorded |
| Legal review | T5 committed | `contract-reviewer`; `reviewer` (text, with T6) | PASS; **Quinn reads the pages** |
| Runbook | T11 written | `reviewer`, against note D16 row by row | PASS |
| **C** | T4-T7 merged (T10 and T11 too, if ready) | `reviewer` (T8) | PASS |
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
- [ ] Set `STRIPE_PRICE_FOUR_WEEKLY_1299` and `STRIPE_PRICE_YEARLY_8437` on staging.
- [ ] Deploy the three functions from the integration branch after T1a-T4a (`--use-api`).
- [ ] **B1.** Build the dev app from the integration branch after T4 and T4a (staging environment, a new
      scratch profile). Its offer carries `terms_version` 2026-09-10, the catalog's until T5.

**B. Part 1: S1 and the staging proof, on the T4 build**
- [ ] **S1 (a) and (b), account A** (a fresh scratch account, signed in through the T4 build):
  1. Open `yearly` and `four_weekly` checkouts. A headless read shows:
     - the price, and "every 4 weeks" or the yearly wording;
     - "14 days", the tax line and D8's sentence;
     - "Sold through Link" if Managed Payments is in effect.
  2. Quinn completes `four_weekly` with a test card.
  3. Then, read-only:
     - `four_weekly`, `trialing`, a period end about 14 days out;
     - the consent row carries 1299 and the offer's `terms_version`.
- [ ] **B0 (r4), before S1 (a).** Account A in the T4 build reaches Subscribe with the network up: the
      buttons carry the offer's lines. Then, with the staging function unreachable (a bad `KNOWLU_API_BASE`
      for that launch only), the buttons are disabled and the retry sentence shows; Try again with the real
      base draws the lines.
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
      No consent row is added. (r4) A `four_weekly` POST with `amount_cents` 1298 answers 409 with D3's
      "changed" sentence, and no consent row is added.
- [ ] **B6.** The upgrade sheet on a signed-out scratch install:
      - after sign-in, both plans appear with their lines;
      - a click on the lines starts nothing.
- [ ] **The S1 report** (`docs/reports/2026-11-2x-v041-s1-managed-payments.md`, under the rule above):
  - what Link sends, and when;
  - the receipt's sender;
  - whether a test refund returns the tax;
  - what the portal offers.

**B. S2, the price-change rehearsal (r4; after S1, on the same T4 build, unchanged)**
- [ ] **S2 (a), setup.** Quinn creates a test-mode price on the product at an amount other than 12.99 (an
      invented one; it never goes live), week × 4, tax-exclusive. The main session sets its
      `STRIPE_PRICE_FOUR_WEEKLY_<CENTS>` on staging. Account C (a fresh scratch account) completes a
      `four_weekly` checkout at 1299 with a test card; it is `trialing`, renewing about 14 days out.
- [ ] **S2 (b), the change reaches the app.** On a scratch branch `v041/s2-rehearsal`, never merged, the
      catalog's only change: the new amount becomes `four_weekly`'s `SELLING` entry, and 1299 moves to `KEPT`
      with `stay`. Deploy the three functions from it. With **no rebuild**, the T4 build shows the
      new figure on Subscribe and on the sheet, and Stripe's test page matches. A POST carrying the old
      offer (1299) answers 409 with D3's "changed" sentence.
- [ ] **How the job is run by hand.** In staging's SQL, run the `knowlu-billing-jobs` cron command once as it
      stands (its URL and token come from Vault, `20260910000600_billing_jobs_vault.sql:21-34`, guarded by
      `000700`), so no token is copied anywhere; read the reply's counts from `net._http_response`. The exact
      statement is `unverified`: the main session copies it from `cron.job` at the time.
- [ ] **S2 (c), stay first.** With 1299's `KEPT` rule set to `stay`, run the job: account C gets no mail, and
      its subscription is unchanged.
- [ ] **S2 (d), then move.** Set the rule to `move`, N = 10, on the scratch branch; redeploy `billing-jobs`;
      run the job. Account C (renewing 10 to 16 days out) gets one notice naming both amounts and the date.
      Quinn reads the test subscription: the metadata mark, the new price, and an upcoming invoice at the new
      amount with no proration for the period in progress. A second run sends no second notice and changes
      nothing more. `entitlements` still reads `four_weekly`.
- [ ] **S2 (e), back.** Redeploy the three functions from the integration branch. Quinn cancels account C's
      subscription and archives the invented test price; the main session unsets its variable. The S1
      report gains an S2 section (counts and wording only), and the runbook lessons go to T11.

  Then checkpoint B.

**B. Part 2: after T5, on the PR head**
- [ ] Redeploy the three billing functions, so the catalog's `TERMS_VERSION` is the new terms date. Build
      the dev app from the PR head.
- [ ] **B8.** A fresh scratch account opens both checkouts. The consent row carries the new terms version.
      An offer read before the redeploy (carrying 2026-09-10) is now refused with D3's "changed" sentence.
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
- [ ] **Quinn's own subscriptions** (Q9, ruled by (6); each act on Quinn's word), for **each of Quinn's
      accounts that holds an old-price subscription**:
  1. `admin.comp_grant` (an `ends_at` within a year), one production write per account.
  2. Quinn cancels its old-price subscription **immediately**, in both modes.
  3. Read back the account's view row: `tester`.
  4. Quinn archives the old prices in both modes once no subscription uses them (the checklist).
- [ ] **7.** Re-run T0's queries. The counts must equal T0's, except:
      - no `active` or `trialing` `monthly` or `academic_year` row remains;
      - the grants above are added.

      Anything else stops the rollout and goes to Quinn. A new account that is not Quinn's makes T5b
      required.
- [ ] **8.** The PR merges. Record the SHA.
- [ ] **9.** Quinn's live-mode checklist. Quinn reads back the live endpoint's enabled events, and the main
      session records whether `customer.subscription.trial_will_end` is present.
- [ ] **10.** Set `STRIPE_PRICE_FOUR_WEEKLY_1299` and `STRIPE_PRICE_YEARLY_8437` on production, and deploy
      the three functions (and `account`, if T5b ran) from the merge SHA. Quinn reads the endpoint's
      deliveries since the deploy: every one is 2xx.
- [ ] **10b. The pre-tag live proof**, on a fresh production scratch account (an address Quinn supplies;
      sign-in by emailed code; `POST /account/consent` with the 18+ attestation and the new versions):
  - The `monthly` and "2026-09-10" POST answers **409 with D3's exact sentence** in `error`. The tester
    sentence fails this step.
  - (r4) `GET /billing-checkout` answers both plans, Yearly first, with the catalog's lines and the new terms
    version. A `four_weekly` POST with `amount_cents` 1298 answers 409 with the "changed" sentence.
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
      offers 0.4.1 to a v0.4.0 install. Then unset `STRIPE_PRICE_MONTHLY` and `STRIPE_PRICE_YEAR` on both
      projects (r4).
- [ ] **14. T10's first live run** (r4; after T10 merges and its preview run passed; on Quinn's word):
      dispatch `site.yml` on `main` with `site/` unchanged. Then knowlu.com serves the same pages,
      `/releases/Knowlu-setup.exe`, the versioned installer and `latest.json`, and the updater still offers
      0.4.1. If any is missing, re-run the tag's `publish` job (`gh run rerun <id> --failed` restores from the
      build artifact while it lasts, `release.yml:262`) and stop: `site.yml` is not used until fixed.

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
- [ ] Q13 is done. Q14 was answered before T5 (the terms say what it rules).
- [ ] (r4) S2 is recorded, and every staging function serves the lane's catalog (its offer carries no
      invented figure).
- [ ] (r4) Production's offer (step 10b) states exactly the figures and version of `site/terms.html` at the
      merge SHA, the page the tag publishes.
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
| A Dashboard price is wrong | D4's 500, on staging or at step 10b | Quinn corrects it with a new Price at the same amount; the derived variable is set to the new id (the only time one is re-pointed: nothing was sold on the old). |
| An unset price value on the webhook | the deliveries at step 10 | It cannot answer 500 (D2). An unknown id shows as `plan: null` and as a `billing-jobs` failure. |
| T0 finds a paying subscription that is not Quinn's (r4) | T0's counts, or step 7 | Stop. Ruling (6)'s premise is false; the lane goes back to Quinn before anything is built on it. |
| The catalog and the terms page disagree (r4) | `price_catalog_test::the_terms_state_the_catalogs_prices` in CI | The PR cannot merge. Never fixed by editing the test. |
| An amount creeps back outside the catalog (r4) | `price_single_source_test::no_amount_is_written_outside_the_catalog` in CI | The PR cannot merge; the amount moves into the catalog. |
| Stripe's no-proration change touches the paid period (r4) | T0r question 9, or S2 (d)'s upcoming invoice | Checkpoint A or B: the move is not built or not shipped (T3's stop rule), `stay` alone ships, and Quinn is told. D14's text is amended within the open round. |
| A move's mark fails after its notice was sent (r4) | `failed` in the job's reply; a second notice the next day | Accepted: the student gets the same notice twice, never a change without one. The run log names the account id only. |
| The offer cannot be read (r4) | B0; `checkout_offer` errors | The plan buttons wait and say why (D9); nothing sells on a stale figure. A staging outage blocks S1 until fixed. |
| S2's scratch functions left deployed (r4) | checkpoint B | S2 (e) redeploys the lane's; checkpoint B reads each function's offer, and an invented figure stops it. |
| Someone runs `wrangler pages deploy site` by hand (r4) | knowlu.com's `/releases/Knowlu-setup.exe` or `latest.json` 404 | Re-run the tag's `publish` job, or `site.yml` once T10 has merged. T6 writes the warning into `docs/reference/app.md`. |
| `site.yml` drops the installer on its first run (r4) | §8 C step 14's check | Re-run the tag's `publish` job (`release.yml:262`'s artifact, while it lasts) and stop using `site.yml` until fixed. Its preview run comes first so this is caught off the live site. |
| The waitlist lane's site-only deploy (W-D3) and T10 collide (r4) | T10 start | T10's stop rule: one workflow, the restore, the narrowed guard. |
| Counts change between T0 and step 7 | step 7 | Stop and tell Quinn. A new account that is not Quinn's makes T5b required. |
| The reminder does not arrive in 10b | 10b step 4 | Quinn cancels at 30 minutes anyway, and the `canceled` read-back comes before `DELETE`. The gate is recorded as failed, and Quinn decides at D whether to hold the tag. |
| The site port has not landed | T5 start, or §8 D | Stop. The tag would replace the live site with an unported `site/`, and T5 and the port both write `site/*.html`. |
| Bump A is open | T5 | T5's stop rule: one privacy key. |
| A task needs another contract-list file | any | Stop, amend the plan, and `contract-engineer` makes the change. |
| The window between step 10 and the tag | by design | Accepted. Sales are closed, and the only install is Quinn's. The tag follows the same day. |
| `release.yml` fails after step 10 | the tag's run | Fix forward on a new tag. Quinn may roll the three functions back to the pre-merge SHA, because the old functions and the old published terms are a coherent pair; the old price variables stay set until step 13 for this reason (r4). |
| A Stripe fact proves false after use | any | Amend the note within the open round, never a third round. |
| The Stripe account changes owner mid-lane | Q4 | The new-account branch adds new secrets, a new endpoint, product, prices and portal, cleared customer ids, and S1 again. Recommend deferring it. |

**Two failed attempts** move one level up, effort before model. `max`, and Fable for a building role, are
Quinn's call.

## 10. Budget

- **Agent runs.** About seventeen, plus fixes (r4 adds five):
  - one `researcher`;
  - one or two `cloud-engineer` (T1a-T3, one run);
  - two `console-ui` (T4, and T5's drafts);
  - one `test-writer`;
  - two `contract-engineer` (T4a, and T5's commit);
  - one `implementer` (T10);
  - two `docs-keeper` (T6, T11);
  - five `reviewer` passes (cloud, app, text with docs, T11's runbook, and the whole branch) and three
    `contract-reviewer` (T4a, T5, T10);
  - the r4 delta review of the note and plan.

  T5b adds about three.
- **Fable** is used only by the planner.
- **Quinn's time.**
  - The twelve items of the note's §8, most of them one word each.
  - The test-mode checklist, about 30 minutes.
  - S1, about an hour.
  - S2, about an hour (a test price, one test checkout, reading the subscription and its upcoming invoice).
  - Reading the pages.
  - The live-mode checklist, about 20 minutes.
  - Step 10b, about 30 minutes with the round trip.
  - About nine production words: T0's read, parity, Quinn's own subscriptions, step 7, the merge, step 10,
    step 10b, the tag, and step 14.
- **Emailed codes on staging.** Four sign-ins (accounts A, B and C, and the sheet's scratch install). Plan
  across two hours. Production's step 10b needs one more.
- **Disk.** At most four live builds. Delete `target/` after each merge.

## 11. Ambiguities, and how this plan reads them

1. **"Existing `academic_year` rows map to the yearly plan."** Answered 2026-11-20 by ruling (6): no
   subscription is kept at an old price, under any name, so there is nothing to map (D1).
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
7. **"Monthly" as a plan word.** Moot in r4: no row will carry `monthly`, so Settings has no word for it.
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
    names Quinn's account means each of them: the T5b trigger, the grant and cancel of §8 C's
    own-subscriptions step, and step 12's acceptance. Quinn names them, and the ledger records counts.
13. **The terms version at checkout.** r3's floor is gone. Checkout records the version the server names,
    so a terms bump refuses no build (note D3, D15). Sign-up still records the compiled `TOS_VERSION`, which
    lags a bump published without a release until the student updates; Quinn confirms that (Q14).
14. **"Without an app release" (ruling 7).** It means the app needs no rebuild for a price change: the
    figures and the checkout terms version come from the server, and the two keys never change. The compiled
    `TOS_VERSION` still moves in the change's PR and reaches students with the next ordinary release (D15).
15. **"Lower or raise."** Both follow D16's steps. A cut under `move` lowers the renewal; under `stay` the
    student keeps paying the old, higher price until they cancel. The amendment for a cut should say which
    it means; the note does not choose (D14).

## 12. Revision 2026-11-20 (Quinn's ruling)

r4 follows the note's §10. A delta review reads the rows below and the note's §10, nothing else.

| Where | r4 change |
|---|---|
| Header, Size, Goal, Architecture, "Read for r4" | Spec r4 (D1-D16); the price catalog, the offer, the move, `site.yml` |
| §1 | `account.rs` and `main.rs` on the list for T4a; `.github/workflows/site.yml` for T10 |
| §3 | Q1, Q3 and Q9 answered; Q14 new; T0's premise check; T0r questions 9-11 (no-proration change, archived price renewal, subscription metadata) |
| §4 | Rows T1a, T4a, S2, T10 and T11 added; T1-T6 rows re-scoped; the order with W-D3 |
| T0 | The counts now check ruling (6)'s premise |
| T0r | Questions 9-11; checkpoint A stops the move if question 9 fails |
| **T1a** (new, `cloud-engineer`) | The catalog module and its six rules, red first |
| T1 | The offer (`GET`), the offer match replacing the floor and the shape check, the catalog through `Deps`, an unset variable in D4 |
| T2 | The plan lookup and the reminder amount through the catalog |
| T3 | Renamed "Billing jobs"; the pause and resume removed; the annual reminder from the subscriber's own price; the move; the source scan in its own file |
| **T4a** (new, `contract-engineer`) | `checkout_offer`, the offer in `open_checkout`'s body, both handler lists |
| T4 | The app holds no price and draws the offer; buttons wait without one; no old plan words; the figures-equal-cents test removed |
| S1 | After T1a-T4a; B0 (the offline check) and B5's changed-offer case |
| **S2** (new, main session) | The rehearsal: a test price, `stay`, then `move`, with no rebuild |
| T5 | The catalog's `TERMS_VERSION` in the commit; the terms tie test; the price-change paragraph; no old-subscriber paragraph; three functions redeployed after |
| T6 | README's Billing section (catalog, derived names, offer, move); `app.md`'s warning about a hand site deploy; C25's marker |
| **T10** (new, `implementer`) | `site.yml` and its six pins |
| **T11** (new, `docs-keeper`) | `docs/reference/price-change.md` from D16 |
| §6 | D1-D4, D6, D9-D11 rows revised; D13-D16 rows added; the `academic_year` and P10 rows removed; §4's premise and own-subscription rows added |
| §7 | The r4 delta review; T4a and T10 to `contract-reviewer`; checkpoint B includes S2 |
| §8 | Variable names `_1299`/`_8437`; B0; S2; Quinn's own subscriptions (was Q9) with archiving; 10b's offer checks; step 13 unsets the old variables; step 14, `site.yml`'s first live run |
| §9 | Ten rows added (premise, catalog drift, source scan, proration, duplicate notice, no offer, S2 left deployed, hand site deploy, `site.yml`'s first run, W-D3); the rollback row keeps the old variables until step 13 |
| §10 | Five more agent runs; S2's hour; one more sign-in; step 14's word |
| §11 | Item 1 answered; item 7 moot; item 13 rewritten; items 14 and 15 new |

**What r4 removed from the plan:** `termsFloor`; the floor, shape and "2026-11-19x" tests; the
`CHECKOUT_TERMS_VERSION` constant and its text-read; `priceOf`'s two new entries; the pause tests kept for
`monthly`; `ui_plans::the_shown_prices_are_the_charged_cents` and `::the_sheet_states_the_same_prices`
(nothing to compare: the app holds no figure); the four price lines written into `wizard.js`.

**What still needs Quinn before work starts:** Q14 (the note's §8 item 14), with Q2, Q4, Q8, Q10 to Q13 as
before. Q5 to Q7 are settled when Amendment 2026-11-17 is signed.
