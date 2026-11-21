PASS with changes

# v0.4.1 pricing: design note and plan review, round 2 (final)

Reviewer (spec/plan mode), 2026-11-19. Branch `docs/v041-pricing` at `664a8d82`. Diff reviewed:
`ca9bd232..664a8d82 -- docs/specs docs/plans`. Documents: `docs/specs/2026-11-19-v041-pricing-design.md`
("the note") and `docs/plans/2026-11-19-v041-pricing-plan.md` ("the plan").

**Verdict: PASS with changes.** The r1 Critical and all seven r1 Importants are closed in the text and in
the tasks and gates, not only in §9. M3 is closed in name only (finding 6). The revision introduced nine
new defects. None needs a redesign. Seven are one-to-three-sentence fixes the planner can make now without
a third round (marked **fix now**). The rest go to Quinn (marked **Quinn decides**, collected at the end).

The money model is still right: 1299 and 8437 cents, and 8437 / 13 = 649. The plan is mapped from the
price id, and the old keys are kept for P10. D3 refuses before the account read. The functions deploy
before the tag, and the terms publish in the same deploy as `latest.json`.

## r1 findings: closure check

| r1 | Closed? | Where |
|---|---|---|
| C1 parity | Yes. A scope gap is new (finding 3). | note §4 step 0; plan §3, T0, §8 C P-parity, §9 rows 1-2 |
| I1 A2 vs P9 | Yes | note "When rule A2 ends", D9; plan T4; §8 item 3 with the price-free branch |
| I2 MP already on | Yes. The reading (ii) consequence is new (finding 1). | D7 table; §3 row 1; checklist "Confirm"; T0r q2 |
| I3 load-bearing keys | Yes. The test wording clashes with D5 (minor m4). | D7 lists; checkpoint A; T1 `the_form_keeps_every_load_bearing_key` |
| I4 Q2/step 13 | Yes | D3 first; 10b asserts the exact body; a fresh scratch account; grant, then cancel immediately |
| I5 live reminder | Yes, with a stated fallback | step 9 read-back; step 10 deliveries; 10b; §8 D |
| I6 LLC branches | Yes. The timing needs refining (Q9 below). | §8 item 9; T0r q8; §9 last row |
| I7 three readings | Yes | §8 items 2, 4, 5 |
| M1 tax_behavior | Yes. The interval per plan is a new gap (finding 2). | D4; T1 case 6 |
| M2 reminder | Yes | D6; T2 |
| M3 terms shape | **No** (finding 6) | D3 item 2 |
| M4 parallel checkout | Partly (minor m1) | D5 |
| M5 S1 gaps | Yes. A sequencing defect is new (finding 4). | §8 B |
| M6 optional prices | Yes | D2; T2 `a_missing_price_setting_matches_nothing` |
| M7 P1 split | Yes | Q1 blocks T1; Q12 blocks the tag |
| M8 100% code | Yes | checklist |
| M9 public report | Yes | §8 rules |
| M10 docs merge | Yes | plan §2 |

I checked these code claims again and found them accurate:
- `billing-checkout/handler.ts:51-72, 92-107, 109-127` and `index.ts:16-20, 35-42`;
- `stripe-webhook/handler.ts:64-67, 92-94, 105-125`;
- `billing-jobs/handler.ts:69-75, 119-137, 173-175` and `index.ts:26-36`. `EMAIL_*` is read lazily inside
  `sendEmail`, so T2's copy of that shape cannot take the webhook down;
- `20261020000500_testers.sql:76-87`, where a grant is matched only when the Stripe status is not active
  or trialing. Q7's order, grant first and then cancel, is therefore right;
- `20261020000600_comp_guards.sql:2-12`;
- `account/handler.ts:199-201`;
- `account.rs:24, 210-217, 1611-1621`;
- `release.yml:294-297`;
- `HANDOFF.md:58, 63, 70, 80, 598, 629`;
- the `#up-subscribe` pins (`ui_account_privacy.rs:66, 105, 333`, `t20.py:94, 115`), which T4's wrapper
  keeps green.

## Findings

### Important

**1. Under reading (ii), Link is already the seller, but the terms line and D12 are gated on "the D7
change ships".** Fix now, plus a Quinn decision folded into Q8.
- *Where:*
  - note D7 table row (ii) (`:215`: "The form needs no change");
  - D11 "Sold through Link, only if the D7 change ships" (`:309`);
  - D12 heading (`:324`);
  - the fallback (`:250-252`: "the terms say 'Stripe'");
  - §8 item 8;
  - plan checkpoint A (`:233-234`: "only D12's question remains").
