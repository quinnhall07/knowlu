# v0.4.1 staging proof: S1 (Managed Payments) and S2 (the price-change rehearsal)

Run 2026-11-27 by the controller, alone. Counts, statuses and wording only: no address, no id, no receipt text.

## What ran, and on what

- **App:** the dev build of the lane (app and engine code unchanged since the build), on fresh scratch profiles, driven
  through the DOM only. No synthetic keyboard or mouse input on the desktop.
- **Server:** staging. The three billing functions from the lane head at first; `billing-checkout` without `custom_text`
  after the gate below (task T1c); the `account` function from task T1d for the last step.
- **Stripe:** the account's sandbox, test cards only. Stripe's pages were read and filled in a headless browser; Dashboard
  acts (moving a trial end, ending a trial, cancelling, the invented price, the invoice reads) were done with the Stripe CLI.
- **Accounts:** six scratch accounts (A to F), signed in by emailed code; E and F were used only to prove T1d.

## The gate S1 found first

Every press of a plan button answered "the payment provider refused the request". Stripe's own answer:
**"You cannot use custom_text with Managed Payments."** It refuses every slot (`submit`, `terms_of_service_acceptance`,
`after_submit`). Without the key the session is created with Managed Payments on and Stripe liable for the tax, and every
other key the form sends is accepted. So Managed Payments is in effect in the sandbox, and D8's sentence cannot be put on
Stripe's page. Task T1c removes the key; it is built and reviewed and waits for the founder's yes. Everything below ran on
that build.

What Stripe's page states by itself (first subscription): "14 days free"; "Then $12.99 every 4 weeks starting <date>" or
"Then $84.37 per year starting <date>"; a tax line ("Enter address to calculate", then the figure); "Total due today $0.00";
a required checkbox agreeing to the seller's Terms of Service and Privacy Policy; the button "Start trial"; "By subscribing,
you agree to Link's Terms and Privacy Policy and authorize Link to charge you for the described subscription until you
cancel."; "Sold through Link". It offers card and Cash App Pay. It does not state how to cancel inside Knowlu or the yearly
money-back guarantee; the app's Subscribe step states both before the page opens.

## S1 and the app checks

| Step | Result | What was seen |
|---|---|---|
| B0, offer loads | pass | Subscribe shows Yearly first, then Every 4 weeks, with the server's lines and "Plus tax where it applies."; no "trial", no "14" |
| B0, offer unreachable | pass | both buttons disabled, no lines, "Prices could not be loaded. Check your connection, then try again."; Try again draws the lines once the function answers |
| S1 (a), two checkouts | pass | each button opens Stripe's page with the panel's figure; opening the second expires the first (one open page per account, T1b, against real Stripe) |
| S1 (b), complete 4-week | pass | test card accepted; redirect to the subscribed page; the wizard moves on by itself; `four_weekly`, `trialing`, period end 14.00 days out; consent rows carry 1299 or 8437 and terms version 2026-11-21 |
| B4, Settings | pass | "Active, Every 4 weeks", "Through <date>", Manage subscription; no "trial" |
| S1 (c), trial reminder | pass | with the trial end moved to two days out, Stripe's event arrived within 15 seconds and Knowlu's mail in the same second: "Your Knowlu free trial is ending. Your first charge is on <weekday, date>: Knowlu, Every 4 weeks, $12.99 every 4 weeks, plus tax where it applies. It renews automatically until you cancel. To cancel before then, open Knowlu and go to Settings, then Membership, then Manage subscription, or reply to this message and we will cancel it for you." |
| S1 (d), cancel | pass | the portal shows the plan, the trial's end, Cancel subscription, the card and the invoice list, and no plan switch; cancelling is two presses there (the link, then "Review your changes") with no survey; the subscription is set to end at the period's end and `entitlements` follows when it ends |
| S1 (d), no second trial | pass | the same account's next checkout reads "Subscribe to Knowlu", the price due today with a sales tax line, button "Subscribe"; no "trial", no "days free" |
| S1 (e) with B2, B3 | pass | a second account: the panel as above; both pages match it; yearly completed: `yearly`, `trialing`, 14.00 days out; wizard finished to Home |
| S1 (e), delete | pass | Delete my data, two presses: "Deleted. Knowlu will close."; the subscription is set to cancel at the period's end with no new invoice; the account's rows are gone and one tombstone is written |
| B5, refusals | pass | `monthly` with the old terms date: "This version of Knowlu can't subscribe any more. Update Knowlu, then try again."; a wrong amount, or the right amount with the old terms date: "Knowlu's prices or terms have changed. Look them over again, then subscribe."; no consent row added, no session made |
| B6, upgrade sheet | pass | a signed-out install shows "Knowlu now needs an account"; after sign-in both plans with their lines; a click on the lines starts nothing |
| B7, re-consent | pass | an account whose recorded privacy version is one behind gets the panel "Our privacy policy changed", which names Link as the seller; Accept records 2026-11-21 |
| B8, terms version | pass | covered by S1 (b) and B5 |

