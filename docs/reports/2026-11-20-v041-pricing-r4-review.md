PASS with changes

# v0.4.1 pricing: r4 delta review, round 1

Reviewer (spec/plan mode), 2026-11-20. Branch `docs/v041-pricing` at `6e9376aa`. Scope: the rows of note §10 and
plan §12, plus anything they break. The authority is Amendment 2026-11-17, signed 2026-11-20 (`docs/price-amendments`,
cloud design `:1234-1550`). Code claims were checked in this worktree.

**Verdict.** The r4 design follows the signed amendment. There is one catalog, the app shows the server's offer, and
`stay`/`move` respect the two promises. The removals are sound. Nothing moves money wrongly at v0.4.1: `KEPT` is empty
and `move` is unused until a later amendment. Six Important findings need text fixes before T1a, T3, T5 and T10 are
dispatched. One of them (I3) also needs a decision from Quinn. The Minor findings are text fixes. Round 2 checks only
these.

## Important

**I1. The terms tie can never pass against the signed terms text.** The rule is at note §6 `:813-815` and D13
`:473`, and in plan T5 `:611-614`: "every `$` figure in `site/terms.html` is some catalog entry's". The approved terms
paragraph, legal spec `:619`, says "The yearly plan: $6.49 every 4 weeks, billed $84.37 once a year". 649 is no entry's
cents.
- *Failure:* T5's red test stays red after the page is written. The contract-engineer must then either edit the
  test, which breaks plan §9 ("never fixed by editing the test"), or drop P11's signed wording from the terms.
- *Fix (text):* count a figure as the catalog's when it is an entry's cents, or a yearly entry's cents ÷ 13. Better,
  require every figure to be one the catalog's formatter writes. Add a red case: "$6.49" passes and "$6.50" fails.

**I2. S2 tests the move on a subscription that has no paid period, so it cannot prove the main promise.** Plan §8
S2 (a) and (d), `:850-869`, moves account C while it is `trialing`. On a trial, an upcoming invoice with no proration
for the current period looks the same whether `proration_behavior=none` worked or not, because the period cost $0.
The ledger row D14 ⚑ (`:754`) rests "never on a paid period" on this run alone.
- *Failure:* checkpoint B records the move as proven safe. The first live `move` is then the first test of a paid
  period. If T0r's answer 9 is wrong, a live student is charged inside a paid period.
- *Fix (text):* in S2 (d), end account C's trial first in the test Dashboard, so the test card is charged and the
  period is paid. Set N = 22, so the 28-day renewal falls inside 22-28. Quinn then reads three things: the paid
  invoice is unchanged, no proration line exists, and the upcoming invoice is at the new amount. A test clock is
  another way to get a paid period. Keep the trialing run as a second case, because T0r's question 9 asks about both.

**I3. D14 does not say what `move` does to a trial's first charge, and no test covers it (Quinn decision).** D14
(`:497-509`) selects subscribers by `current_period_end`. On a trialing subscription that date is the trial end, so a
student who started a trial at the old price a few days before a raise is moved before the first charge. Their
consent row (D3) records the old cents. The amendment's raise recommendation (`:1546-1547`, "at each student's next
charge after the notice") does not settle whether a trial's first charge counts.
- *Failure:* a student agrees to $12.99 on day 0. A raise lands on day 2. The student is noticed and charged the new
  price on day 14, against a consent record of 1299, and S2 makes this the rehearsed path.
- *Recommend:* on a raise, `move` skips a `trialing` subscription for the charge that ends the trial, so the first
  charge matches the consent. On a cut, it moves it. The rule follows from comparing the old and new cents, so it
  needs no new catalog field. Plan T3 gains a trialing case for each direction.
- Either way, T3 also needs a `yearly` `move` case, a renewal a year out reached through the window. Focus 3 asked for
  yearly renewals, and no pin covers one now (`:464-472`).

**I4. The notice window's edges are not defined, so "7 to 30 days" is not guaranteed.** D14 (`:497-509`) says the
renewal is "between N and N + 6 days away". It does not define how the days are measured. The tests use 12 days, then
9 and 17 (plan `:465-469`), and never an edge.
- *Failure:* the job rounds days down. With N = 24, a renewal 30 days and 20 hours away counts as day 30, and the
  notice goes out more than 30 days ahead. With N = 7, a renewal 6 days and 23 hours away counts as day 7, and the
  notice is late.