- *Conflicts with:*
  - D12's own premise: Link receives the card, address and email, and the list of companies is closed
    (`site/privacy.html:86`);
  - CLAUDE.md's privacy rule, that each data class is on the page.
- *Failure scenario:*
  1. T0r confirms reading (ii).
  2. T1 makes no form change, so "the D7 change" never ships.
  3. Checkpoint B treats D12 as optional. If any S1 gate fails, the recommended fallback is taken.
  4. v0.4.1 publishes terms that say "Stripe" and a privacy page with no Link, while every live session
     is sold through Link.
  5. Under (ii), the "fallback" is not available at all without switching Managed Payments off at the
     account level in live mode. Nothing in the documents names that switch.
- *Fix:*
  - Condition D11's Link line and D12 on **"Managed Payments is in effect"**, meaning reading (ii), or
    reading (i) with the change.
  - Under (ii), the fallback becomes: Quinn turns the account setting off before step 10, and step 10b
    reads that the page does not say "Sold through Link".
  - Q8's text gains that branch.
  - T5's "with D12" conditions follow.

**2. D4's interval check does not tie the interval to the plan.** Fix now.
- *Where:* note D4 (`:143`: "the interval is `week` × 4 or `year` × 1"); plan T1
  `the_price_is_checked_before_it_is_sold`, whose cases are `month` and count 1 only.
- *Failure scenario:*
  1. In the live Dashboard, Quinn creates the $12.99 price with "Yearly" picked by mistake.
  2. Its id goes into `STRIPE_PRICE_FOUR_WEEKLY`.
  3. D4 sees `active`, `usd`, 1299, `year` × 1 and `exclusive`. All pass under the "or" reading.
  4. Checkout sells $12.99 a year under terms that say every 4 weeks.
  5. The webhook maps by id, so the row says `four_weekly`. Nothing downstream notices.
- *Fix:*
  - "The interval matches the plan: `four_weekly` is week × 4, `yearly` is year × 1."
  - Add a T1 case: `four_weekly` names a `year` × 1 price at 1299, and the answer is 500.
  - While there, D4 says "six things" over five bullets. Count interval and count as two checks.

**3. The parity push has no upper bound, and its recommendation assumes it is small.** Fix now (text),
Quinn decides (Q10).
- *Where:* note §4 step 0 (`:364-367`, "run through those stamps … `db push --include-all`"); plan §3
  (`:118-121`) and §9 row 2.
- *Conflicts with:*
  - `supabase db push --include-all` pushes **every** pending local migration. It has no "through"
    option. L9 (`HANDOFF.md:598`) runs it "from a scratch copy of `cloud/supabase`" precisely so the set
    is chosen.
  - L9's step (2) puts the Vault entries in place before email's migrations "so the cron does not
    raise", and has its own function redeploys.
- *Failure scenario:*
  1. T0 finds testers missing.
  2. The main session runs `db push --include-all` from the integration worktree.
  3. Production also receives `20261023000100`, `20261109000100_definer_trigger_execute.sql` and
     `20261111000100_client_table_rights.sql`, which nobody reviewed for production. If email's
     migrations were also missing, it receives those too, and their cron fires without the Vault
     entries.
  4. Only "L8's `entitlement` redeploy" follows, not the L9 function set.
- *Fix:*
  - The push runs from a scratch copy whose `migrations/` ends at `20261020000600_comp_guards.sql`.
  - If T0's list shows anything missing that needs a secret, a Vault entry or a function redeploy beyond
    `entitlement` and `billing-checkout`, the precondition is L9 proper. v0.4.1 waits, or Quinn rules.
  - Q10's recommendation says so.

**4. S1 cannot run where the plan places it.** Fix now.
- *Where:* plan §4 S1 "After: T1-T3 merged; P7" (`:168`); §8 B, where S1 (e) step 1 is "Account B walks
  the wizard to Subscribe (proof B2 below)" (`:524`) and B4 is "Account A, **before its S1 (d) cancel**:
  Settings reads 'Active, Every 4 weeks'" (`:541`).
