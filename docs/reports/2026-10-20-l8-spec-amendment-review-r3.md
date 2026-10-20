# L8 friends-testers spec, Amendment 2026-10-20: review r3 (delta)

Spec: `docs/specs/2026-10-18-friends-testers-design.md`, revised after `docs/reports/2026-10-20-l8-spec-amendment-review-r2.md`
(FAIL 0C/3I/6M). Reviewer, spec mode, delta only. Filed by the controller from the reviewer's returned text.

**Verdict: FAIL (0 critical, 2 important, 4 minor).** All nine r2 findings are closed. The S5 tester screen fits D2:
nothing changes for a student before a grant exists, and `entitlement_now` already returns `plan`
(`app/src/account.rs:1037`), so `account.rs` needs no change. Two claims still go further than the steps behind them.

## Confirmed (the nine)

- **I-1:** closed. Step 5 at :802-809; Q1's trade-off softened at :610-619; the Settings line specified at :381-383.
- **I-2:** closed. Claims scoped at :32-33, :444-447, :766-769; check 1b at :781-788; C1b weighing marks it
  "unverified" at :825-826.
- **I-3:** closed. Lede hidden and heading retitled at :364-367; the line once, in `#wiz-sub-note` (:368-370,
  :377-379); pin :1320-1323, live proof :1331-1333, S5 :1371.
- **M1** :807-809; **M2** :794-798; **M3** :800-801, :810-812; **M4** :604-605, :1375; **M5** :26-29, :816-818; **M6**
  :670-671. All closed.
- **D2:** consistent. Tester copy appears only after `checkEntitled()` is true and the plan is `tester`; :375-376 still
  turns down any line shown before the grant. **Rule 1:** no personal identifier.

## Critical

None.

## Important

1. **Check 1b says the first Google tester's read "confirms it or drops it". One fresh read can do neither** (:40-41,
   :787-788, S7 :1374). The check predicts (a) a fresh Google sign-in shows `google` only and (b) a Google sign-in on an
   address that already has a user shows `google` beside `email`. A genuine first tester tests only (a); (b), the part
   that catches a pre-registration, stays untested, yet :825 counts it toward not waiting for C1b. And an `email`
   beside `google` on the first read is also the check's own stop signal, so it cannot be read as "drop the check".
   *Fix:* "confirms only that a fresh Google sign-in shows `google` alone; (b) stays unverified"; an `email` beside
   `google` on the first read goes to Quinn as a stop, never as a reason to drop the check.
2. **Step 5 can pass before a Checkout opened earlier has finished** (:805-806; claims at :616-617 and :826). The friend
   presses a price before the grant and leaves the Stripe tab open; the grant is written; Settings shows the tester line
   and `paid` is unset, so step 5 passes within the hour; hours later the friend completes the still-valid Checkout
   Session (Q3's 409 refuses only new sessions; `billing-checkout` sets no `expires_at`, so Stripe's default 24-hour
   lifetime applies); the trial wins (P2) and the card is charged on day 8. *Fix:* step 5's re-read of `paid` happens no
   sooner than 24 hours after the grant, or a second re-read is added at 24 hours; reword :616-617 and :826.

## Minor

1. **The price buttons are missing from S5t's pin** (:1315-1323). `#wiz-sub-month` / `#wiz-sub-year` are never hidden
   today; the only entitlement check is in the click handler (`app/static/console.js:3123-3135`); the buttons sit in a
   `div.wiz-row` with no id (`app/static/index.html:131`). *Fix:* the pin covers the buttons, hidden for an entitled
   account.
2. **The tester screen can flash a price on a step back.** `WIZ.entitled` is set from the plain boolean in `wizGo`
   (:2812-2814), the price click (:3128) and `pollEntitlement` (:3052); none records the plan. *Fix:* S5 records the
   plan wherever `WIZ.entitled` is set, from one helper returning status and plan, and renders the tester form only
   once the plan is known.
3. **A Next pressed before the grant tells the friend to pay** (`wizValid`, `console.js:2787`: "Finish the payment page
   in your browser, then come back."). *Fix:* the install note says this line means "wait for Quinn"; Q1 names it.
4. **Copy and stale references.** Step 5, Q1, the live proof and S7 quote "Free tester access through {date}" word for
   word while Quinn writes the copy (:362): refer to "§3.4's Settings tester line". S1's task row (:1356) still names a
   Google user, against :15. §3.5's summary (:450-452) leaves out 1b's `email`-beside-`google` stop.
