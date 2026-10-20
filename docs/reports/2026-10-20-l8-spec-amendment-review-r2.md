# L8 friends-testers spec, Amendment 2026-10-20: review r2 (delta)

Spec: `docs/specs/2026-10-18-friends-testers-design.md`, revised after `docs/reports/2026-10-20-l8-spec-amendment-review.md`
(FAIL 0C/3I/7M). Reviewer, spec mode, delta only. Filed by the controller from the reviewer's returned text.

**Verdict: FAIL (0 critical, 3 important, 6 minor).** All ten earlier findings are closed. The new checks before a
grant are sound in structure. Three gaps came in with the revision: one lets a comp land unseen behind a trial, one
makes a claim to Quinn that S1b never tested, and one is a step-back screen the live proof can't pass.

## Confirmed (the ten)

- **I-1:** closed at :751-757. The query joins `auth.users`, needs exactly one account and needs it to match on both
  addresses. The revision rightly goes past the old fix: `last_sign_in_at` only limits how stale the account can be
  (:758-762, :33), and step 5 is the real proof.
- **I-2:** closed. S1b is recorded at :20-27, and the claim is reworded at :29-33, :431-434 and :745-747. Check 2b is
  weighed and left out at :775-782. (Its scope is too wide; see Important 2.)
- **I-3:** closed. Q7 sentence 2 names the reason (:642-646) and gives the rationale (:648-650).
- **M1:** closed at :1115-1116. **M2:** closed at :444-446 and :1270-1271. **M3:** closed at :763-767. **M4:** closed
  at :772-774 and :657-658. **M5:** closed for the note at :363-367, :1277-1279 and :1325 (the lede is still open; see
  Important 3). **M6:** closed at :1088-1091. **M7:** closed at :3 and :67-68.
- **D2:** consistent. The only way to a comp is an owner write (:405-416). Q1 (b), the tester-only door, is still
  turned down (:604-606). **Rule 1:** no personal identifier in the spec.

## Critical

None.

## Important

1. **Step 5 accepts a sign that a trial also produces** (:768-769; the claim at :597-599).
   - *Scenario:* while waiting, the friend presses a price and opens Stripe Checkout, which creates no `entitlements`
     row yet. Step 3 reads `none` and the grant is written. The friend then finishes Checkout; Q3's 409 cannot stop a
     session opened earlier. The trial wins (P2), Next moves on, and step 5 passes. The comp is hidden, and the friend
     is charged on day 8.
   - *Fix:* step 5 accepts only the Settings line "Free tester access through {date}". At step 5 the controller also
     re-reads `admin.testers` and finds `paid` unset. Soften the wording at :597-599 to match.
2. **"Nor `auth.identities`" is wider than what S1b read** (:29-30, :432-433, :745-746). S1b read two email-provider
   users only, and no Google user was made (:14).
   - Under GoTrue's automatic linking, a Google sign-in on an address someone already registered by password adds a
     `google` identity to that user, next to its `email` one. A fresh Google friend has `google` only.
   - So for Google friends, the identities table probably does reveal the pre-registered case §8.3 calls invisible.
     That bears on Quinn's C1b question (:783-788).
   - *Fix:* one staging read with a Google scratch user. If it confirms, add a check 1b: a friend who says Google must
     show `google` as the only identity, and any `email` identity means stop and ask Quinn. Otherwise reword these
     three places to "for email-code and password sign-ups".
3. **The step-back screen still shows a price** (:362, :1288). The subscribe panel's lede (`app/static/index.html:130`)
   reads "$9.99 a month, or $69.99 … your card is taken now", and its h2 is "Subscribe".
   - Neither §3.4 nor S5t's pin (:1274-1279) touches it, so the live proof's "never a price" (:1288) fails on today's
     markup, or gets waved through.
   - Putting the nothing-to-pay line in both the step and its note (:365, :1288) shows the same sentence twice.
   - *Fix:* when the plan is `tester`, S5 hides the lede, and the pin covers that. Pick one place for the line.

## Minor

1. **The controller ends a prod grant on its own** (:770-771), against :417 (prod writes only on Quinn's go). *Fix:*
   "asks Quinn, who ends it", or record that Quinn's go at step 4 covers this end.
2. **Step 3 doesn't say what to do with `canceled` or `past_due`** (:763-765). *Fix:* any status other than `none`
   stops the grant.
3. **The lookup stays in a snippet for up to a day** (:772-773). *Fix:* delete the tab right after step 4, then
   re-check the snippet list at step 6.
4. **Two stale references.** Q1 calls the friend's report "§8.3's last check" (:588), but step 6 is the last: say
   "step 5". S8's install note (:1330) asks only for the address; step 2 also needs "roughly when" (:750, :585).
5. **S1b says the password-amr session is "unexplained"** (:25-26). A likely cause, untested: for an unknown address,
   GoTrue's `/otp` creates the user through its sign-up path with a generated password, and with autoconfirm that
   sign-up opens a session `/otp` never returns. Leaving 2b out is right, but ":778 stopped a genuine friend"
   overstates the evidence, because the user tested never typed a code. *Fix:* name the hypothesis and soften :778.
6. **Q7 sentence 2 leaves out `cohort`**, which is stored and exported (:444, :1270). Harmless; the page could say
   "the kind of access". Quinn's choice.