Mail that reached the four test addresses: sign-in codes, two trial reminders and one price-change notice, all from Knowlu.
**The sandbox sent nothing from Stripe or Link** (no receipt, no trial notice), so what Link sends in live mode, and its
sender, are still unread. No test refund was made: the test charges carried no tax, so a refund would not have shown whether
tax comes back.

## S2, the price-change rehearsal

An invented higher 4-week price was created in the sandbox and the catalog changed on a scratch branch that is now deleted.

| Step | Result | What was seen |
|---|---|---|
| (a) setup | pass | account C subscribed at 1299: `four_weekly`, `trialing` |
| (b) the change reaches the app | pass | with no app rebuild the offer and the Subscribe step read "$14.99 every 4 weeks." and Stripe's page matches; a POST carrying the old offer is refused with the "changed" sentence |
| (c) `stay` | pass | job reply: reminded 0, moved 0, failed 0; no mail; subscription unchanged |
| (d) `move` on a trial | pass | notice 10 days: reminded 0, moved 0, failed 0; no mail; unchanged |
| (e) `move` on a paid period | pass | trial ended, 1299 charged, 28-day period; notice 22 days: reminded 0, moved 1, failed 0; one mail "Your Knowlu price is changing" naming both amounts and the renewal date and saying the paid period does not change |
| (e) three reads | pass | the paid invoice is unchanged (1299 paid, 0 credit notes); no proration line and 0 pending invoice items; the upcoming invoice is at the new amount, one line, on the same renewal date; the notice mark is in the subscription's metadata beside the account's |
| (e) second run | pass | reminded 0, moved 0, failed 0; still one notice; `entitlements` still `four_weekly`, `active` |
| (f) back | done | test subscriptions cancelled, the invented price archived and its variable unset, the functions redeployed; the offer answers 8437 and 1299 |

The job was run by hand with the cron job's own SQL command, so its address and token never left the database's vault.

## Findings

1. **Stripe refuses `custom_text` under Managed Payments.** Checkout was unusable on the lane head. T1c fixes it. D8 needs
   the founder's ruling.
2. **A former subscriber could not delete their account.** `DELETE /account` asked Stripe to cancel a subscription that
   had already ended; Stripe refused and the delete stopped. T1d fixes it (reviewer PASS after two fix rounds) and was
   proven on staging three times: twice with an ended subscription ("Deleted. Knowlu will close.", rows gone) and once
   with a live one (set to cancel at its period's end, no new invoice, rows gone). Measured on the way: Stripe answers
   HTTP 400, code `resource_missing`, when a list names a customer it does not know.
3. **Ending a trial by hand sends a wrong reminder.** Stripe sends its trial-ending event the moment a trial is ended from
   the Dashboard or CLI; Knowlu then mailed "Your first charge is on <the old date>" seconds after the card was charged.
   Open. It takes a staff action to cause it.
4. **An ended membership has no way back inside the app.** A signed-in profile whose subscription ended sees "Ended" and
   Manage subscription only; the plan buttons exist only in setup and on the sheet for installs without an account. Open.
5. **The live subscribed page is out of date.** It still names a seven-day trial and "the gear". The repository's copy is
   right; the site needs deploying before the release.
6. **A refused checkout still writes its consent row** (the row is written before the session, by design): two such rows
   were written while finding 1 was live.
7. **Settings shows the last answer it fetched**, so a cancel made in the browser shows at the next launch or the
   six-hourly check, not at once.
8. **The re-consent panel waits behind the week-setup sheet** on a new profile; it shows as soon as that sheet closes.
9. **Stripe's refusals are logged with the request path**, which holds an id, in the billing functions T1d did not touch.
10. **No message confirms a new subscription** in the sandbox, from Knowlu or from Stripe.

## What was left behind

Staging holds two of this proof's scratch accounts with no live subscription (their removal needs a confirmed SQL
statement). The sandbox holds this proof's cancelled subscriptions, the archived invented price, and the older test objects
it already had. The machine holds nothing: scratch profiles, vaults and credentials are gone.