- The notice's date may also come from `billing_subscribers.current_period_end`. That is the webhook's copy
  (`20260910000200_billing_jobs.sql:30`), which can lag Stripe.
- *Fix (text):* define the window as `N×24h ≤ period_end − now < (N+6)×24h`. Take both the window and the stated date
  from the Stripe subscription the job has already read. Pin both edges, and the date source, in T3.

**I5. `site.yml` uses two different "latests", and nothing stops it running alongside a release.** In D15
(`:538-544`) and plan T10 (`:678-683`), the installer comes from the latest GitHub Release but the privacy guard
compares against the latest `v*` tag. `release.yml` creates the Release only after its Pages deploy (`:294-311`).
- *Failure (a):* tag v0.4.2 is pushed with a privacy bump, and its run fails or is still building. `site.yml` then
  restores v0.4.1's installer and finds main's `privacy.html` equal to v0.4.2's tag. The guard passes, and the new
  privacy page goes live while every installed build carries the old `PRIVACY_VERSION`. That is the case C25 exists
  to prevent.
- *Failure (b):* `site.yml` runs while `release.yml`'s publish job is running. It restores the previous Release and
  deploys after the tag's deploy, so `latest.json` and `Knowlu-setup.exe` go back to the old version.
- *Fix (text):* read one tag, the latest Release's `tagName`, and use it for both the restore and the privacy
  comparison. Refuse when any `v*` tag is newer than that Release. Give `site.yml` a `concurrency` group, and have it
  refuse while a `release` run is in progress. Its single job declares `environment: release`. If the `CLOUDFLARE_*`
  secrets are scoped to that environment, `site.yml` needs it to deploy. If they are not, leaving it out bypasses a
  future required-reviewer gate on publishing. Pin it beside `workflows.rs:190`.

**I6. The recovery the plan names reverts the terms once `site.yml` has published.** Plan §9 (`:990-991`) and §8 C
step 14 (`:951-952`) both say "re-run the tag's `publish` job". That job deploys the tag's `site/`.
- *Failure:* after the first price change, `site.yml` has published terms version X with the new figures. A
  re-run of the v0.4.1 publish job, for example after a hand deploy, puts the old terms back on knowlu.com. Checkout
  meanwhile keeps recording `TERMS_VERSION` X at the new price, so students consent to text that is not published.
- The artifact is also kept only 7 days (`release.yml:262`). Step 14 may run weeks after the tag, since T10 can
  arrive in a follow-up PR, and by then the named recovery no longer exists.
- *Fix (text):* once `site.yml` exists, recovery is `site.yml`, or a hand restore from the Release assets, never a tag
  re-run. Say so in §9, step 14, T6's `app.md` warning and D16.

## Minor (text fixes)

- **m1. A refusal is reported as a connection problem.** D9 `:341-346` and plan T4 `:518-521` show one sentence,
  "Check your connection", for every failure. A 401 from an expired pending session then loops forever on Try again,
  against plan §1 `:84` ("every refusal names its reason"). Show the command's own error when it has one, and the
  connection sentence only when the server cannot be reached.
- **m2. "One place" is claimed more widely than it is enforced.** D13 `:447-449` says the catalog is the only place
  under `functions/` where an amount or cadence is written. But:
  - `_shared/judge_caps.ts:45,97` says "$9.99 subscription", and the scan reads only four folders (plan `:474`);
  - D8's `custom_text` writes cadence words in `handler.ts`;
  - the app scan covers `wizard.js`, `picker.js`, `carried/account.js` and Settings, not all of `app/static`;
  - D16 (`:585-586`, "CI fails if one is missed") does not list `docs/reference/app.md:309-311`, the README, VISION
    or CLAUDE.md's Direction line.

  Fix: narrow the claim, or widen the scan to all of `functions/` and `app/static`, and put the documents in D16 as
  places to check.
- **m3. The cents rule will catch HTTP status codes.** The rule "any entry's cents as a number token" (plan `:476`)
  fails CI on `fail(500, …)` the day a price is $5.00, $4.00 or $4.09. Limit it to price contexts, or rely on the `$`,
  decimal and `STRIPE_PRICE_` rules.
- **m4. A derived variable name can collide.** If a price returns to an earlier amount while that amount is still in
  `KEPT` (raise 1299→1499 under `stay`, then cut back to 1299), D13's "no key and amount twice" and the derived name
  collide. The same happens for a new Price at the same cents with another tax behaviour. D16 should say: reuse
  (unarchive) the existing Price, and move the entry from `KEPT` back to `SELLING`.
