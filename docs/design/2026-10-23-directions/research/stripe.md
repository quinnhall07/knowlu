# Stripe as a design source

Research for the 2026-10-23 directions run. Researched 2026-10-23 from Stripe's own docs and
writing. Stripe matters to Knowlu for one reason above the rest: **Connect onboarding gets a
business through dozens of verification steps, some of which take days, without losing them.**
Knowlu's onboarding (spec `docs/specs/2026-10-23-onboarding-redesign-draft.md`) has the same shape:
many sign-ins, slow background reads, optional sources, quit and resume, then a confirmed result.

Confidence key: **[doc]** read from the cited page today; **[obs]** observed in Stripe's product or
known from public material but not re-verified today; treat [obs] as weaker.

All student examples below are invented.

---

## 1. Onboarding through many steps without losing people

### 1.1 The flow is rendered from a requirements list, not a fixed wizard [doc]
**What.** Hosted and embedded onboarding are a form "that renders dynamically based on the
capabilities, country, and business type of each connected account." The platform does not tell
the form which fields to show: "You don't need to identify the specific requirements, because the
onboarding interface knows what information it needs to collect." Step names exist "only for
analytics"; "steps can appear in any order and can repeat."
**Why it works.** A person only ever sees what is still outstanding for *them*. Re-entering the
flow later shows the remaining gaps, never the whole questionnaire again.
**URL.** https://docs.stripe.com/connect/hosted-onboarding ,
https://docs.stripe.com/connect/supported-embedded-components/account-onboarding.md?platform=web
**Knowlu.** Model onboarding as a list of sources with states (school sign-in, payment, Gmail,
calendar, zyBooks, VHL, syllabi per course), and render the next screen from "what is still
outstanding" rather than "step 7 of 11". Resume after quit then falls out for free: relaunch reads
`onboarding.json`, drops what is done, and opens on the first gap. The progress indicator becomes
a short checklist of sources, each with its state, not a step counter.
**Where not.** Stripe's order is opaque and varies; Knowlu's order is ruled (account -> school ->
proof -> payment -> Gmail -> calendar -> homework sites -> syllabi -> week). Keep the ruled order
and use the requirements idea only for skipping what is done and for resume.

### 1.2 Four requirement states, each with a different meaning [doc]
**What.** The `requirements` hash separates `currently_due` (needed by `current_deadline`),
`eventually_due` (may become needed), `past_due` (deadline passed, a capability is now off), and
`pending_verification` ("being reviewed ... No action is required"). `errors[]` carries a
plain-language `reason` per requirement.
**Why it works.** "We are checking" and "you must act" are different states with different UI, so
waiting never reads as failure and failure never hides inside waiting.
**URL.** https://docs.stripe.com/connect/handling-api-verification
**Knowlu.** A direct template for source states in onboarding and on the main page:
*needed now* (school sign-in), *optional, skipped* (VHL, with its cost named), *reading*
(first run in progress), *needs you* (sign-in expired, MFA), *done with proof*. And a striking
parallel on the main page: `current_deadline` -> `past_due` is exactly Knowlu's late window. A task
past its late window moves to "Late window closed", the way a `currently_due` field moves to
`past_due` when its deadline arrives: a state change with a name, not a red alarm.

### 1.3 Up-front vs incremental collection, stated as a trade-off [doc]
**What.** Platforms choose `eventually_due` (up-front: "one request for all information", avoids
later disruption) or `currently_due` (incremental: "onboard quickly"). Stripe documents both with
their advantages in a small table.
**Why it works.** It names the real cost of each choice instead of pretending there is none.
**URL.** https://docs.stripe.com/connect/hosted-onboarding#determine-the-information-to-collect
**Knowlu.** The ruled onboarding is up-front (one long flow), but optional sources are
incremental: skip zyBooks now, and the main page later offers it once, ambiently, with what it
would add ("zyBooks would add 12 activities this week"). The skip button states its cost, which the
spec already rules.

