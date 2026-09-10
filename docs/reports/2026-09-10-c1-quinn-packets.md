# C1 — the two packets for Quinn (P4 and P5)

Collected at the close from Task 19's and Task 20's reports (2026-09-10). The pages themselves are `site/privacy.html` and `site/terms.html` on the C1 branch; the Google texts are verbatim from the plan's Task 20.

## P5 — the facts only Quinn can confirm in the drafted policies (Task 19)


Each of these is a real sentence on a published page, written to the best-supported value in the briefing. None is a placeholder; each is a fact only Quinn can settle.

1. **The legal entity.** `privacy.html`: "Knowlu is an independent business in Alabama, run by its founder — one person, who also writes the code." If Knowlu is (or becomes) an LLC, that sentence should name it, and the terms' *Governing law* and *Disputes* paragraphs should name it as the contracting party.
2. **The postal address.** `privacy.html`: "If you would rather write on paper, ask and we will send you a postal address." Written this way deliberately — Alabama's breach statute and CalOPPA want a contact a person answers, not necessarily a street address on the page, and a home address on a public site is not one to publish lightly. If a business address exists, it belongs here instead.
3. **The two mailboxes.** `support@knowlu.com` (contact, rights requests, refunds, disputes-first) and `security@knowlu.com` (the responsible person for security) appear on both pages. **They must exist and be read before either page is published** — the privacy policy is a Google verification artefact and a CalOPPA obligation, and both promise a person answers. If one address should do the job of both, say so and I will collapse them.
4. **The named person responsible for security.** The page says "One person is responsible for security at Knowlu — its founder, the same person who answers support" and does not print a name, per the brief's pronoun rule. Alabama's § 8-38-3 factor list contemplates a *designated* responsible employee; if the lawyer wants a name on the page, that is a one-line change.
5. **Governing law: Alabama**, and venue in the state and federal courts in Alabama (terms, *The usual, said plainly*). Taken from the brief's step 3 #12.
6. **Arbitration — the clause the brief flags for the lawyer.** The draft **does not** contain an arbitration clause or a class-action waiver. It says instead: informal resolution first (30 days), then the Alabama courts, with small claims preserved, and "We do not require you to arbitrate", with a promise to give notice and a way to opt out if that ever changes. Reasoning: a home-drafted arbitration clause is the clause a court reads hardest and the one that does not bind a minor, and the briefing's §2 says the practical downside of a minor's voidability is a refund, which §7 of the terms already gives. **A lawyer should see this paragraph and the limitation-of-liability paragraph specifically** before the first non-founder paid sign-up; if they want arbitration, it should be their words, not mine.
7. **The email provider.** `privacy.html` names Supabase, Stripe, Cloudflare and Anthropic outright and describes the fifth — "the provider that sends our account and billing email" — without a name, because P2 has not fixed one (the plan suggests Resend). Once it is chosen, it should be named like the other four.
8. **Stripe's billing emails must carry the portal link.** The terms say "Stripe's receipts link straight to that portal". That is a Stripe Customer Portal / billing-email setting in P2, and California's ARL is the reason it matters. Turn it on, or tell me and I will soften the sentence.
9. **The reminder email's wording vs. the app's button.** Task 5's `annualReminder()` tells the student to "open Knowlu, click the gear, and choose **Cancel subscription**", but the settings row's button says **Manage subscription** (`app/static/index.html:161`, `set-portal`). The terms follow the app. One of the two should change — cloud file, not mine to edit.
10. **The 18+ gate, as the brief's step 6 asks it be put:** we decline under-18s rather than collect a birthdate. It is the cheap path and it costs us the 17-year-old freshmen for a semester. The privacy page says so in as many words, including "If you are 17 and this is the wrong answer for you, write to us".



11. **Two Stripe dashboard settings the terms assert as fact.** `terms.html` says cancelling in the portal is one click with "No survey, no phone call, no email required", and that Stripe's receipts link straight to that portal. Neither is set by code in this repo: the first is the Customer Portal's cancellation configuration (P2 asks for it — "cancellation enabled and no cancellation survey"), the second is a Stripe billing-email setting. Both must be on before the pages are published, because the first is the sentence California's ARL is read against. (This supersedes the narrower item #8 above, which named only the receipt link.)


## P4 — the Google verification packet (Task 20 steps 3 and 4)

### Step 3 — gmail.readonly justification

> Knowlu is a desktop planner for university students. It reads the student's own inbox, twice a day, to find messages that contain an obligation — an assignment change, a meeting request, a deadline from an instructor — and proposes them as tasks the student approves or declines inside Knowlu. Message text is read on our server, used for that one classification, and discarded; only the message id, the classification and the fields the student approved are stored. Attachments are never fetched, nothing is ever sent on the student's behalf, and Gmail-derived content is excluded from any model improvement. `gmail.readonly` is the narrowest scope that permits this: `gmail.metadata` is also restricted and carries no body, so it cannot tell an assignment change from a newsletter, and the label scopes give no message access at all.

### Step 4 — demo shot list

Google wants an unlisted video showing the consent screen and the data's use, on the actual app,
with the OAuth client id visible. Two to three minutes:

1. The site at `knowlu.com`, then the privacy policy, showing the Gmail section.
2. The installed app, the wizard's Gmail step, and the sentence explaining testing mode.
3. Pressing Connect: the Google consent screen, **with the app name and the requested scope on screen**, and the browser's address bar showing the client id in the URL.
4. Back in the app: a proposal card that came from an email, with the source visible on the card.
5. The Decisions deck: approving one, declining another.
6. Settings → the Gmail row → **Disconnect**, and the sentence saying the token is revoked at Google and the row deleted.
7. Settings → Delete my data, and the confirmation text.