- **m5. A `KEPT` entry can be removed too early.** D16 step 7 removes it "once no subscription uses the old price".
  `move` skips subscriptions that are cancelling at period end and those that are `past_due` (the view, `:34`). One
  that recovers or un-cancels later is then left unmapped. Check with a Stripe list of subscriptions by price, every
  status included.
- **m6. Three gaps in T3.**
  - Run the move before the annual reminder in the loop, so a same-day reminder states the price the renewal will
    charge.
  - Pin that the metadata write sends only the mark key, so `metadata[account_id]`, which the webhook keys on, is
    never overwritten.
  - Add the T2-style `index.ts` text-read: `billing-jobs` resolves every price variable as optional, so one unset
    `KEPT` variable cannot return 500 for the whole daily run.
- **m7. Link's own notice rules are not asked.** Add to T0r question 9: whether Link, as seller of record, sends its
  own notice of a price change or imposes a notice rule of its own.
- **m8. Some amendment citations are stale.** The note still calls Amendment 2026-11-17 a draft (`:28-38`), and its
  cites `:1360-1377`, `:1481-1491`, `:1439-1441` and `:1505-1511` have moved. In the signed text they are
  `:1368-1379`, Reading (7) `:1395-1404`, `:1480-1482` and `:1543-1550`. "Open item 2" no longer exists: Reading (7)
  hands that question to the lane, so Q14 (`:1006`) should cite Reading (7). Q5-Q7 are now settled, as are D9's
  "pending Quinn" (`:76, 80`) and plan §12 `:1111`.
- **m9. `site.yml` has two unstated preconditions.**
  - Step 14 says `site/` is "unchanged" but never checks it. Require `git diff v0.4.1 main -- site/` to be empty first.
  - A privacy bump merged on main before its release blocks every price change until that release ships. Put this in
    D16 step 0.
- **m10. One test wording contradicts the design.** Plan `:468-469`'s "cancel_at_period_end … make no call" cannot
  hold, because the flag comes from the Stripe read. Write "no mail, no write".
- **m11. Plan changes still need a release.** Adding or dropping a plan (the 6-month open item) leaves every v0.4.1+
  build at "Prices could not be loaded", because the app needs both keys. D16 should say a plan change needs a
  release.

## Checked and clean

1. **One source.** Checkout, the webhook, `billing-jobs`, the offer, the consent row and the terms tie all read the
   catalog through `Deps`. The derived names remove re-pointing. The other places amounts live are in m2.
2. **`checkout_offer`.** It caches nothing, and the buttons are disabled with no offer. A stale offer gets a 409 and a
   fresh offer. Only `account.rs` and `main.rs` (`:181, 270`, confirmed) change, by `contract-engineer` with
   `contract-reviewer`. `TOS_VERSION` moves only in T5's commit. `credentials.rs` and `updates.rs` are untouched.
3. **`stay`/`move`.** They are built to the signed promises (`:1406-1415`) and the change-time recommendations
   (`:1543-1550`). Each Stripe behaviour is marked `unverified (Stripe)`, and T0r questions 9-11 and the stop rules
   gate it. The gaps are I2-I4.
4. **`site.yml`.** The restore, the main-only rule, the privacy guard, the SHA pins (`workflows.rs:47` reads every
   workflow) and the no-self-hosted pin are all in. `release.yml` is not edited. The gaps are I5, I6 and m9.
5. **Removals.** Each item kept for zero customers still guards a real risk: the premise check, ending Quinn's own
   subscriptions grant-first, T5b for testers, deactivating the 100% code, the old-build 409, and the old variables
   until step 13. Nothing protective was dropped.
6. **Hygiene.** Every changed task is red first. The agents match CLAUDE.md's table. D13-D16 have ledger rows. S1,
   S2, B part 2 and T10's preview run are the live proofs on staging.
7. **Prices.** $12.99 every 4 weeks; $84.37 once a year, and 8437 = 13 × 649; "every 4 weeks" everywhere; the DOM
   envelope's 12831 = 13 × 987. No address, id or private detail appears in either document.

## For Quinn

- **I3:** does `move` change a trial's first charge? *Recommend:* not on a raise, yes on a cut.
- **Q14** is unchanged and still blocks T5 and T10. *Recommend:* yes, with I5's single tag and I6's recovery rule
  written into D15.