### 1.4 "Save for later" exits cleanly, and leaving is not finishing [doc]
**What.** "Stripe redirects the connected account back to this URL when they complete the
onboarding flow or click **Save for later** at any point." The return "doesn't mean that all
information has been collected"; the platform should re-check requirements and "provide prompts
in your application to allow them to continue onboarding later." Links are single use and expire
in minutes; an expired one goes to a refresh URL that silently mints a new one.
**Why it works.** Quitting is a supported path with no penalty, and stale sessions recover without
an error page.
**URL.** https://docs.stripe.com/connect/hosted-onboarding#return-url
**Knowlu.** Every onboarding screen gets a quiet "Finish later" that is honest about what is
unfinished ("Knowlu works with what you've connected; 3 steps left"). An expired school session at
resume should re-open the sign-in window, not show an error.

### 1.5 Prefill, then ask to confirm [doc]
**What.** If the platform knows details, it prefills them; "the onboarding interface asks the
account holder to confirm the pre-filled information," and every prefilled field stays editable.
Networked onboarding lets an owner "reuse that information from an existing account instead of
resubmitting it."
**Why it works.** Confirming is far cheaper than typing, and it keeps the person the authority.
**URL.** https://docs.stripe.com/connect/embedded-onboarding#prefill-information
**Knowlu.** This is the ruled "class times come from syllabi/registrar, never typed" step: show the
week Knowlu inferred, one row per class, with Edit on each row; typing is the last resort.

### 1.6 A summary step with per-section edit [doc]
**What.** The final `summary` step lists everything entered, and `summary_business`,
`summary_persons`, `summary_external_account` etc. each reopen one section from the summary.
**Why it works.** The person sees the whole result at once and corrects in place, without walking
back through the flow.
**URL.** https://docs.stripe.com/connect/supported-embedded-components/account-onboarding.md?platform=web
**Knowlu.** The "Here's your week" reveal is Knowlu's summary step. Each block on it (a class's
times, an exam, a late rule, a source) should open its own small editor and return to the reveal.

### 1.7 Errors carry the cause and the fix, in plain words [doc]
**What.** Each error has a machine `code` and a human `reason` that includes the fix: "Greyscale
documents cannot be read. Please upload a color copy of the document." "The provided street
address cannot be found. Please verify the street name and number." Duplicate uploads "fail
automatically" so a person is never told the same thing twice.
**Why it works.** The message is the way back; there is no dead end.
**URL.** https://docs.stripe.com/connect/handling-api-verification#validation-and-verification-errors
**Knowlu.** Matches BRAND-VOICE ("every hard truth comes with the way back"). Every failed source
reads as cause + action: "Blackboard asked for your phone code again. Sign in once more." Never
"Sync failed". Keep a code underneath for support.

### 1.8 Pending states carry a time expectation [doc]
**What.** "Stripe can take anywhere from a few minutes to a few business days to verify an image";
asynchronous payments are `processing` and "can take up to a few days."
**Why it works.** A wait with an expected length is tolerable; an unbounded spinner is not.
**URL.** https://docs.stripe.com/connect/handling-api-verification#confirm-id-verification
**Knowlu.** Every "reading..." in onboarding shows the spec's expected time ("usually 1-3 min"), and
the reveal waits up to 3 minutes, then names what is still running and when it will land.

---

## 2. Status at a glance

### 2.1 The banner that only exists when there is something to do [doc]
**What.** The notification banner "renders only when the connected account must respond ... or
provide currently due requirements." Its callback separates `actionRequired` from `total`, so a
host can say "You must resolve these" vs "The items below are in review."
**Why it works.** Calm by default; when it appears, it means something, and "in review" is never
dressed as "act now".
**URL.** https://docs.stripe.com/connect/supported-embedded-components/notification-banner
**Knowlu.** The ambient source-health line on the main page. Careful: DESIGN-PRINCIPLES 9 says
"silence is never ambiguous", so Knowlu cannot simply disappear when healthy. Keep a quiet positive
state ("Up to date, 12:04") and let the line grow only when the student must act, with
"needs you" and "checking" visibly different.

