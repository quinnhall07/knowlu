# Legal pages audit: privacy.html and terms.html

Researcher (Opus, high), 2026-11-17. Read-only audit of `site/privacy.html` and `site/terms.html` at origin/main 84933416 against the code, the signed amendments and current law and policy. Filed by the controller. Not legal advice; section 6 lists what a lawyer should check.

I've audited both pages. They are careful and mostly true to the code, but they are not airtight yet. The terms are the weaker page.

The terms still carry the old $9.99 / $69.99 / 7-day / summer-pause prices. Their refund line contradicts the site's money-back promise, and they promise a re-acceptance flow that doesn't exist in code. Neither page names a legal entity or postal address. On privacy, about 30 statements check out against code. About 14 are partly true or overstated, mainly retention, "we collect nothing else", deletion and the email vendor. One is plainly false: the number of run logs kept. One structural fact affects both pages: the repo says Stripe Managed Payments "stays on", which would make Link (Stripe's checkout service) the seller. But the checkout code doesn't create Managed Payments sessions.

**Limits of what I checked.** The hook allowed neither `git -C` nor `ls-tree`. So I read code facts as files from the worktree, and confirmed only that `origin/main` = `84933416`. I did not prove the worktree HEAD is that commit. All cites below are relative to `C:\Users\danie\GitHub\knowlu\.claude\worktrees\v040-ui\`. `P` = `site/privacy.html` and `T` = `site/terms.html`. I read the homepage with WebFetch, which could not see its `<head>`. I opened no vault.

## 1. Claims check

**False or partly true**

| Claim | Where | Verdict | Evidence |
|---|---|---|---|
| App data keeps "the last sixty run logs" | P:24 | **False** | `app/src/scheduler.rs:30-35` keeps 80 log *files* (about 20 runs), and never prunes `quit-*` records |
| "If a category is not on this list, we are not collecting it" | P:33 | Partly | Several things are collected but not listed: emails sent to support or hello@; the judgments table's title words, event organizer and email sender (`cloud/supabase/functions/_shared/judge_rules.ts:13-16, 62-78`); mail batch and delivery-check records (`mail-process/sweep.ts:12-14`); and the IP and user agent Supabase Auth logs by default (couldn't verify the columns from the repo) |
| Judgments keep no title | P:14, 58 (implied) | Partly | `judgments.fields` holds the first three words of each title (`judge_rules.ts:15-16, 67-68`); the page never says so |
| The email provider "sees the address a message is going to" | P:86 | Partly | It is Resend (`cloud/supabase/config.toml:81-90`, `billing-jobs/index.ts:30`). It gets the whole message, including the sign-in code and reminder text, and the page doesn't name it |
| "Seven companies… Nobody else gets anything" | P:75, 86 | Partly | Resend is unnamed. Link would be a recipient too if Managed Payments is on |
| "Deleting is immediate, and it is a purge" | P:135 | Partly | The purge is real (`functions/account/index.ts:99-153`). Platform backups aren't mentioned, and the consent rows survive with no upper limit: no cron in `migrations/*.sql` touches `consents` |
| The consent record is "the one record a deletion does not take" | P:141 | Partly | All consent rows survive deletion, not just the subscription one: `tos`, `privacy`, `age_18` and `auto_renew` (`account/index.ts:114` patches every row; `migrations/20260910000100_accounts.sql:36-45`) |
| The tombstone hash "cannot be turned back into an address" | P:139 | Partly | It is an unsalted SHA-256 of the lowercased email (`_shared/crypto.ts:31-33`), so anyone who guesses an address can confirm it |
| Calendar links: a key "only the code that fetches your feeds can use" | P:158 | Partly | The account function holds the same key to encrypt the links (`account/index.ts:261-263`) |
| Breach notice "no later than 45 days" for everyone | P:160 | Partly | Alabama's limit is 45 days, but Colorado, Florida and Washington require 30 ([PRC survey](https://privacyrights.org/resources-tools/reports/data-breach-notification-laws-50-state-survey-2026-edition)) |
| "None of it is ever sold or shared" | P:14 | Partly | Not sold, and not "shared" in the CCPA sense. But in plain English it is passed to eight processors, which the page itself lists |
| Zyp/VHL: Knowlu "reads the assignment list that comes back" | P:44 | Partly | The server receives the raw portal payload (`ingest-coursework/handler.ts:1`) |
| A material change brings an email, then a yes in the app | P:166 | Partly | The in-app panel exists (`app/static/console.js:3127-3160`). No code sends the email |
| A material terms change means accepting it before going on | T:70 | **Not built** | The server records only later *privacy* versions and never restamps the terms version (`account/handler.ts:223-247`). The terms version is pinned at `app/src/account.rs:24` |
| Price $9.99 / $69.99, 7-day trial | T:24 | True of billing today; false against the signed prices and the site | `billing-checkout/handler.ts:58-59`, `billing-jobs/handler.ts:71-75` |
| "We email you when the pause ends, before the charge comes back" | T:29 | Partly | The notice and the resume happen in the same run, with no lead time (`billing-jobs/handler.ts:146-161`) |
| "California requires this reminder for a monthly subscription" | T:36 | Partly (stale) | California now requires it for every auto-renewal ([AB 2863 summary](https://btlaw.com/en/insights/alerts/2025/california-expands-automatic-renewal-law-new-requirements-now-in-effect)). The code already sends it for every plan (`billing-jobs/handler.ts:130-134`) |
| Stripe's receipts link to the portal | T:33 | Can't verify | It's a dashboard setting. Under Managed Payments, receipts come from Link ([Stripe](https://docs.stripe.com/payments/managed-payments/how-it-works)) |
| "Knowlu reads… It never posts" | T:17 | Partly | It turns forwarding on and off, forwards older mail and trashes the Sent copies (P:123-130) |
| What Knowlu does | T:16 | Partly | It omits email forwarding, grades, the registrar read and the kept sign-ins. The terms date from 2026-09-10 and predate all of these |
| "We never ask for your university sign-in" | T:49 | Partly | Knowlu opens the school sign-in in its own window and keeps the school-mailbox and Blackboard sessions (P:26, 69) |
| zyBooks/VHL terms "discourage" automated access | T:50 | Partly false | zyBooks *prohibits* scrapers and AI processing (`docs/notes/2026-09-09-knowlu-cloud-legal-landscape.md` §6). Coursework titles still reach the model, because tier 1 never sets importance (`engine/src/judge.rs:440-451`) |
| "At the pace a person would read at… stops at the first sign…" | T:50 | Can't verify | I found no throttle and no block detection in `engine/src` |
| If Knowlu closes the account: 30-day hold and a pro-rata refund | T:60 | Not built | No code for either |
| Gmail is in testing mode: 100 users, 7-day expiry | P:103 | Can't verify | That's Google Cloud console state |
| Amazon's one-day expiry | P:83, 106 | Can't verify | The lifecycle rule isn't in the repo. The sweep assumes it (`mail-process/sweep.ts:5`) |
| No third-party cookies on the website | P:163 | Can't verify | The page body has no third-party scripts. The `<head>` wasn't visible, and Cloudflare's own cookies are unknown |

**Verified true (short list):**
- Vault under `%USERPROFILE%\Knowlu` (`app/src/onboarding.rs:431, 540`).
- Commitment questions stay on the computer (CLAUDE.md, `commitments::LOCAL_CARD_KINDS`).
- Consent fields recorded as stated (`account/handler.ts:182-213`); checkout consent stores price, terms version and IP (`billing-checkout/handler.ts:121-128`).
- Google profile name and picture are dropped from the token but kept in the identity record (`migrations/20260922000100_trim_user_metadata.sql:13-17`).
- 200 MiB cap (`migrations/20260912000100_sync.sql:155-160`); plaintext copy, not end-to-end (`20260912000300_sync_plaintext.sql:5-6`).
- The computer name is in the journal (`engine/src/journal.rs:68-75`).
- Google token kept in Supabase Vault (`20260929000200_gmail_disconnect_purge.sql:35-39`).
- No free text in telemetry; course corrections drop both values (`telemetry/handler.ts`).
- Tester records: 30 days and 3 years (`20261020000500_testers.sql:194-195`).
- Issue reports: 200 lines, kept 90 days (`app/src/report.rs:207, 270`; `20260910000500_issues.sql:22-26`).
- Gmail: read list 30 days, waiting proposals 7 days (`20260911000900_final_review_fixes.sql:48, 72`).
- Character limits of 1,200, 600 and 2,000 (`_shared/judge_prompts.ts:15-17, 321`).
- Zero-retention pin, one upstream, no fallback (`_shared/judge_openrouter.ts:27-39`). Assignments and email go to DeepInfra, calendar events to CoreWeave (`20261023000100_task_effort_anchors.sql:28-32`, `20261002000200_mail.sql:347-358`, `20260916000100_provider_swap.sql:35-39`).
- Attachments are never fetched (`gmail-read/index.ts:40`).
- Mail retention of 180 days, 7 days and 30 days (`sweep.ts:12-14`).
- Journal kept 400 days, except human-set entries (`sync.sql:170-199`).
- Tombstone kept 90 days (`20260910000300_deletion.sql:11-16`).
- Delete order as described (`app/src/account.rs:1817-1841`).
- Under-18 sign-up is refused before anything is sent (`app/src/account.rs:601-603, 1579-1581`).
- 72-hour grace (T:56; CLAUDE.md, `engine/src/entitle.rs`).

## 2. Missing or weak, by risk

**Who "we" is today.** The privacy page says "an independent business in Alabama, run by its founder — one person" (P:18). The terms say "between you and Knowlu" (T:13). Neither names a legal entity or a postal address, and the site footer says only "© 2026 Knowlu." If nothing is incorporated, the contracting party is the founder personally.

| # | Gap | Why it matters, and the rule |
|---|---|---|
| 1 | No contracting party, no address, and an unsettled seller of record | Federal subscription law (ROSCA, 15 U.S.C. § 8403) needs clear material terms from an identifiable seller. Commercial email needs a postal address ([FTC CAN-SPAM guide](https://www.ftc.gov/business-guidance/resources/can-spam-act-compliance-guide-business)). Under Managed Payments the customer sees "Sold through Link", and Link sends receipts and trial reminders, handles cancellation at link.com, and may refund within 60 days on its own ([Stripe](https://docs.stripe.com/payments/managed-payments/how-it-works)). But the code passes neither `managed_payments[enabled]` nor drops the parameters Managed Payments forbids (`automatic_tax`, `customer_update[address]`; `billing-checkout/handler.ts:52-71`; [Stripe](https://docs.stripe.com/payments/managed-payments/update-checkout)) |
| 2 | Auto-renewal consent and notices are incomplete | California (Bus. & Prof. Code § 17600 et seq., as amended July 2025) needs express affirmative consent to the renewal terms, an acknowledgment the customer can keep, online cancellation, an annual reminder, notice before a price change, and consent records kept for 3 years or 1 year after the contract ends, whichever is longer ([summary](https://btlaw.com/en/insights/alerts/2025/california-expands-automatic-renewal-law-new-requirements-now-in-effect)). Today the checkbox is Stripe's generic terms checkbox; no custom consent text is set. No Knowlu acknowledgment email is sent, and the trial reminder (P14) isn't built. In v0.4.0 the app shows no price, so the renewal terms appear only on Stripe's page. The federal click-to-cancel rule was vacated in July 2025; the FTC published an advance notice (ANPRM) on 2026-03-11 and has no proposed rule yet, and it still enforces under ROSCA and § 5 ([Gibson Dunn](https://gibsondunn.com/ftc-restarts-negative-option-rulemaking-after-eighth-circuit-vacatur-enforcement-under-rosca-continues/)). New York's 2025 click-to-cancel bill: I could not confirm it was enacted ([S5615](https://www.nysenate.gov/legislation/bills/2025/S5615)) |
| 3 | The "money-back guarantee" has unstated limits | Under 16 CFR 239.3, "Money Back Guarantee" should be used only if the full price is refunded, and any material limits must be disclosed prominently ([eCFR via LII](https://www.law.cornell.edu/cfr/text/16/239.3)). The site's line has no limits; the signed scope is yearly-only |
| 4 | Google user data | The Limited Use sentence is present (P:94). Missing: the restricted-scope verification and annual security assessment (CASA) before Gmail goes past testing ([Google](https://developers.google.com/identity/protocols/oauth2/production-readiness/restricted-scope-verification)). Also: a merger transfer of Google data needs *explicit prior consent* ([Workspace policy](https://developers.google.com/workspace/workspace-api-user-data-developer-policy)), but T:67 only promises "we will tell you". The carve-out for a user's personalized model is still in the policy, so P:99 holds |
| 5 | No authority clause for acting in mailboxes and portals, and no non-affiliation clause | The terms need the student to authorize Knowlu to act for them: set forwarding, send forwards, trash Sent copies, use the kept sessions. They need the student to state they are permitted to under their school's and provider's rules. And they need "not affiliated with zyBooks, VHL, Blackboard, Google, Microsoft". The access-authorization risk after *Van Buren* and *Power Ventures* is set out in the landscape note §6. zyBooks' AI-processing clause conflicts with model judging of coursework titles |
| 6 | Accuracy and academic-integrity disclaimer is thin | T:17 and T:63 cover ranking, not wrong or missed due dates. Add: Knowlu is not the official record, so check the LMS or syllabus; AI estimates can be wrong; Knowlu doesn't submit work |
| 7 | Breach and security | Use "the shortest time the law requires, and no later than 45 days". Alabama requires reasonable security (Ala. Code § 8-38-3) and AG notice above 1,000 residents. The repo has no incident plan |
| 8 | Subprocessor list | Name Resend; name Link if Managed Payments is on; give regions (AWS is us-east-2, per memory); confirm data processing agreements |
| 9 | Retention without limits | Telemetry, corrections, judgments and consents have no maximum, and the backup tail isn't stated |
| 10 | State privacy laws | None is triggered. California's CCPA applies above $26,625,000 in revenue or 100,000 consumers ([CPPA](https://cppa.ca.gov/announcements/2024/20241217.html)). Alabama's APDPA applies above 25,000 consumers from 2027-05-01, and exempts businesses with fewer than 500 employees that don't sell data ([Hunton](https://www.hunton.com/privacy-and-cybersecurity-law-blog/alabama-becomes-21st-state-with-comprehensive-consumer-privacy-law)). Keep the universal rights; add a response time and an appeal path |
| 11 | No FERPA positioning | Add "Knowlu works for you, not your school; it is not a school official," and never claim compliance (landscape note §1) |
| 12 | Age and COPPA | COPPA doesn't apply with the 18+ gate (landscape note §2; not re-fetched). Nebraska (19) and Mississippi (21) are still unverified |
| 13 | CAN-SPAM | Billing email is transactional and exempt. Waitlist and launch email will need a postal address and an opt-out |
| 14 | Standard terms clauses that are missing | Software licence and restrictions; your licence to Knowlu to process your content; automatic updates and starting with Windows; consent to electronic notices; third-party services (Google, Stripe, Link); notices address; entire agreement; survival; force majeure; pilot/beta status; chargebacks |
| 15 | Clauses that are present but weak | Liability cap (T:64): a trial user's cap is $0, so consider "the greater of $50 or…". There is no indemnity, which is defensible for a consumer product. Governing law (T:65) and no arbitration (T:66) are clear. Termination (T:59-60) exists. Changes clause: notice period unspecified and the mechanism isn't built. Contact: support@ and security@ are named but unverified, while the site uses hello@ |

## 3. Inconsistencies

| A | B |
|---|---|
| T:39 "We do not refund a month you have already used" | Site: "30-day money-back guarantee. Cancel anytime." (unqualified); Amendment P12: yearly only, 30 days, access ends on refund |
| T:24, app `app/static/index.html:123, 135`, `VISION.md:153-155` ($9.99, $39.99 6-month, Pell 75%) | Amendment 2026-11-10: $12.99 every 4 weeks, $84.37 a year, Pell 50%, 14-day trial; site: "Pricing will be announced" |
| T:27-29 summer pause | Amendment: no pause for new subscriptions (P10 keeps it for old ones) |
| T:25 "will not apply to a period you have already paid for" | Amendment (6): a Pell year can be shortened to 6 months |
| T:36 reminder text and `billing-jobs` "per month" | Every-4-weeks billing |
| Brief: Pell "taken on trust" | Signed amendment (5): award letter within 30 days |
| P:92 Google sign-in gets "three things" (including the picture) | App `index.html:129`: "your email address and your name, and nothing else" |
| Site FAQ: data "goes to Knowlu's servers to plan your day" | Ranking runs on the device; content also goes to OpenRouter, DeepInfra, CoreWeave and AWS (P:74-86) |
| Planned onboarding "Your password stays on this computer" | P:69: Knowlu never sees the school password. The onboarding draft's shared school profile would be a third kept sign-in (`docs/specs/2026-10-23-onboarding-redesign-draft.md:91-92`), against "two kept sign-ins" (P:26) |
| v0.5.0 questions (why downloaded, where heard) sent to Knowlu | Not on the P:33 list, so this is a material change; P:52 "no free text" |
| HANDOFF "Managed Payments stays on" | `billing-checkout/handler.ts:52-71` doesn't create Managed Payments sessions |
| T:33 Stripe receipts and portal | Under Managed Payments, Link sends receipts and handles cancellation |
| T:49 "never ask for your university sign-in" | P:26 kept school-mailbox and Blackboard sessions |
| Site mailto hello@ | Pages name only support@ and security@ |

## 4. Questions for you

1. **Who is "we"?** I recommend forming an Alabama LLC before the first paid sale, and naming it with a mailing address (a PO box is fine) in both pages and in commercial email.
2. **Who is the seller of record?** I recommend Managed Payments, since it takes tax and disputes off a solo operator. That means fixing `billing-checkout` and writing the payment clauses around "Sold through Link". Otherwise, turn it off and register for Kentucky sales tax.
3. **Refund scope?** I recommend P12 as signed (yearly, 30 days), plus keeping the "charge you didn't mean" refund on the 4-week plan. The site line would become "30-day money-back guarantee on the yearly plan".
4. **Pell proof: trust or award letter?** I recommend the signed letter rule. If you want trust instead, amend the spec first. Either way, the privacy page must say what is kept.
5. **Governing state and venue?** I recommend keeping Alabama law, with small-claims court available to the student wherever they live.
6. **Arbitration?** I recommend no arbitration, as now. It is cheap trust, and consumer arbitration is costly for a tiny company.
7. **Support and contact?** I recommend one monitored support@ and one security@, with hello@ forwarding to support@, and replies to billing email reaching a person.
8. **Liability cap and indemnity?** I recommend a cap of the greater of $50 or 12 months paid, and no indemnity from the student.
9. **Retention limits?** I recommend telemetry and corrections at 2 years, consents deleted 3 years after the account ends, and the backup tail stated.
10. **Kept sign-ins "at every school until 20 paying users"?** I recommend keeping it, but adding a terms clause that the student confirms their school allows forwarding and saved sessions.
11. **The v0.5.0 onboarding answers?** I recommend closed choices only, stored like telemetry, and announced with a privacy-version bump and re-consent.
12. **When does the lawyer read happen?** I recommend before the first paid sale, on the rewritten pages. It is already the gate in Amendment 2026-10-16b.

## 5. Code or process changes

| Item | Status | Evidence | Size (files; on the contract list?) |
|---|---|---|---|
| Re-acceptance when the terms version changes | missing | `account/handler.ts:223-247`, `console.js:3127` | M (4: `account.rs`, handler, console, tests; **yes**, `account.rs`) |
| Email to accounts on a material policy change | missing | no job | S–M (1 function and a cron; no) |
| Sweep consent rows 3 years after the account ends | missing | no cron | S (1 migration; no) |
| Keyed hash (HMAC with a server secret) for tombstone and consents | partial | `crypto.ts:31-33` | M (migration and 2 functions; no) |
| Checkout consent text naming the renewal terms; consent record keeps plan and text shown | partial | `billing-checkout/handler.ts:66-67, 121-128` | S (2; no, but money, so Contract workflow) |
| California acknowledgment email after checkout | missing | `stripe-webhook` | M (2; no) |
| Trial reminder 3 days before the first charge (P14) | missing | `billing-jobs` | S–M (2; no) |
| Pause-resume notice with real lead time | partial | `billing-jobs/handler.ts:146-161` | S (1; no) |
| Managed Payments parameter, or Managed Payments off | unknown | `billing-checkout/handler.ts:52-71` | S (2; no) |
| Move billing to the signed prices | missing | `billing-checkout/handler.ts:16-20`, `billing-jobs/handler.ts:71-75` | L (≥8; **yes**, `account.rs`) |
| Fix the page's log count, or the code's | false | `scheduler.rs:35` | S (1; no) |
| Make the app's Google copy match the page | partial | `index.html:129` | S (1; no) |
| Export: judgments, Google connection, Gmail proposals | partial | `account/index.ts:157-189` | S–M (1; no) |
| Knowlu-initiated closure: 30-day hold and pro-rata refund | missing | none | M (admin function; no) |
| Retention limits on telemetry, corrections and judgments | missing | migrations | M (1 migration; no) |
| Incident plan (30/45 days, AG notice) and an email-deletion runbook | missing | none in `docs/` | S (docs; no) |
| Coursework pacing and stop-on-block, or drop the T:50 claim | can't verify | `engine/src/zybooks.rs`, `vhl.rs` | S–M (2; no) |
| AWS lifecycle rule captured in the repo | partial | `sweep.ts:5` | S (1 doc; no) |

## 6. What a lawyer should still check before sales open

- The entity and seller of record under Managed Payments, and what the terms must say if Link is the seller.
- Placement and wording of the auto-renewal consent across California, New York and Colorado; the acknowledgment; notice windows (trial, price change, the shortened-year renewal under P7).
- The money-back wording against 16 CFR 239.3, and the "half price" comparison.
- Pell conditional pricing and handling of the award letter.
- Gift codes against federal and state gift-card rules.
- Mailbox actions with saved sessions, against Google's and Microsoft's terms and university policies on forwarding.
- zyBooks and VHL AI-processing clauses against model judging of coursework titles.
- Whether "Alabama courts" holds against out-of-state consumers, and whether the liability cap holds for $0 trial users.
- Breach clocks by state, and the Alabama § 8-38 security programme.
- Unverified age-of-majority states (Nebraska, Mississippi).

Sources:
- [Jones Day: FTC revives click-to-cancel](https://www.jonesday.com/en/insights/2026/05/ftc-revives-clicktocancel-rule-new-risks-for-subscription-businesses)
- [Gibson Dunn: FTC restarts negative option rulemaking](https://gibsondunn.com/ftc-restarts-negative-option-rulemaking-after-eighth-circuit-vacatur-enforcement-under-rosca-continues/)
- [Barnes & Thornburg: California expands auto-renewal law](https://btlaw.com/en/insights/alerts/2025/california-expands-automatic-renewal-law-new-requirements-now-in-effect)
- [Google API Services User Data Policy](https://developers.google.com/terms/api-services-user-data-policy)
- [Google Workspace API user data policy](https://developers.google.com/workspace/workspace-api-user-data-developer-policy)
- [Google restricted scope verification](https://developers.google.com/identity/protocols/oauth2/production-readiness/restricted-scope-verification)
- [Nylas: Google security assessment guide](https://developer.nylas.com/docs/provider-guides/google/google-verification-security-assessment-guide/)
- [Hunton: Alabama privacy law](https://www.hunton.com/privacy-and-cybersecurity-law-blog/alabama-becomes-21st-state-with-comprehensive-consumer-privacy-law)
- [FPF: Alabama Personal Data Protection Act](https://fpf.org/blog/the-alabama-personal-data-protection-act-brings-consumer-privacy-to-the-heart-of-dixie/)
- [CPPA 2025 thresholds](https://cppa.ca.gov/announcements/2024/20241217.html)
- [Stripe: how Managed Payments works](https://docs.stripe.com/payments/managed-payments/how-it-works)
- [Stripe: update Checkout for Managed Payments](https://docs.stripe.com/payments/managed-payments/update-checkout)
- [FTC CAN-SPAM guide](https://www.ftc.gov/business-guidance/resources/can-spam-act-compliance-guide-business)
- [16 CFR 239.3 (LII)](https://www.law.cornell.edu/cfr/text/16/239.3)
- [PRC breach notification survey 2026](https://privacyrights.org/resources-tools/reports/data-breach-notification-laws-50-state-survey-2026-edition)
- [NY S5615](https://www.nysenate.gov/legislation/bills/2025/S5615)
- [Supabase Auth audit logs](https://supabase.com/docs/guides/auth/audit-logs)
- [knowlu.com](https://knowlu.com/)