- *Conflicts with:*
  - B1 builds the app "from the PR head". T4 (the wizard buttons and the `four_weekly` Settings word)
    runs beside S1, not before it.
  - T5, and so the final build, comes after checkpoint B, which comes after S1.
  - Today's wizard sends `monthly`/`academic_year`, which T1's D3 answers with 409.
- *Failure scenario:*
  1. At S1 (e) there is no build whose wizard can reach a checkout.
  2. At S1 (d), account A is cancelled before any build can show "Active, Every 4 weeks", so B4 can
     never be observed on account A.
- *Fix (either):*
  - S1's After becomes "T1-T4 merged". T4 does not depend on S1. S1 (e) and B4 then use a T4 dev build,
    and the floor is still 2026-09-10, which that build sends.
  - Or S1 (e) opens `yearly` by direct POST, and B4 moves to a fresh account in the post-T5 proof.

**5. "More than one account adds T5b" fires on Quinn's own two accounts.** Fix now.
- *Where:* plan T0 (`:222`) and §9 "A new account makes T5b required"; note D11 (`:320-322`: "if Quinn's
  is the only account … if any other account exists").
- *Conflicts with:* `HANDOFF.md:58`: "Both prod accounts that accepted the 2026-09-16 version are
  Quinn's."
- *Failure scenario:*
  1. T0's count is 2.
  2. The plan's rule adds T5b. That is a contract-list change to `account.rs`, an `account` function
     change and the re-consent panel, in a lane that needs none of it.
  3. Or the controller improvises, and step 12 records acceptance for one of Quinn's two accounts.
     Q7 likewise grants only "your own account".
- *Fix:*
  - The rule is "any account that is not Quinn's". Quinn identifies which accounts are theirs; the ledger
    records counts only.
  - Step 12 records acceptance on each of Quinn's accounts.
  - Q7 grants and cancels per account that holds a subscription.

**6. M3 is not closed: D3 cites the account function as a shape check, and it is not one.** Fix now.
- *Where:* note D3 item 2 (`:113-115`, "a `YYYY-MM-DD` date … the same rule and sentence the account
  function uses"); plan T1 `a_malformed_or_future_terms_version_is_refused`, with cases "zzz",
  "9999-99-99" and the day after tomorrow.
- *Conflicts with:* `account/handler.ts:199-201`, which only refuses a string that sorts after tomorrow.
  The sentence names a shape that the code never checks.
- *Failure scenario:*
  1. An implementer copies "the same rule".
  2. All three T1 cases fail the `> tomorrow` comparison, so the test passes.
  3. "2026-11-19x", "2026-9-30" and "2026-10-01 " all sort between the floor and tomorrow. They pass and
     are written into `consents.version`.
- *Fix:*
  - Require `/^\d{4}-\d{2}-\d{2}$/` before the comparison.
  - Add one in-range malformed case, for example "2026-11-19x".
  - Drop "the same rule" from D3's wording.

**7. T5's one commit breaks T1's text-read pin.** Fix now.
- *Where:* plan T1's `index.ts` text-read pins `CHECKOUT_TERMS_VERSION = "2026-09-10"` (`:263`). T5's
  Owns column has "the `CHECKOUT_TERMS_VERSION` line in `billing-checkout/index.ts`" but not
  `handler_test.ts` (`:170`).
- *Failure scenario:*
  1. `contract-engineer` makes T5's commit.
  2. `deno test` in CI fails on T1's pin.
  3. Either a file outside T5's Owns column changes, which is a T8 finding, or the commit lands red.
- *Fix:*
  - T1's pin checks only that the constant is declared once, as a quoted `YYYY-MM-DD` literal.
    `bundle_config` already ties its value to `TOS_VERSION`.
  - Or T5 owns that one assertion.

**8. The tag's site deploy collides with the in-flight site port.** Quinn decides the order; the gate
line is fix now.
- *Where:* note §4 steps 11 and 13; plan §8 C 11, 13 and §8 D; T5 owns `site/terms.html`,
  `site/subscribed.html` and `site/privacy.html`.
- *Conflicts with:*
  - `release.yml:294-297` runs `pages deploy site --project-name knowlu --branch main`, which replaces
    the whole live site with the tagged `site/`.
  - The 11-17 record says knowlu.com is now a hand upload that a `v*` tag reverts until it is ported.
  - Branch `site/port-live-1119` exists, at `04c5f85f` with no commits of its own yet.
- *Failure scenario:*
  1. v0.4.1 is tagged before the port merges.
  2. The deploy publishes the new terms and also puts the old repository homepage back over the live one.
  3. Or the port merges after T5 and rewrites the three legal pages T5 composed. That conflicts on
     `site/*.html` and on the `site.rs` pins.
- *Fix:*
  - §8 D gains "`site/` at the merge SHA is the live site plus T5's pages (the port has landed)".
  - The plan names the port as a lane to sequence with T5.
  - Whether v0.4.1 waits for the port, or the port rides v0.4.1, is Quinn's call.

**9. 10b's reminder round trip has no stop condition if the reminder never arrives.** Fix now.
- *Where:* note §4 10b (`:403-410`); plan §8 C 10b (`:571-576`). The steps run "3. move the trial end
  within 3 days, 4. the reminder arrives, 5. Quinn cancels immediately".
- *Failure scenario:*
  1. The reminder is what is under test, and it does not arrive.
  2. Step 5 is written after step 4, so nobody cancels.
  3. If the move put the trial end hours away, or the session stalls before `DELETE /account`, Quinn's
     card is charged $12.99 plus tax.
  4. The charge is under terms not yet published: knowlu.com still shows $9.99 and seven days until
     step 11.

  `DELETE /account` is a backstop, because it sets `cancel_at_period_end` and for a trialing subscription
  the period end is the trial end (`account/handler.ts:88-94`). But it only helps if it runs before the
  moved trial end.
- *Fix:*
  - Move the trial end no closer than 24 hours.
  - Cancel immediately once the reminder arrives, or after a fixed wait (for example 30 minutes)
    without it. Record the miss as a failed gate.
  - Read back `canceled` before `DELETE`.

### Minor (all fix now unless marked)

- **m1. D5 still allows a second subscription beside a `past_due` one.** D5 claims "one subscription at a
  time" (`:152-158`), but only `active` and `trialing` are refused. A student whose card failed opens a
  new checkout and gets a second subscription while the first is still dunning. Account deletion cancels
  only the stored id (refund case (a)). *Fix:* refuse `past_due` too, with a sentence pointing to Manage
  subscription. Or reword D5 to "no second live subscription" and accept the gap.
- **m2. The equal tie (`CHECKOUT_TERMS_VERSION == TOS_VERSION`, bundle_config) makes every future terms
  bump refuse every older build at checkout.** That includes typo fixes. It may be intended, but D3 and
  D11 do not say so. *Fix:* state it as a consequence. Or tie with `<=`, and move the floor only when the
  money terms change.
- **m3. The terms date has a bound nobody states.** Checkout's new check and the production account
  function both refuse a version later than tomorrow. If T5 dates the terms on a planned publish date
  more than a day ahead, the staging proof and 10b both fail. *Fix:* T5's date is no later than the day
  after the staging proof. The commit date is the simple choice.
- **m4. T1's load-bearing test contradicts D5.** "D7's seven load-bearing keys are present" includes
  `trial_period_days`, which D5 omits on a second subscription. *Fix:* "present (the trial key on a first
  subscription)".
- **m5. The new wizard ban covers less than the old one.**
  - T4's "Nowhere in `wizard.js`: … 'trial'" drops the old test's `picker.js` coverage.
  - Read file-wide, it fails on the existing `"trialing"` comparison (`wizard.js:162`).
  - *Fix:* keep `string_literals` over both files, with the `"trialing"` exception.
- **m6. Clicking the price lines would call `subscribe()`.** `#up-subscribe` becomes a wrapper, but the
  click handler is `closest("#up-subscribe")` (`account.js:214`). A click on the lines would start
  checkout with no plan. *Fix:* T4 binds the two buttons. A click elsewhere in the wrapper does nothing,
  and `ui_plans` pins that.
- **m7. The note still uses r1's "Q2" label.** It appears in §4 ("Q2, if Quinn agrees") and in the
  checklist ("once Q2's comp grant exists"). The plan's Q2 is `academic_year`. *Fix:* "item 7"
  throughout the note.
- **m8. A deletion citation is wrong.** The note cites `account/index.ts:90-96` for the cancel on
  deletion (`:166`, `:248`). The cancel is `account/handler.ts:88-94`, and it is `cancel_at_period_end`,
  not immediate. S1 (e)'s "The subscription is cancelled" should read "set to cancel at period end, no
  invoice".
- **m9. Q1's amendment should also record item 4.** Item 4 replaces a 2026-09-14 ruling.
- **m10. 10b leaves permanent production records against unpublished text.** Each checkout open writes an
  `auto_renew` consent row carrying the new terms version, before that version is published.
  `POST /account/consent` does the same for the new privacy version under D12. Deletion keeps the
  consent row, with the account id nulled and the hash kept (`account/index.ts:114`). Under Managed
  Payments, Link may keep a record of the scratch address. The account is a founder test account, so
  this is harmless. *Quinn decides:* accept it and say so in the ledger. Recommended.

## The live-mode steps (10b, and Quinn's card)

- **Safe?** Yes once finding 9 is fixed.
  - Checkout charges nothing under the trial, and Quinn cancels the same day.
  - The 409 probe and the headless page reads pay nothing.
  - Deletion is a second backstop.
- **Minimal?** Yes.
  - A fresh account is the only way to see the trial live. Quinn's own accounts have had a subscription,
    and after Q7 they read `tester`, which is refused with 409.
  - Checkout needs an authenticated account, so the live pages cannot be read without one.
- **Reversible?** Mostly. The account and its rows go. What stays is in m10, plus the live Stripe
  customer, which deletion keeps on purpose.
- **Whose hands?**
  - Quinn's: the card, the Dashboard trial move and the cancel.
  - The main session's, on Quinn's word: the emailed-code sign-in, the POSTs, the reads and `DELETE`.
  - No subagent holds a session (plan §8 rules). This matches CLAUDE.md.

## Rollout windows

- **Charged at a price the published terms do not state?**
  - Old builds are refused from step 10 on, by D3 item 1.
  - The new floor means only a client sending the unpublished version can buy between steps 10 and 11.
    No such build is published.
  - The only live completion in that window is 10b's trial, which is cancelled. This holds once
    finding 9 is fixed.
  - Rolling back after a failed `release.yml` restores a coherent pair, old functions with old terms,
    as §9 says.
- **Charged without the reminder?**
  - The first real charge comes at least 14 days after a post-tag checkout.
  - The reminder code is live from step 10, and the event is enabled at step 9.
  - The remaining gap is production `EMAIL_*`, if Q11 is declined and `mail-canary` is not on
    production. The documents route that to Quinn at D, which is correct.
  - Note `HANDOFF.md:594, 597`: which Resend key is live, and the key roll, are still open. A roll after
    10b needs one more send.

## Quinn's list (§8): review

The twelve items are correct, and the right things are on it. Two need reordering, and three
recommendations need a sentence.

- **Order.**
  - Item 10 (parity) is discovered at T0. It gates item 7's grant (no `comp_grants` table without it), not
    only step 10. Move it up to sit after item 1.
  - Item 9 (the LLC) should block P7, the test-mode checklist, not step 9. The new-account branch voids
    S1 and the test-mode setup, which live in the same Stripe account.
- **Item 8:** add finding 1's reading (ii) branch.
- **Item 9:** the recommendation (defer) is sound for the new-account branch. Add two things:
  - updating the business details on the same account costs nothing and could happen before step 9, so
    the merchant name on a charge is not the sole proprietorship's while the pages name KNOWLU LLC;
  - the new-account branch must happen before Amendment 2026-10-16b's gate opens sales. After that, it
    means recreating every paying subscription.
- **Item 10:** recommend "yes **if** T0's list shows only the testers and comp-guard stamps missing.
  Otherwise L9 proper, and v0.4.1 waits for it" (finding 3).
- **Item 11:** recommend yes, with finding 9's stop condition written into the step.
- Items 1-7 and 12: the recommendations are sound (add m9 to item 1).

## For Quinn (decisions still open after this round)

1. **Items 1-12 of note §8**, with the reorder and the three amended recommendations above.
   *Recommend:* as the note says, amended as above.
2. **Reading (ii) and the fallback** (finding 1, folded into item 8). If Managed Payments applies to
   every live session, either ship D11's Link line and D12, or switch Managed Payments off in live mode
   before step 10. *Recommend:* ship Link on the pages. Switching it off reverses a signed choice (R2, U5).
3. **The tag and the site port** (finding 8). *Recommend:* land the port first, or carry it in this PR,
   so the tag publishes the live site plus T5's pages. Never tag v0.4.1 over an unported hand upload.
4. **10b's leftover records** (m10). *Recommend:* accept them, recorded as counts in the ledger.

Everything else marked "fix now" goes to the planner as text edits, with no third review round. The
controller confirms each one against this list before T0.