### 2.2 Many system states collapse to a few human labels [doc]
**What.** The Dashboard maps seven PaymentIntent states to a handful of labels: three different
API states all read **Incomplete**; `processing` reads **Pending**; detail is one click away in the
Inspector.
**Why it works.** The list stays scannable; precision lives on request (Knowlu's third visibility
level).
**URL.** https://docs.stripe.com/payments/payment-intents/verifying-status
**Knowlu.** A task row shows one of very few states (next, due today, late window open, done);
the reason ("moved up because the quiz opens at 2") is on request. Turned-in work never appears as
a state on Today at all: it is archived or auto-marked done with Undo.

### 2.3 Status pills typed by what the reader must do [doc]
**What.** Stripe Apps' Badge has six types defined by required action, not mood: Neutral ("no
action required"), Info, Positive, Negative ("no action required"), Warning ("needs immediate
action, optional to resolve"), Urgent ("strong requirement to resolve"). Badges are read-only;
"unlike buttons, badges ... don't perform any actions."
**Why it works.** The vocabulary is about consequence, so the same pill means the same thing
everywhere (Knowlu principle 13).
**URL.** https://docs.stripe.com/stripe-apps/components/badge.md?app-sdk-version=9
**Knowlu.** Adopt the axis (does the student need to act? must they?) but not the palette: red is
Knowlu's brand, never an alarm, and no state may rely on color alone, so each pill carries a word
and a shape. Keep pills read-only; actions are buttons.

### 2.4 Money shown with precision and comparison [obs]
**What.** Dashboard Home shows gross volume and balances as exact figures in tabular numerals,
each with a small chart against the previous period, plus "important notifications, like
unresolved disputes or identity verifications" [doc: https://docs.stripe.com/dashboard/basics].
Available and pending balances are shown separately.
**Why it works.** A number plus its comparison answers "is this normal?" without an adjective.
**Knowlu.** "Numbers over adjectives": show load as hours ("about 3 h of work today, 1 h more than
a usual Tuesday") with tabular numerals, and split *done* from *still to do* the way Stripe splits
available from pending. Where not: a student's day is not a revenue chart; no trend lines on
Today, one comparison at most.

---

## 3. Trust at the moment of asking

### 3.1 Checkout's elements of trust [doc, vendor guide]
**What.** Stripe's own payment-page guidance lists a transparent order summary, one highly visible
pay button, trust indicators and inline error feedback; ask only for necessary information; don't
force account creation first.
**URL.** https://stripe.com/resources/more/payment-page-template-best-practices
**Knowlu.** The payment step comes right after proof, so the "order summary" is the proof itself:
"Knowlu found 5 classes, 23 assignments, 9 already turned in." Then one price, exactly ($9.99 a
month, no free tier), how to cancel, and one button.

### 3.2 Recognize, then verify, then fill (Link) [doc]
**What.** Link recognizes a returning email, sends a one-time code only "on a new site or device",
then autofills saved details.
**URL.** https://support.stripe.com/questions/integrating-link-with-the-payment-element ,
https://stripe.com/payments/link
**Knowlu.** The same rhythm for the kept school sign-in: one sign-in, then every later read reuses
it, and the screen says so ("You won't sign in to these again"). Name the boundary at the moment
of asking: the password stays on this computer ("fetch on device, think in the cloud").

### 3.3 Sensitive links never leave the app [doc]
**What.** "Don't email, text, or otherwise send account link URLs outside of your platform
application," because a link "grants access to the account holder's personal information."
**Knowlu.** Supports a short, specific privacy line at each login rather than a generic badge:
what is read, where it is stored, what never leaves the device.

---

## 4. Craft

### 4.1 Accessible color by perceptual steps [doc]
**What.** Stripe rebuilt its palette in a perceptually uniform space so every hue follows the same
lightness curve. Rule: colors "five levels apart" pass for small text, "four levels apart" for
icons and large text. Badges were fixed by shifting background and text up one level together.
Authors Daryl Koopersmith and Wilson Miner, 2019-10-15.
**URL.** https://stripe.com/blog/accessible-color-systems
**Knowlu.** Build tokens in OKLCH with fixed lightness steps per hue, and write the step-distance
rule into the token file so light and dark modes keep contrast by construction (principle 8,
"readability is not negotiable").

### 4.2 Motion that does work (Checkout, 2014) [obs]
**What.** Michael Villar's essay on Checkout animation: the payment form is where people decide to
trust you; animation should explain what is happening; test: if turning it off makes the flow feel
broken, it was doing work. His favorite was the checkmark on success; errors shake the field.
**URL.** https://medium.com/bridge-collection/3d1b0a9b810e (403 to fetch today; summarized from
secondary sources)
**Knowlu.** Use the test as a rule for every animation. Take the success checkmark for proof
moments ("found 5 classes") and a task marked done. **Not** the error shake: it is an
attention-grabbing alarm, against "calm by default" and principle 11.

### 4.3 The gradient and the type [obs]
**What.** stripe.com's animated mesh gradient (WebGL, reconstructed publicly as "minigl"; see
https://tympanus.net/codrops/2022/09/26/how-to-recreate-stripes-lava-lamp-gradient-with-three-js)
and a precise grotesque set with tight spacing; Stripe Press books get a separate, editorial,
slower tone (https://press.stripe.com/).
**Knowlu.** At most one expressive moment: the "Here's your week" reveal could carry color and
motion once, the way Stripe's marketing does. Not on the daily page: principle 7 forbids decoration
without information, and the Dashboard itself is plain. Stripe Press's lesson is tone, not style:
long-form reading deserves a calmer type setting than a dashboard, which suits syllabus and
"why" text on request.

### 4.4 Docs that feel like an application [doc]
**What.** Markdoc (Ryan Paul, 2022-09-13) let Stripe docs "feel like an application rather than a
traditional user manual": samples carry the reader's own test key; content shows or hides by what
they use; a fixed set of tags enforces consistency at build time.
**URL.** https://stripe.dev/blog/markdoc
**Knowlu.** Explanations in onboarding should use the student's own found data, not generic copy:
"Your Statistics syllabus says late work loses 10% a day" beats "Syllabi contain late rules." Keep
every benefit line and proof template as structured data (one table) so the voice stays consistent
across steps.

---

## Where Stripe does not transfer

- **Density.** The Dashboard is built for operators who scan hundreds of rows. Knowlu answers one
  question; Today is not a table.
- **Stakes vocabulary.** "Urgent", "disabled", "rejected" fit compliance. Knowlu's late work gets
  "Late window closed" with Archive and Still doing it, never alarm language.
- **Opaque step order.** Knowlu's order is ruled and explained; only resume and skipping borrow the
  requirements model.
- **Brand color for warning.** Stripe uses red for negative states; Knowlu's red is the brand and
  never marks being behind.

## Top 12 transferable ideas

1. Render onboarding from a list of outstanding sources, so resume opens on the first gap and never repeats a done step.
2. Use distinct source states (needed now, optional skipped, reading, needs you, done with proof), so "checking" never looks like "failed".
3. Treat the late window like `current_deadline` -> `past_due`: a named move into "Late window closed", not an alarm.
4. Offer "Finish later" on every step, and be honest that leaving is not finishing ("3 steps left").
5. Prefill everything inferred and ask only to confirm, with Edit on each row; typing is the last resort.
6. Make the "Here's your week" reveal a summary step where each block opens its own editor and returns.
7. Write every failure as cause plus fix ("Blackboard asked for your code again. Sign in once more."), with a code underneath.
8. Give every wait an expected time, cap it, then name what is still running.
9. Keep a calm positive health line by default, and let it grow only when the student must act, keeping "needs you" separate from "checking".
10. Type status pills by required action (none / optional / required), always with a word and a shape, never by red.
11. Use the proof as the order summary before the $9.99 payment: exact price, how to cancel, one button.
12. Build OKLCH tokens with a step-distance contrast rule, and keep motion to explanatory moments (proof checkmark, task done), never an error shake.
