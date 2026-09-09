# Knowlu in the cloud — the legal landscape (research briefing)

**Date:** 2026-09-09. **Status:** research note, not legal advice. Written so the founder can brief a
lawyer efficiently and make product decisions now. Every source was read on 2026-09-09 unless a
different date is given; where a page could not be fetched the note says so and relies on the next
best source, marked. Nothing here was invented; "could not verify" means exactly that.

**The product this is about** (decision of 2026-09-09): accounts required (Supabase Auth, email),
$9.99/month via Stripe, no free tier, one edition; a US-hosted cloud backend (Cloudflare + Supabase)
that runs all AI judgment on the student's data — LMS `.ics` feed classification, read-only Gmail
triage, campus-event relevance — through a commercial inference API under zero-retention terms;
coursework portals (zyBooks, VHL Central) scraped today on the student's own device with credentials
in Windows Credential Manager, with the question open whether that may move server-side; analytics
of three kinds ((a) product events, (b) the student's corrections to AI judgments, (c) opt-in raw
content for model improvement) plus in-app issue reports with diagnostic context; pilot at the
University of Alabama, then the University of Kentucky, then other US campuses; many users aged
17–19; the founder is himself a UA freshman.

Background read for this note: `VISION.md` and the market document's §4–§5 and §8
(`docs/superpowers/notes/2026-09-01-market-pricing-and-distribution.md`). Nothing under `state/`,
`tasks/`, `approvals/`, `config/` or `profile/` was opened.

---

## 0. Executive summary

Risk ranks: **BLOCKING** = do not ship the feature as designed until handled;
**BEFORE-WIDENING** = must be in place before the pilot goes beyond a handful of known testers (or,
where marked, before the first paid signup); **LATER** = real but not yet triggered, or a watch item.

| Area | Risk | The one decision it forces | Cheapest compliant path |
|---|---|---|---|
| 5. Gmail restricted scope (`gmail.readonly`) | **BLOCKING** for Gmail beyond 100 users | Budget 6+ weeks and an annual CASA assessment before Gmail leaves testing; **exclude all Gmail-derived content from any cross-user model training, opt-in or not** | Verify once, aim for CASA AL1, keep Gmail data server-transient, add the Limited Use sentence to the privacy policy, keep Gmail features "prominent in the UI" |
| 6. Portal scraping (zyBooks, VHL) | **BLOCKING** if moved server-side; **LATER** if kept on-device | Keep credential-based scraping on the student's device; never store portal passwords in the cloud | Status quo (Credential Manager, deterministic parser), rate-limit, stop on vendor block, disclose the ToS tension to the student |
| 2. Minors / contracts | **BEFORE-WIDENING** | Paid accounts require the buyer to be 18+ (Alabama's 2019 amendment makes 18-year-olds bindable); block 17-year-olds or route them to a parent-pays flow | One age attestation checkbox ("I am 18 or older") at signup; store a boolean, not a birthdate |
| 8. Subscriptions (ROSCA, CA ARL, NY 527-a) | **BEFORE FIRST PAID SIGNUP** | Design the checkout and cancel flow to California's July 2025 standard everywhere | Stripe Checkout terms checkbox + Stripe Customer Portal cancel link in-app and in every billing email + annual reminder email + 3-year consent log |
| 3/7. Privacy notice and analytics (a)(b) | **BEFORE-WIDENING** | Publish one privacy policy meeting CalOPPA, disclosing (a) and (b) plainly, before any non-founder user | One policy, rights (access/delete/correct/export) implemented for everyone, "we do not sell or share" stated, no geo-fencing |
| 7c. Opt-in raw content for training | **BEFORE-WIDENING** (if offered) | A separate, unbundled, revocable toggle, default off, scoped to content that is not Gmail-derived | Own toggle screen, plain list of what/why/how long, consent log with version, delete-on-revoke |
| 3. Alabama breach law (Ala. Code § 8-38) | **BEFORE-WIDENING** | We become a custodian of "sensitive personally identifying information" the moment we hold email+password pairs or OAuth tokens: write the security program and the 45-day breach plan | Encrypt tokens at rest, no portal passwords server-side, a one-page incident plan |
| 3. State comprehensive privacy laws | **LATER** (none triggered at pilot scale) | Build the rights universally now so thresholds never matter; watch Alabama APDPA (25,000 consumers, effective 2027-05-01) and Texas/Nebraska (no numeric threshold; small-business exemption) | Universal rights + no sale of data |
| 1. FERPA | **LATER / none** | None: FERPA binds institutions, not a student-chosen app; keep the ICS feed + OAuth architecture and never touch LMS API tokens | Nothing to build; expect universities to police through acceptable-use rules and Workspace app controls, not FERPA |
| 4. Student-data laws (SOPIPA family) | **none** | None: every statute found is scoped to K-12 | Never market to K-12 |
| 10. University policies | **LATER** | Never request or hold myBama/linkblue credentials (already true); design Gmail ingest to survive a UA Workspace admin blocking third-party apps for `@crimson.ua.edu` | Personal-Gmail fallback, visible failure |
| 9. Issue reports | **LATER** | Show the payload, scrub tokens/emails, let the user edit, keep it briefly | A preview screen |
| 8. Sales tax | **BEFORE FIRST KENTUCKY SALE** | Kentucky taxes SaaS at 6% (since 2023-01-01); Alabama very likely does not — confirm with a CPA | Stripe Tax with KY registration; get an Alabama answer in writing |

The three findings most likely to change the product's shape are in §11 at the end.

---

## 1. FERPA

**What the law says.** FERPA is a spending-clause statute aimed at institutions:
"No funds shall be made available under any applicable program to any educational agency or
institution which has a policy or practice of permitting the release of education records..."
(20 U.S.C. § 1232g(b)(1)). The regulations "apply to an educational agency or institution to which
funds have been made available under any program administered by the Secretary" (34 CFR § 99.1(a)).
"Education records" are records that are "(1) Directly related to a student; and (2) Maintained by an
educational agency or institution or by a party acting for the agency or institution" (34 CFR
§ 99.3; the statute at § 1232g(a)(4)(A) uses the same "maintained by" test).

**Does it bind Knowlu?** No. Knowlu receives no Department of Education funds, is not an
educational agency or institution, and — decisively — is not "a party acting for the agency or
institution." Data a student pastes or authorizes into Knowlu (an `.ics` URL, an OAuth grant on
their own mailbox) is held by Knowlu on the student's behalf, not the university's. The Department
of Education's own vendor guidance (PTAC FAQ-5, *Responsibilities of Third-Party Service Providers
under FERPA*, August 2015) is framed entirely around services "typically procured through a contract
or formal written agreement" and says FERPA continues to govern the provider's use only "when PII
from education records is disclosed to the provider" by the school under the school-official
exception; it does not contemplate a student-chosen app because the statute has no hook for one.

**The school-official exception is irrelevant to us.** 34 CFR § 99.31(a)(1)(i)(B) lets an institution
treat an outside party as a "school official" only if it "performs an institutional service or
function for which the agency or institution would otherwise use employees," "is under the direct
control of the agency or institution with respect to the use and maintenance of education records,"
and is bound by § 99.33(a) on redisclosure. Knowlu satisfies none of these and should not want to:
becoming a school official is the tier-3 institutional posture the market document rejects (§6.2 —
"FERPA posture flips ... a one-way door per school").

**What universities police instead.** They cannot reach Knowlu under FERPA, but they can reach
their own students and their own systems, and the 2024–2026 record shows them doing exactly that
for Canvas personal access tokens:

- **Cornell:** "The manual Canvas API access token service was decommissioned on April 2, 2024";
  students "should not provide your API access token to any individual, vendor, application, or
  bot"; the stated reason is that a student token exposes *other* learners' "course enrollment,
  discussion posts, grades (from group assignments), and more."
- **Penn:** "Beginning in 2025, Penn no longer permits student creation of API tokens or generation
  of tokens on behalf of students in Canvas ... in order to minimize security and data privacy risks
  and ensure compliance with the University's Policy on Acceptable Use of Electronic Resources."
- **UW-Madison:** "As of November 14th, 2025, student users in the UW-Madison instance of Canvas can
  no longer manually generate Access Tokens"; an admin-issued token expires within 90 days.
- **Penn State (May 20, 2026):** "Penn State disabled the ability for users to manually create access
  tokens in Canvas"; faculty and staff request one through the Help Desk.
- **Texas A&M:** tokens "not available for students or development purposes"; sharing with
  third-party vendors "may not be in compliance with Standard Administrative Procedure 29.01.03";
  "any existing access with third-party vendors should be deleted immediately" (the TAMU page moved
  and both URLs returned 404 on 2026-09-09; wording is from the search index and the market document
  §4.3, which read the live page on 2026-09-01).

The pattern is consistent: the institutional objection is *other students' data*, and the tool is
acceptable-use policy plus a technical switch. That is precisely why the market document ruled LMS
tokens out (§4.3) and this note agrees.

**Does the student's own `.ics` feed URL raise a FERPA issue?** No. The feed is a vendor-built,
institution-enabled feature whose whole purpose is for the student to hand the URL to a third-party
calendar (Google, Apple, Outlook). It carries only that student's items. The institution has already
decided to make this disclosure route available to the student; the student's onward use is the
student's. Two practical cautions that are not FERPA: the URL is a bearer secret (anyone holding it
can read the feed), so store it encrypted and never log it; and if UA's Blackboard administrators
ever disable feed sharing, Knowlu must fail visibly rather than silently.

**Does our Gmail read raise one if a professor's email about another student's grade is in the
inbox?** Not for Knowlu. If such an email exists, the disclosure was the professor's (and the
institution's compliance problem) at the moment it was sent; the copy in student A's mailbox is
student A's correspondence, and A's OAuth grant to Knowlu is A's decision about A's data. Knowlu is
not "maintaining" the institution's record. The exposure that *does* exist is under Google's Limited
Use rules (§5) and general privacy law (§7): another person's data will pass through Knowlu's
classifier, so Knowlu should (i) never surface, store or train on content about third parties beyond
what the triage feature needs, and (ii) drop message bodies after classification. Note also that
UA's Crimson mail is Google Workspace for Education; a Workspace administrator can block third-party
OAuth apps for the domain (see §10), which is the realistic way a university would stop Gmail ingest
— not FERPA.

**Risk rank:** LATER / none. **Avoid:** LMS API tokens, any "school official" positioning, and
marketing copy that says "FERPA-compliant" (it is a category error for a consumer app and reads as a
claim we cannot substantiate).

---

## 2. Age of majority and minors

**Alabama.** Ala. Code § 26-1-1(a) sets majority at 19. But subsection (f), added by Act 2019-447
(effective 2019-09-01), provides: "An unemancipated minor who is 18 years old and of sound mind,
notwithstanding his or her minority, may enter into a binding contract as may be exercised by an
individual of full legal age. The minor, by reason of his or her minority, may not rescind, avoid, or
repudiate the contract or rescind, avoid, or repudiate any exercise of a right or privilege under the
contract." So in Alabama an 18-year-old's click-wrap ToS and subscription are binding; a
17-year-old's are voidable at the minor's election under ordinary contract law. The practical
consequence of voidability is a refund (and unenforceable arbitration/limitation clauses against that
user), not a penalty.

**Kentucky.** KRS 2.015: "persons of the age of eighteen (18) years are of the age of majority for all
purposes in the Commonwealth except for the purchase of alcoholic beverages and for purposes of care
and treatment of children with disabilities, for which twenty-one (21) years is the age of
majority." Contracts of a minor are voidable at the minor's election (common law; see the 1980
Attorney General opinion OAG 80-256 surfaced in research). Other pilot states: 18 in nearly all;
**Nebraska is 19 and Mississippi is 21 — could not verify from primary sources in this pass; check
before entering either.**

**COPPA does not apply.** The FTC's COPPA FAQ: "The Rule was designed to protect children under age
13" and "Although COPPA does not apply to teenagers, the FTC is concerned about teen privacy."
Coverage is triggered only for services "directed to children under 13" or with "actual knowledge"
of collecting from an under-13. A university-student product with an 18+ gate is neither. (The
amended COPPA Rule was published 2025-04-22, effective 2025-06-23, with full compliance due
2026-04-22 — irrelevant to us but it now requires *separate* parental consent for third-party
disclosures including AI training, a signal of where regulators are heading on §7.)

**State minors' laws — do they reach a productivity app?** Each one found is scoped by feature set,
and Knowlu has none of the features:

- **California AADC (CAADCA).** Ninth Circuit, *NetChoice v. Bonta*, No. 25-2366 (2026-03-12):
  narrowed the preliminary injunction — the coverage definition and age-estimation provision are no
  longer enjoined; the data-use restrictions and dark-patterns prohibition remain enjoined as likely
  unconstitutionally vague; remanded. The Act applies to a "business" as defined by the CCPA, so it
  cannot reach Knowlu before the CCPA thresholds in §3 are met (could not re-verify the cross-reference
  in this pass; it is how every summary read describes it).
- **Texas SCOPE Act (HB 18, Bus. & Com. Code ch. 509).** § 509.002 applies only to a digital service
  that "connects users in a manner that allows users to socially interact with other users," lets a
  user "create a public or semi-public profile," **and** lets a user "create or post content that can
  be viewed by other users." Knowlu fails all three. Parts of the Act are enjoined (Pitman, J.,
  2024-08-30; on appeal).
- **Utah Minor Protection in Social Media Act (Utah Code § 13-71-101).** "Social media service" is a
  public site or app that displays primarily user-generated content, permits public-facing profiles,
  connects account holders for social interaction, and allows posting viewable by others; it
  expressly "does not include: (i) email; (ii) cloud storage; or (iii) document viewing, sharing, or
  collaboration services." Preliminarily enjoined (Shelby, J., 2024); appeal pending in the Tenth
  Circuit.
- **Florida HB 3** targets social media platforms with addictive features; enforceable after the
  Eleventh Circuit stayed the district-court injunction in November 2025. Not a productivity app.
- **Arkansas Social Media Safety Act** — permanently enjoined (March 2025).
- **Nebraska Age-Appropriate Online Design Code Act** (operative 2026-01-01) covers "covered online
  services" above revenue/scale thresholds (summaries cite >$25M revenue among the criteria) and
  exempts services with actual knowledge that fewer than 2% of users are minors. Not triggered at
  pilot scale; watch if Nebraska is ever a campus.
- **App Store Accountability Acts** (Texas SB 2420 — enjoined 2025-12-23, then the injunction was
  stayed by the Fifth Circuit and the Supreme Court declined to vacate the stay, so it is in effect;
  Utah; Louisiana; **Alabama HB 161, signed 2026-03-09**, requiring app stores to age-categorize
  accounts, with existing accounts as of 2026-10-02 to be categorized by 2027-10-01). These regulate
  *app stores* and developers distributing through them. Knowlu is a direct-download Windows installer
  and is outside that channel. **If it is ever listed in the Microsoft Store, re-check whether that
  store and this product fall within each Act's definitions.**

**What comparable student products do (ToS age clauses).**

- **Quizlet:** "The Service is intended for users over the age of 13, but is open to all ages. For
  children age 13 or younger ... Quizlet offers a restricted feature set" (quizlet.com/tos, quoted via
  the search index; the page itself returned 403 to the fetcher).
- **Chegg:** "If you are between the ages of 13 and the age of majority where you live, you must
  review these Terms of Use with your parent or guardian to confirm that you and your parent or
  guardian understand and agree to it" (chegg.com/en-US/termsofuse, via the search index; 403 on
  fetch).
- **Notion:** "you may not use the Service if you are 13 years of age or younger. By using the
  Service, you represent and warrant that you are over the age of 13" (Personal Use Terms of Service,
  via the search index; the page is JS-rendered).

None of these gate at 18. They accept the voidability exposure for 13–17-year-olds, and Chegg papers
over it with a parental-review representation. That is the industry norm, and it is the norm because
the downside is a refund.

**Cheapest compliant path.** Gate *paid* accounts at 18 (not 19 — Alabama's § 26-1-1(f) makes 18
sufficient, and every other pilot state is 18). Implement it as an attestation checkbox at signup
and store only a boolean plus timestamp; do not collect a birthdate (a birthdate is personal data
you then have to protect and it buys nothing). A 17-year-old freshman is a real but small cohort for
roughly one semester; if that cohort matters, add a parent-pays flow (the parent is the Stripe
customer and the contracting party) rather than a parental-consent form. Keep the arbitration and
limitation-of-liability clauses but expect them not to bind a minor.

**Avoid:** age *verification* (ID upload, face estimation) — it is not required by any law that
reaches us, it creates sensitive data, and CAADCA's age-estimation clause, though no longer enjoined,
does not apply below the CCPA thresholds.

**Risk rank:** BEFORE-WIDENING.

---

## 3. State comprehensive privacy laws, CalOPPA, and Alabama

**Thresholds (as read 2026-09-09; annual figures; "consumers" means that state's residents).**

| State | Law / effective | Applies if you… | Pilot-scale trigger? |
|---|---|---|---|
| California | CCPA/CPRA (2020/2023) | >$25M revenue (CPI-indexed; ~$26.6M from 2025 — verify) **or** 100,000 consumers/households **or** ≥50% revenue from selling/sharing PI | No |
| Virginia | VCDPA (2023-01-01) | 100,000 consumers, or 25,000 + >50% revenue from sales | No |
| Colorado | CPA (2023-07-01) | 100,000, or 25,000 + any revenue from sales | No |
| Connecticut | CTDPA (2023-07-01; amended) | 35,000 consumers (from 2026-07-01), or 25,000 + 25% revenue from sales | No |
| Utah | UCPA (2023-12-31) | $25M revenue **and** (100,000, or 25,000 + 50%) | No |
| Texas | TDPSA (2024-07-01) | No numeric threshold: anyone who "conducts business in this state or produces a product or service consumed by residents" and "processes or engages in the sale of personal data," **except** a small business as defined by the SBA (Tex. Bus. & Com. Code § 541.002); small businesses must still get consent before *selling* sensitive data (§ 541.107) | Exempt as a small business; we do not sell |
| Oregon | OCPA (2024-07-01) | 100,000, or 25,000 + 25% | No |
| Montana | MCDPA (2024-10-01) | 25,000, or 15,000 + 25% | No |
| Florida | FDBR (2024-07-01) | $1B revenue plus platform conditions | No |
| Delaware | DPDPA (2025-01-01) | 35,000, or 10,000 + 20% | No |
| Iowa | ICDPA (2025-01-01) | 100,000, or 25,000 + 50% | No |
| Nebraska | NDPA (2025-01-01) | Texas model: no numeric threshold, SBA small-business exemption | Exempt |
| New Hampshire | NHPA (2025-01-01) | 35,000, or 10,000 + 25% | No |
| New Jersey | NJDPA (2025-01-15) | 100,000, or 25,000 + 50% | No |
| Tennessee | TIPA (2025-07-01) | $25M revenue **and** (175,000, or 25,000 + 50%) | No |
| Minnesota | MCDPA (2025-07-31) | 100,000, or 25,000 + 25% | No |
| Maryland | MODPA (2025-10-01) | 35,000, or 10,000 + 20% | No |
| Indiana | INCDPA (2026-01-01) | 100,000, or 25,000 + 50% | No |
| **Kentucky** | **KCDPA (2026-01-01)** | 100,000 Kentucky consumers, or 25,000 + 50% revenue from sales; sensitive data needs opt-in consent; 30-day cure; up to $7,500 per violation (KY AG) | No — UK's whole undergraduate body is ~22,000 |
| Rhode Island | RIDTPPA (2026-01-01) | 35,000, or 10,000 + 20% | No |
| Oklahoma | OKCDPA (2027-01-01) | 100,000, or 25,000 + 50% | No |
| **Alabama** | **APDPA, HB 351, signed 2026-04-17, effective 2027-05-01** | >25,000 Alabama consumers (payment data excluded) **or** >25% revenue from sales; AG-only enforcement, 45-day cure, up to $15,000 per violation; exemptions reported for higher-education institutions and for small businesses (<500 employees) and nonprofits that do not sell data — **verify the small-business exemption in the enrolled text before relying on it** | Not before 2027; a full UA deployment (~40,000 students) could cross 25,000 |

Sources for the table: the Texas statute (texas.public.law mirror of § 541.002), the Kentucky
Attorney General's KCDPA page, the Hunton and DLA Piper summaries of Alabama HB 351, and two
tracker tables (PrivacyLawMap, Clym) cross-checked against MultiState's 2026 list. Trackers agree
on the numbers above; where they differ from the statute the statute wins, so confirm any state you
actually enter.

**Plain conclusion.** A pilot of a few thousand paying students triggers **none** of these laws. The
two "no-threshold" states (Texas, Nebraska) exempt SBA-defined small businesses, and their residual
duty (consent before *selling* sensitive data) is met by not selling anything. Alabama's own law
does not start until 2027-05-01 and has the lowest count in the country (25,000), so it is the first
one likely to matter — and only at roughly the scale of the whole UA student body.

**Rights to implement universally anyway** (cheaper than geo-fencing, and every one of them is
already a feature a privacy-skeptical student expects): confirm/access, delete (account deletion
that actually purges, with a stated backup-retention tail), correct, export (portable copy — the ICS
export and journal in VISION already are that), and a plain statement that Knowlu does not sell or
share personal data for targeted advertising (which makes opt-out links and Global Privacy Control
handling unnecessary). Treat "sensitive data" (race, religion, health, sexual orientation,
citizenship/immigration status, genetic/biometric, precise geolocation, data of a child under 13 —
the Virginia-model list every state uses) as opt-in: Knowlu does not ask for any of it, but Gmail
triage will incidentally *read* it, so the Gmail connection screen should be the consent (see §7).
Note that California alone lists "the contents of a consumer's mail, email, and text messages unless
the business is the intended recipient" as *sensitive personal information* (Cal. Civ. Code
§ 1798.140(ae) — cited from memory of the statute; verify the paragraph letter) — meaning that once
Knowlu is a CCPA "business," Gmail triage is sensitive-PI processing with a right-to-limit. Design
for it now.

**CalOPPA applies at any size.** Cal. Bus. & Prof. Code § 22575(a): "An operator of a commercial Web
site or online service that collects personally identifiable information through the Internet about
individual consumers residing in California ... shall conspicuously post its privacy policy."
"Operator" is "any person or entity that owns a Web site located on the Internet or an online service
that collects and maintains personally identifiable information from a consumer residing in
California" (§ 22577). No threshold. Required contents (§ 22575(b)): categories of PII collected and
third parties it may be shared with; any process to review and request changes; how material
changes are notified; the effective date; how "do not track" signals are handled; and whether third
parties may collect data across sites. The first Californian who signs up makes this mandatory, so
write the policy to CalOPPA from day one.

**Alabama has no comprehensive law in force today — only breach notification** (the APDPA above
arrives 2027-05-01). The **Alabama Data Breach Notification Act of 2018, Ala. Code § 8-38-1 et
seq.**, in summary:

- *Covered data* (§ 8-38-2): "sensitive personally identifying information" = an Alabama resident's
  first name/initial + last name in combination with SSN/tax ID, government ID number, financial
  account number with access code, medical/health-insurance information, **or "a username or email
  address, in combination with a password or security question and answer that would permit access
  to an online account."** Every Knowlu account (email + password hash) and every stored portal
  credential is therefore SPII; encrypted data is out of scope unless the key is also taken.
- *Duty of care* (§ 8-38-3): implement and maintain "reasonable security measures" — the statute
  lists factors (designated responsible employee, risk identification, safeguards, service-provider
  contracts, board/management reporting). DWT's summary did not reproduce § 8-38-3; the factor list
  is from the statute as summarized by several firms — verify against the code.
- *Investigation and notice* (§§ 8-38-4, -5): notify affected individuals "as expeditiously as
  possible ... but no later than 45 calendar days" after determining a breach reasonably likely to
  cause substantial harm; document the determination and keep it five years.
- *Regulator and others* (§§ 8-38-6, -7, -8): notify the Attorney General within 45 days when more
  than 1,000 Alabama residents are notified; consumer reporting agencies at that scale; a
  **third-party agent** (a processor such as Supabase or Cloudflare) must notify the covered entity
  "no later than 10 days" after discovering a breach — put that in the vendor terms review.
- *Penalties* (§ 8-38-9): civil penalties (secondary sources cite up to $5,000 per day, capped at
  $500,000 per breach); **no private right of action**. Disposal duty in § 8-38-10.
- Kentucky's counterpart is KRS 365.732 (not fetched in this pass).

**Risk rank:** breach-law duties BEFORE-WIDENING; comprehensive laws LATER. **Avoid:** collecting
anything you would then have to classify as sensitive; storing portal passwords server-side (§6);
calling any data "anonymous" that is keyed to an account id (it is pseudonymous, and the FTC treats
"anonymous" as a factual claim — §7).

---

## 4. Student-data privacy laws (the SOPIPA family)

These statutes regulate *operators of services used by schools*, and every one found is scoped to
K-12 by definition. Three definitions:

- **California SOPIPA, Bus. & Prof. Code § 22584:** "'Operator' means the operator of an Internet Web
  site, online service, online application, or mobile application with actual knowledge that the
  site, service, or application is used primarily for K-12 school purposes and was designed and
  marketed for K-12 school purposes." "K-12 school purposes" are "purposes that customarily take
  place at the direction of the K-12 school, teacher, or local educational agency."
- **Connecticut, Conn. Gen. Stat. § 10-234aa:** "operator" is one who operates a site, service or app
  "with actual knowledge that [it] is used for school purposes and was designed and marketed for
  school purposes"; "student" means a Connecticut resident "enrolled in grades kindergarten to
  twelve, inclusive, in a public school" (or preschool/IEP/board-of-education categories).
- **Kentucky, KRS 365.734 (HB 232, 2014):** duties fall on a "cloud computing service provider" that
  "enters into an agreement to provide cloud computing services to an educational institution," and
  "educational institution" is a unit "serving students in kindergarten to grade twelve (12)."

**Does any reach a consumer app used by university students?** No. Two limbs fail at once: Knowlu
is neither *designed and marketed for K-12 school purposes* nor *contracting with a school*. Even a
17-year-old user does not change the product's design or marketing. **Alabama has no SOPIPA-style
statute**; the State Board of Education's Data Use and Governance Policy is an administrative policy
about the Board's own data. Roughly forty states have such laws; the definitions above are
representative of the two models (California's operator model, Kentucky's contract model), and no
higher-ed-only variant was found.

**Risk rank:** none. **Avoid:** any marketing to high-school seniors or dual-enrollment students
that could be read as "designed and marketed for K-12 school purposes" — that is the only path in.

---

## 5. Google API Services User Data Policy and Gmail restricted scopes

**Classification.** `https://www.googleapis.com/auth/gmail.readonly` ("View your email messages and
settings") is a **restricted** scope; so is `gmail.metadata`. `gmail.labels` is non-sensitive. Any
Gmail read that sees bodies or headers is restricted.

**What restricted scopes require.**

1. *Brand and scope verification* — privacy policy, homepage, justification, demo video. Google's
   FAQ gives an overall figure of about **6 weeks** for restricted-scope verification and the
   developer page says it "can potentially take several weeks to complete."
2. *The security assessment.* "Every app that requests access to Google users' restricted data and
   has the ability to access data from or through a third-party server must go through a security
   assessment" (restricted-scope verification page). It is run under the App Defense Alliance's
   **CASA** framework by an authorized lab at assurance level **AL1 or AL2** (the help page: "Once an
   application has been validated at the highest level (AL2), it will continue to be validated at
   that level in subsequent years"); the outcome is a Letter of Validation/Assessment; "apps must be
   reverified for compliance and complete a security assessment at least every 12 months after your
   assessor's Letter of Assessment (LOA) approval date," and "the required assurance level is
   dynamic and may increase based on changes in your user base or data-handling practices." AL2
   "tests the application, the application deployment infrastructure and any user data storage
   location."
3. *Cost.* Google: "The cost for such a service is agreed on between the developer and the assessor
   without any involvement from Google." Vendor-published figures (a 2026 DeepStrike article quoting
   lab price lists; treat as indicative, not verified): **$540–$1,800 for AL1 (the old "Tier 2")**,
   about **$4,500 for AL2 (the old "Tier 3")**, 1–3 weeks and 2–4 weeks respectively once testing
   starts, annually. Other blogs quote five-figure sums; those describe larger scopes of assessment
   and remediation, not the lab fee. A self-scan (AL0) no longer yields a certificate.
4. *The trigger.* Verification and assessment are required for a **production** app regardless of
   user count; what the user count changes is how long you can avoid them:
   - **Testing** publishing status: "limited to up to 100 test users listed in the OAuth consent
     screen," and "Authorizations by a test user will expire seven days from the time of consent. If
     your OAuth client requests an `offline` access type and receives a refresh token, that token
     will also expire." A daily background Gmail sync therefore breaks weekly in testing mode; it is
     good for demos and useless for a pilot.
   - **In production, unverified:** "100 new users in total, after the app presents the unverified
     app screen." That is the real ceiling.

**Does keeping Gmail processing on-device avoid the assessment?** Google's own sentence limits the
assessment to apps that have "the ability to access data from or through a third-party server," and
the verification exemptions listed are personal use, dev/test, service-account-own-data, internal
Workspace use and domain-wide install — not "local-only." A Google community manager stated on
2026-03-24 that "while there were historically nuances for 'local-only' applications, the current
Google API Services User Data Policy effectively requires a security assessment for *any*
application that accesses Restricted Scopes in a production environment (beyond 100 users)" and that
"the 'local-only' nature of an app does not automatically waive the requirement." **Conclusion: do
not plan on on-device processing to escape CASA.** It may still lower the assurance level (no server
storage of Gmail data to assess) and it certainly shrinks the breach surface, but it does not remove
verification, and Google's practice in 2026 is to require the assessment above 100 users either way.
Since the redirected product processes Gmail server-side anyway, budget for verification + AL1/AL2.

**Limited Use — the part that changes plan (c).** The Google Workspace API user data and developer
policy (last updated 2026-07-22), which is the product-specific policy for Gmail scopes, requires
developers to "limit your use of data to providing or improving your appropriate use case or
features that are visible and prominent in the requesting application's user interface," restricts
transfers to user-consented user-facing features, security, legal compliance or a merger with
explicit consent, and among prohibited uses lists, verbatim:

> "Transferring, selling, or using user data to create, train, or improve a machine learning or
> artificial intelligence model beyond that specific user's personalized model for the appropriate
> use case or user-facing feature."

Human access is allowed only when "you have obtained and documented the user's explicit consent or
affirmative agreement to view specific messages, files, or other data," for aggregated/anonymized
internal operations, for security, or by law. Transfers for advertising, credit, lending or resale
are "completely prohibited." The general API Services User Data Policy (last updated 2024-02-15)
carries the same Limited Use frame and requires that apps with restricted scopes "pass an annual
security assessment and obtain a Letter of Assessment from a Google-designated third party."

*Read against the product:* Gmail-derived content (bodies, subjects, senders, and features derived
from them) may be used to run and improve **the triage feature the user sees** and to build **that
user's own personalized model or rules** — which is exactly the rule-promotion loop in VISION — but
**may not be pooled across users to train or improve a general model, and an opt-in does not cure
that**, because the clause is a prohibited *use*, not a transfer that consent can unlock. The
"transfer with user consent for a user-facing feature" exception covers sending a message to the
inference provider to classify it for that user; it does not cover retaining it as training data.
Plan (c) must therefore exclude everything Gmail-derived. Plan (b) (corrections) is fine as long as
what is logged is the label, the item id and non-content features — never the email text.

**Disclosure text.** Google requires "an affirmative statement in [the] application or on a website"
that use of the data complies with Limited Use, e.g. "The use of information received from Google
Workspace scopes will adhere to the Google User Data Policy, including the Limited Use
requirements." Put that sentence in the privacy policy and on the Gmail connection screen.

**Cheapest compliant path.** One OAuth client; `gmail.readonly` only (do not add `gmail.modify` for
labels — it widens the review); keep Gmail data transient on the server (classify, write the
proposal, discard the body); document data deletion on account close (the assessment specifically
checks "deleting user data upon user request"); publish the policy with the Limited Use sentence;
start verification the week the backend exists, because 6+ weeks is the floor and the pilot cannot
cross 100 Gmail users before the letter arrives. Keep the Gmail feature "visible and prominent" in
the UI — the whole Limited Use test hangs on it.

**Avoid:** testing-mode "pilots" (7-day token expiry); any wording in the policy or code that
retains Gmail bodies for "product improvement"; letting anyone at Knowlu read a user's messages
without a per-message, documented consent (support tooling must not have that button).

**Risk rank:** BLOCKING for Gmail beyond 100 users; the training exclusion is a product decision to
make now.

---

## 6. Scraping coursework portals with the student's credentials

**What the vendors' terms say (both read 2026-09-09).**

*zyBooks Terms of Use (last updated 2025-11-07):*
- Prohibits "Using any scraper, crawler, spider, robot or other automated means of any kind to
  access to copy this Site, the Services, the Materials, Content, data therein" and bypassing robot
  exclusion headers.
- "Subscribers are solely responsible for the security and confidentiality of their account names
  and passwords and for all activity using their account name and password."
- Prohibits "allowing another person to use the Site or Services using your account name and
  password."
- Prohibits "Using or enabling artificial intelligence technologies and tools to ingest, train, test,
  analyze, process, copy, distribute, make publicly accessible, and/or generate output ... based on
  this Site, the Services, the Materials and/or Content," including "indirectly (e.g., through the
  use of third-party plugins, extensions or decentralized custom chatbots)."
- Prohibits attempts "to breach security or authentication measures without proper authorization."

*Vista Higher Learning / VHL Central Terms of Use (page shows "Last Modified May 10, 2024" and an
effective date of 2025-11-14):*
- "You are responsible for controlling the use and maintaining the confidentiality of all Product
  Access Codes, Login Credentials and other passwords"; "You are fully responsible for all
  activities that occur using Your Product Access Code and Login Credentials."
- "You will not resell, license, assign, share or otherwise transfer any Product Access Code or Your
  Login Credentials to or with any other person"; any such transfer "shall be void."
- "You may not attempt to gain unauthorized access to any portion or feature of the VHL Site ... by
  any means, including without limitation, hacking or password 'mining'."
- "You may not ingest or otherwise use all or any portion of the Site Content or other VHL IP in
  connection with any A.I. technology (including without limitation, Large Language Models)."
- No express robots/scraper clause was found in the VHL terms.

**CFAA after *Van Buren* and *hiQ*.** *Van Buren v. United States*, 593 U.S. ___ (2021): "An
individual 'exceeds authorized access' when he accesses a computer with authorization but then
obtains information located in particular areas of the computer—such as files, folders, or
databases—that are off-limits to him"; liability "stems from a gates-up-or-down inquiry—one either
can or cannot access a computer system, and one either can or cannot access certain areas within the
system." Footnote 8 leaves the key question open: "we need not address whether this inquiry turns
only on technological (or 'code-based') limitations on access, or instead also looks to limits
contained in contracts or policies." *hiQ Labs v. LinkedIn* (9th Cir. 2022-04-18, on remand from
*Van Buren*): scraping *publicly available* pages is not "without authorization"; "violating a
public website's user agreement alone is insufficient to trigger CFAA liability" — but the court
was explicit that password-protected data is a different case, and in November 2022 the district
court held LinkedIn's anti-scraping and fake-account clauses "enforceable under a breach of contract
claim," so hiQ lost on contract even though it won on the CFAA. The Ninth Circuit's
password-sharing cases fill in the rest: *United States v. Nosal* (Nosal II, 2016) and *Facebook v.
Power Ventures* (2016) hold that using credentials with the *account holder's* permission is not a
CFAA violation by itself, but once the *system owner* revokes authorization (a cease-and-desist
letter, an IP block), continued access through any means is "without authorization" — and the
account holder's consent cannot restore it.

**On-device automation the student runs vs our servers logging in.**

| | On-device (today) | Server-side with stored credentials |
|---|---|---|
| Who is the accessor | The student, on their own machine, with their own credentials | Knowlu |
| ToS breach | The student's (automated access; AI-processing clauses) — exposure is account termination; Knowlu's exposure is at most tortious-interference theories, which are weak when the tool is user-directed | Knowlu breaches directly: "allowing another person to use ... your account name and password" is the plain case; VHL's "share or otherwise transfer ... Login Credentials" is triggered by the storage itself |
| CFAA | Gates are up for the student; *Van Buren* + *hiQ* make Knowlu's exposure remote | Low until the vendor objects; after one cease-and-desist letter, *Power Ventures* makes every further login "without authorization" — federal criminal exposure for a student founder |
| Credential custody | None; Windows Credential Manager, never in the repo, never in a prompt (VISION standing rule) | Knowlu holds thousands of email+password pairs = SPII under Ala. Code § 8-38-2, with the § 8-38-3 duty of care and a 45-day breach clock; students reuse passwords, so a breach leaks university and bank logins too |
| Vendor countermeasures | Per-user; a block hurts one student and is visible to them | One IP range, one User-Agent, one CAS flow: trivially fingerprinted and blocked for everyone at once; recall zyBooks already 403s on a missing User-Agent |
| Zero-retention / data minimization story | Intact | Broken: "we hold your Spanish-homework password in our database" is the sentence that ends a privacy-skeptic's trial |

**Recommendation.** Keep portal scraping on the student's device, exactly as built: credentials in
Windows Credential Manager, the deterministic parser (no model touches portal content, which also
keeps Knowlu clear of both vendors' AI-ingestion clauses), rate-limited, read-only, session-scoped,
and failing visibly. Do **not** move it server-side. Tell the student, once, in plain words on the
connection screen, that the vendor's terms prohibit automated access and that Knowlu acts only on
their instruction and only as they could themselves. Design the scraper to stop on the first sign of
a vendor block and never to retry through a change of identity (that is the *Power Ventures* line).
In parallel, the honest long-term route is the one in the market document's §6: an official
integration or vendor permission when there are users to justify the ask. If a lawyer later wants
more comfort on the on-device path, the question to put is whether user-directed local automation
can be characterized as "the student's own access" under footnote 8 of *Van Buren* in the Eleventh
Circuit (Alabama) and Sixth Circuit (Kentucky), neither of which has adopted *hiQ*.

**Avoid:** headless-browser farms, shared IPs, residential-proxy tricks, storing session cookies
server-side, anything that touches portal content with a model, and any marketing that names
zyBooks or VHL as "integrations" (they are not; call them "your own logins, on your own PC").

**Risk rank:** BLOCKING if server-side; LATER if on-device.

---

## 7. Analytics and consent

**The US baseline is notice, not consent — with two exceptions.** There is no federal statute
requiring opt-in for first-party analytics. The governing rule is FTC Act § 5 (15 U.S.C. § 45):
*deception* (your privacy policy and UI must be true) and *unfairness* (no substantial,
unavoidable, un-offset injury). The 2024–2026 enforcement record maps the edges:

- **Avast (FTC, 2024-02-22):** collected browsing data through privacy-branded software and sold it
  through a subsidiary; the order bans sale of browsing data for advertising, requires "affirmative
  express consent from consumers before selling or licensing browsing data from non-Avast products,"
  and requires Avast to "delete the web browsing information transferred to Jumpshot and any
  products or algorithms Jumpshot derived from that data" — algorithmic disgorgement. $16.5M.
- **X-Mode/Outlogic (2024-01-09) and InMarket (2024-01-18):** first bans on selling sensitive location
  data; both orders require programs to ensure "Affirmative Express Consent" was obtained upstream
  for the collection and use of the data. The lesson generalizes: for sensitive categories the FTC
  expects consent that is affirmative, specific and informed, not a privacy-policy sentence.
- **OkCupid / Match Group (proposed order 2026-03-30):** OkCupid gave "photos, demographic
  information, and location information" about millions of users to the AI company Clarifai,
  contrary to a privacy policy that named only "service providers, business partners, and businesses
  within its 'family of businesses.'" No fine (first-offence § 5 rule), a 20-year order barring
  misrepresentation of information practices and privacy controls. The FTC's framing: "say what you
  do and do what you say." This is the first federal action treating undisclosed AI-training data
  sharing as a consumer-protection violation in itself.
- **COPPA amendments (compliance 2026-04-22):** separate parental consent for third-party
  disclosures, including for AI training. Not applicable to us; a marker of where consent norms are
  moving.

**Is a student's schedule or grades "sensitive" under state law?** Not under any comprehensive-law
definition found (the standard list: racial/ethnic origin, religious beliefs, mental or physical
health diagnosis, sexual orientation, citizenship or immigration status, genetic or biometric data,
precise geolocation, personal data of a known child under 13; Kentucky's KCDPA uses exactly this
list). Two caveats. First, **the Gmail inbox will contain sensitive data incidentally** — campus
health, disability services, campus ministries, the international student office — so under the
Virginia-model laws, once one applies, processing it needs consent. Second, California treats the
*contents of email* as sensitive personal information when the business is not the intended
recipient (§ 1798.140 — verify the paragraph letter). Neither law applies at pilot scale; both say
what "correct" looks like.

**For (a) product events and (b) corrections to AI judgments, collected automatically under the
ToS:** a privacy policy that says specifically what is collected, why, how long it is kept, that it
is keyed to a pseudonymous account id (do not say "anonymous"), and that it is not sold — plus ToS
acceptance at account creation — is sufficient in the US today. Three conditions keep it that way:
(i) the disclosure is in the policy *and* in a one-screen in-app notice at first run, not only in a
5,000-word document; (ii) (b) logs the correction (old label, new label, item id, source type, the
engine's non-content fields), **never** the content of the item — for Gmail-derived items that is a
Limited Use requirement (§5), and for everything else it is the difference between telemetry and
surveillance; (iii) there is a switch to turn (a) off (not legally required, but the VISION text
already promised opt-in telemetry and a promise in your own docs is the kind of thing § 5 holds you
to if it ever reaches marketing copy). If the switch stays, the ToS can make (b) a condition of
service — it is how the product learns — as long as that is said plainly.

**For (c) opt-in raw content for model improvement, an effective opt-in is:**

- **Separate and unbundled:** its own screen, never pre-checked, not a condition of the subscription
  or of any feature; declining changes nothing.
- **Specific:** names the content (e.g. "the text of tasks you create, syllabus PDFs you upload,
  assignment titles from your LMS feed"), the purpose ("to improve how Knowlu classifies coursework
  for all users"), the retention period, and who processes it (the inference provider, under
  zero-retention terms — say whether training happens on your infrastructure or theirs).
- **Scoped by source:** **excludes Gmail-derived content entirely** (Limited Use, §5), and prudently
  excludes anything pulled from zyBooks or VHL (their AI-ingestion clauses).
- **Revocable with effect:** one click in settings; future collection stops; collected raw content is
  deleted within a stated window; and it says honestly that models already trained are not untrained
  (the Avast order shows the FTC can demand exactly that when consent was defective, which is the
  reason to get the consent right rather than to promise the impossible).
- **Logged:** timestamp, policy version, the exact text shown. Keep the log as long as the model
  trained on the data exists.
- **No dark patterns:** the decline button is as prominent as accept; no re-prompting on a schedule.

**Risk rank:** (a)(b) BEFORE-WIDENING; (c) BEFORE-WIDENING if offered at all — and it is worth asking
whether (c) is needed at all in year one, given that the Gmail exclusion removes the richest source
and the rule-promotion loop is per-user by design.

---

## 8. Subscriptions

**Federal floor — ROSCA, 15 U.S.C. § 8403.** An online negative-option seller must (1) provide "text
that clearly and conspicuously discloses all material terms of the transaction before obtaining the
consumer's billing information," (2) obtain "a consumer's express informed consent before charging
the consumer's credit card, debit card, bank account, or other financial account," and (3) provide
"simple mechanisms for a consumer to stop recurring charges." This applies to Knowlu from the first
charge, everywhere.

**The FTC "click-to-cancel" (Negative Option) Rule — status on 2026-09-09.** Vacated in its
entirety by the Eighth Circuit on 2025-07-08 (procedural: no preliminary regulatory analysis) days
before its compliance date. The FTC sent a draft Advance Notice of Proposed Rulemaking to OIRA on
2026-01-30, published the ANPRM in March 2026 with comments due 2026-04-13 (about 100 received), and
has signalled a rule that may be "broader or more prescriptive than the 2024 version." **No federal
rule is in effect; there is no proposed rule text yet; enforcement continues under ROSCA and § 5.**
The 2024 rule's three obligations (clear disclosure, express informed consent, cancellation "no more
difficult" than sign-up) remain the safe design target because California now requires all of them
by statute.

**California ARL as amended by AB 2863 (effective 2025-07-01; Bus. & Prof. Code § 17600 et seq.).**
Applies to contracts entered into, amended or extended on or after that date with California
consumers. Requires: "express affirmative consent" to the auto-renewal terms themselves (a separate
checkbox, not buried in ToS acceptance); retention of consent records for **three years or one year
after termination, whichever is longer**; an **annual reminder for all subscriptions** stating the
product, the frequency and amount of charges, and how to cancel, sent through the same medium as the
transaction; cancellation for anyone who enrolled online "exclusively online, at will, and without
engaging any further steps," through "a prominently located direct link or button" in the account or
a pre-formatted cancellation email — retention offers are allowed only with a "prominent and
proximately displayed 'click to cancel' button" alongside; price-change notice 7–30 days before the
increase with cancellation information; free-trial reminders 3–21 days before the end of any
promotional period longer than 31 days. Enforced by the AG and district attorneys and, through the
UCL, private plaintiffs.

**New York GBL §§ 527, 527-a as amended (effective 2025-11-05, the "click to cancel act").**
Affirmative consent before charging with clear and conspicuous terms near the consent; cancellation
through the consumer's chosen contact method plus toll-free number, email address and a website
link; material-change notice **5–30 days** before the change; on a price increase, either prior
affirmative consent **or** a 14-day post-charge cancellation window with a pro-rata refund; renewal
reminders 15–45 days before renewal for initial terms of a year or more (and renewals of six months
or more). **Colorado SB25-145 (effective 2026-02-16):** one-step online cancellation link for online
enrollees; save offers allowed only with the direct link "prominently located."

**What Stripe Billing plus an in-app cancel button must satisfy — the checklist.**

1. *Checkout:* Stripe Checkout's terms-of-service acceptance checkbox, with the auto-renewal terms
   (price, cadence, that it renews until cancelled, how to cancel, the summer pause mechanics) shown
   next to it — that is the ROSCA/CA/NY consent. Store your own record: user id, timestamp, price,
   terms version, IP. Three years minimum.
2. *Cancel:* the Stripe Customer Portal with cancellation enabled, reachable from a button labelled
   "Cancel subscription" in the app's account screen (one click to the portal, one click to cancel —
   no survey gate), **and** Stripe's "Manage subscription" link enabled in billing emails (Stripe's
   own FTC/California FAQ describes this as the intended compliance path). Offer "pause" beside
   cancel, never instead of it.
3. *Reminders:* Stripe's upcoming-renewal email for the academic-year plan, plus an annual reminder
   for monthly subscribers (California) — a cron and a template. Under a summer pause, send a
   reminder before the first post-pause charge; a resumed charge after months of $0 looks like a
   free-to-pay conversion to a regulator and to a student.
4. *Price changes:* email 7–30 days ahead (CA) / 5–30 (NY) with a cancel link; for New York either
   collect fresh consent or honour a 14-day pro-rata refund on the first increased charge.
   Grandfathering avoids the whole question during the pilot.
5. *Refunds:* a stated policy; a 17-year-old who disaffirms gets one (§2).
6. *Copy:* the price page says "$9.99/month, renews monthly until you cancel; cancel any time in
   Settings" — that sentence is most of the law.

**Sales tax, one paragraph.** *Kentucky* taxes SaaS: since 2023-01-01 (HB 8), "prewritten computer
software access services" — "the right of access to prewritten computer software where the object
of the transaction is to use the prewritten computer software while possession ... is maintained by
the seller or a third party ... regardless of whether the charge ... is on a per use, per user, per
license, subscription, or some other basis" (KRS 139.010(34), taxed under KRS 139.200) — at 6%;
remote-seller nexus is $100,000 of sales or 200 transactions into Kentucky (KRS 139.340 — cited from
memory; verify), and 200 monthly charges to UK students arrives in the first month, so register
before the first Kentucky sale. *Alabama* treats software itself as tangible personal property
("Computer software is tangible personal property ... subject to Sales Tax, Use Tax, or Rental Tax
... regardless of its function or form of transmission," Ala. Admin. Code r. 810-6-1-.37(4), amended
effective 2020-01-13 after *Ex parte Russell County Community Hospital*, Ala. S. Ct. 2019-05-17),
but no rule or ruling addresses remotely accessed software with no download, and every tax-compliance
tracker read (Anrok, Numeral) reports the Department's position as *not taxable* for true SaaS.
Because Knowlu is an Alabama business selling to Alabama residents there is no nexus question — only
the taxability one — so get a written answer from a CPA (or a Department ruling) before launch
rather than relying on trackers. Stripe Tax can handle registration-by-state once the answer is
known.

**Risk rank:** BEFORE FIRST PAID SIGNUP (and before the first Kentucky sale for tax).

---

## 9. Issue reports with diagnostic context

No statute specifically governs a "report an issue" button; the constraints are the general ones.
(i) Notice: the privacy policy must list diagnostic logs as a category collected and say what they
may contain. (ii) Proportionality: California's standard — collection "shall be reasonably
necessary and proportionate to achieve the purposes for which the personal information was collected
or processed" (Cal. Civ. Code § 1798.100(c)) — is the right design rule even before it binds.
(iii) Truthfulness: do not call the report "anonymous" if it carries an account id. (iv) Google
Limited Use: Gmail content may reach a human only with the user's "explicit consent or affirmative
agreement to view specific messages" — so a report must never auto-attach email text; if the user
wants to include a message, they attach it themselves and the UI says a person will read it.
(v) Third parties' data: an inbox-triage log may name other people; keep reports scoped to Knowlu's
own fields. **Cheapest path:** a preview screen showing the exact payload, with tokens, URLs
containing secrets (the ICS capability URL), email addresses and note bodies redacted by default and
an editable text box; keep reports 90 days; store them with the same access controls as the vault.
The existing rule that judgment logs "hold ids and field values only, and never enter the vault"
(CLAUDE.md) is already the right shape. **Risk rank:** LATER.

---

## 10. University policies — Alabama and Kentucky

**University of Alabama.**

- *Information Protection Procedure* (OIT; effective 2020-06-01, revised 2023-08-01): "Passwords
  should never be shared." IT resource users, students included, "are responsible for protecting the
  security of all data and IT resources to which they have access"; "In the cases where students are
  involved, such issues will result in the reporting of a Student Code of Conduct violation." Its
  third-party clause ("A contractual agreement ... shall be in place and approved through the
  Procurement Services process before exchanging information with the third party / service
  provider") governs University *units* sharing University information, not a student's personal
  use of their own data.
- *HPC Acceptable Use Policy:* "Sharing of account login information is not allowed."
- *Terms of Use of University Technology Resources* exists on UA's PolicyStat (policy 14865470) and
  is the general acceptable-use document, but the site is JavaScript-rendered and returned no text to
  the fetcher on 2026-09-09 — **read it directly before the pilot; it is the document a UA
  administrator would cite.**
- *Blackboard third-party integrations:* the Center for Instructional Technology's overview (Confluence,
  last updated 2025-08-26) states that CIT "supports the integration between third-party tools and
  the learning management system" but "does not directly support third-party tools," reviews
  integrations periodically, and "reserves the right to disable or remove any integration that
  becomes a vulnerability to the system." The full page was truncated by the fetcher; those quotes
  come from the search index. The process is instructor/department-driven LTI review; there is no
  published stance on students connecting personal apps, and nothing on the student ICS feed, which
  is a Blackboard-native feature Anthology documents for students.
- *Registrar FERPA page:* institution-facing; describes directory information and the myBama
  proxy/FERPA-release mechanism; silent on third-party apps.
- *Crimson mail is Google Workspace for Education.* A Workspace administrator can set third-party
  OAuth apps to "Trusted," "Limited," "Specific Google data," or "Blocked," and Education tenants can
  "choose different settings for users who are over and under 18 years old." The default is
  permissive ("Allow users to access any third-party apps"), and UA's current setting could not be
  observed. **This is the switch that would end Gmail ingest for `@crimson.ua.edu` overnight** —
  build for it: the personal Gmail account is the fallback, and the failure must be visible.

**University of Kentucky.**

- *Administrative Regulation 10:1, Use of Technology Resources* (effective 2018-08-01; contact
  information updated 2024-01-22; parts moved to AR 10:7 *Security of Data* and AR 10:8 *Security of
  Information Technology Resources*). Applies to "all users of technology resources at or through the
  University." Each user must "Secure and maintain all computer accounts, passwords, and other types
  of authorization in confidence" (IV.J.a.ii). Users must not "Obtain or use another's login
  credentials or otherwise access technology resources to which authorization has not been expressly
  given" (IV.J.c.i), must not "attempt to circumvent any established security measures, for any
  reason, (e.g. using a computer program to attempt password decoding)" (IV.J.c.vi), must not
  "Knowingly access, add, or modify any data without proper authorization" (IV.J.c.x), and must not
  "Utilize University technology resources to promote, solicit, support or engage in any commercial
  activities on behalf of ... any person or entity other than the University without prior
  authorization" (IV.J.c.xi). Violations bring "terminating access ... disciplinary action, civil
  liability, and criminal sanctions" (IV.K.d).
- *UK ITS on linkblue:* "LinkBlue passwords are used for many different sites, and it is important
  that they not be shared with others."
- *Canvas:* UK Online documents SLAM ("Self-Service LTI App Management") for instructors; **no
  published UK policy on student API tokens or third-party apps was found**, on uky.edu or
  elsewhere, on 2026-09-09. Absence of a page is not permission; assume the sector norm (§1).

**What follows for Knowlu.** Both universities forbid credential sharing and unauthorized access,
which Knowlu's architecture already respects: it never asks for myBama or linkblue credentials, and
its two university-facing connectors are a student-generated ICS URL and a student-granted, read-only
OAuth scope. The one clause that could be turned against a student user is UK's IV.J.c.xi
("commercial activities ... on behalf of ... any person or entity other than the University" using
University resources) — a stretch for a student running a paid personal app, but a reason to keep
the desktop app off University-owned machines in the pilot. Neither university has a published
stance on Blackboard/Canvas third-party integrations that reaches student-side tools; both reserve
the right to disable integrations and, in UA's case, third-party OAuth access to Workspace. **Risk
rank:** LATER; the design implication (survive a Workspace block) is a now item.

---

## 11. The three findings most likely to change the product's shape

1. **Gmail data cannot feed a shared model, and Gmail beyond 100 users is gated by Google, not by
   us.** The Workspace policy's prohibition on "using user data to create, train, or improve a machine
   learning or artificial intelligence model beyond that specific user's personalized model" removes
   Gmail — the richest source — from plan (c) regardless of opt-in; and restricted-scope
   verification plus an annual CASA assessment (about six weeks; lab fees in the low thousands; a
   100-user ceiling until the letter arrives) is a fixed cost and a schedule item for the paid Gmail
   feature. Testing mode's 7-day token expiry means there is no informal pilot path.
2. **Server-side portal scraping converts a student's ToS breach into Knowlu's own contract and
   CFAA exposure and makes Knowlu a password custodian under Alabama's breach law.** Both vendors
   forbid sharing credentials with another person; *Power Ventures* makes access after a vendor's
   objection "without authorization"; and § 8-38-2 counts every stored email+password pair as SPII
   with a 45-day breach clock. On-device scraping with a deterministic parser keeps every one of
   those risks on the far side of the line. The redirect to "all judgment in the cloud" should stop
   at the portals.
3. **The minors question is smaller than feared and has a one-checkbox answer.** Alabama's 2019
   amendment (§ 26-1-1(f)) binds 18-year-olds; Kentucky's majority is 18; COPPA stops at 13; and every
   state minors' online law examined is defined by social features Knowlu does not have. Gate paid
   accounts at 18 with an attestation, accept the refund exposure or add a parent-pays flow for
   17-year-olds, and collect no birthdate.

---

## 12. Questions for the lawyer

1. Confirm that a student-authorized third-party app receiving the student's own data is outside
   FERPA entirely, and that no state law (Alabama, Kentucky) creates a student-record duty for such an
   app. Is there any value in saying so in the ToS?
2. Age gate: is an 18+ attestation sufficient in Alabama under § 26-1-1(f), or does the lawyer want a
   parent-pays flow for 17-year-olds from day one? Confirm Nebraska (19) and Mississippi (21) before
   those campuses.
3. Does the Google Workspace Limited Use clause on AI models permit *per-user* rule promotion from
   Gmail-derived corrections (our reading: yes) and forbid any pooled training even with opt-in (our
   reading: yes)? Should plan (c) exist in year one at all?
4. Portal scraping on-device: in the Eleventh and Sixth Circuits, is user-directed local automation
   with the user's own credentials defensible as the user's own access under *Van Buren* footnote 8,
   and what disclosure to the student best allocates the ToS risk? Draft the connection-screen text.
5. Privacy policy: one policy to CalOPPA plus the Google Limited Use sentence, with universal rights —
   review it. Confirm no state currently requires opt-in for first-party product analytics keyed to a
   pseudonymous id.
6. Is Gmail triage "processing sensitive data" under the Virginia-model laws given incidental content
   (health, immigration, religion)? Draft the Gmail connection consent so it works as that consent
   when a law later applies, and as CCPA sensitive-PI notice (email contents).
7. Subscriptions: review the Stripe Checkout consent text, the cancel flow, the annual-reminder
   template, and the summer-pause mechanics against California AB 2863 and New York 527-a. Does a
   pause-then-resume charge need a fresh consent or only a reminder?
8. Alabama § 8-38: what does a "reasonable security measures" program look like at our size, and
   which vendor contracts (Supabase, Cloudflare, Stripe, the inference provider) need the 10-day
   third-party-agent notice term? Confirm the penalty figures in § 8-38-9.
9. Sales tax: obtain a written position on Alabama taxability of no-download SaaS; confirm Kentucky
   registration timing and the 200-transaction trigger.
10. Alabama APDPA (2027-05-01): confirm whether the small-business exemption reported in summaries is
    in the enrolled HB 351 and how "consumers" are counted (accounts vs. individuals).
11. Terms: arbitration/limitation clauses vs. minors; a clause reserving the right to disable a
    connector when a university or vendor objects; the "not affiliated with the University of
    Alabama" disclaimer (trademark/endorsement, not covered in this note).
12. Read UA's *Terms of Use of University Technology Resources* (PolicyStat 14865470) and advise
    whether anything in it reaches a student running a paid third-party app against their own data.

---

## 13. Sources (all accessed 2026-09-09)

**FERPA**
- 20 U.S.C. § 1232g (LII): https://www.law.cornell.edu/uscode/text/20/1232g
- 34 CFR § 99.3 (LII): https://www.law.cornell.edu/cfr/text/34/99.3
- 34 CFR § 99.31 (LII): https://www.law.cornell.edu/cfr/text/34/99.31
- 34 CFR § 99.1 (eCFR; blocked to the fetcher, text via nces.ed.gov mirror):
  https://www.ecfr.gov/current/title-34/subtitle-A/part-99/subpart-A/section-99.1
- ED PTAC FAQ-5, *Responsibilities of Third-Party Service Providers under FERPA* (Aug 2015):
  https://studentprivacy.ed.gov/sites/default/files/resource_document/file/Vendor%20FAQ.pdf
- Cornell, Canvas API access tokens: https://learn.canvas.cornell.edu/canvas-api-access-tokens
- Penn, Canvas site policies: https://infocanvas.upenn.edu/policies/canvas-site-policies/
- UW-Madison KB 156712: https://kb.wisc.edu/luwmad/156712
- Penn State (2026-05-20): https://www.it.psu.edu/news/canvas/canvas-personal-access-tokens/
- Texas A&M (404 at both URLs on 2026-09-09; search-index text): https://lms.tamu.edu/support/canvas-api
- UA Registrar FERPA: https://registrar.ua.edu/academics-policies/ferpa/

**Minors**
- Ala. Code § 26-1-1 (FindLaw): https://codes.findlaw.com/al/title-26-infants-and-incompetents/al-code-sect-26-1-1.html
- KRS 2.015 (Justia): https://law.justia.com/codes/kentucky/chapter-2/section-2-015
- FTC COPPA FAQ: https://www.ftc.gov/business-guidance/resources/complying-coppa-frequently-asked-questions
- Amended COPPA Rule (Fed. Reg. 2025-04-22): https://www.federalregister.gov/documents/2025/04/22/2025-05904/childrens-online-privacy-protection-rule
- *NetChoice v. Bonta*, No. 25-2366 (9th Cir. 2026-03-12): https://law.justia.com/cases/federal/appellate-courts/ca9/25-2366/25-2366-2026-03-12.html ; Cooley summary: https://www.cooley.com/news/insight/2026/2026-03-30-netchoice-v-bonta-ninth-circuit-narrows-injunction-against-californias-ageappropriate-design-code-act
- Tex. Bus. & Com. Code § 509.002 (FindLaw): https://codes.findlaw.com/tx/business-and-commerce-code/bus-com-sect-509-002/
- Utah Code § 13-71-101 (FindLaw): https://codes.findlaw.com/ut/title-13-commerce-and-trade/ut-code-sect-13-71-101/
- Utah/Florida/Arkansas status (JURIST, Courthouse News, AVPA): https://www.jurist.org/features/2025/05/05/teen-social-media-law-the-ebbs-and-flows-in-2025/ ; https://www.courthousenews.com/utah-urges-10th-circuit-to-reinstate-social-media-law-for-minors/ ; https://avpassociation.com/us-state-age-assurance-laws-for-social-media/
- Nebraska AADC (Hunton; Wilson Sonsini): https://www.hunton.com/privacy-and-cybersecurity-law-blog/nebraska-enacts-new-laws-protecting-children-online ; https://www.wsgr.com/en/insights/nebraska-and-vermont-pass-age-appropriate-design-codes.html
- Texas SB 2420 status (Privacy World; Wikipedia summary of CCIA v. Paxton): https://www.privacyworld.blog/2025/12/federal-judge-enjoins-enforcement-of-texas-app-store-age-verification-law/ ; https://en.wikipedia.org/wiki/Computer_%26_Communications_Industry_Association_v._Paxton
- Alabama HB 161 (Hunton; Alabama Reflector): https://www.hunton.com/privacy-and-cybersecurity-law-blog/alabama-enacts-app-store-accountability-act-requiring-age-verification ; https://alabamareflector.com/2026/02/06/alabama-senate-passes-bill-requiring-app-stores-to-verify-ages-of-users/
- Quizlet ToS (403 to fetcher; search-index text): https://quizlet.com/tos ; Chegg ToU (403; search-index text): https://www.chegg.com/en-US/termsofuse ; Notion Personal Use ToS (JS-rendered; search-index text): https://www.notion.so/Personal-Use-Terms-of-Service-00e4e5d0f2b9411cbee6493f15779500

**State privacy, CalOPPA, Alabama**
- Tex. Bus. & Com. Code § 541.002: https://texas.public.law/statutes/tex._bus._and_com._code_section_541.002
- Kentucky AG, KCDPA: https://www.ag.ky.gov/about/Office-Divisions/ODP/KCDPA/Pages/default.aspx
- Alabama HB 351 (Hunton; DLA Piper; enrolled PDF): https://www.hunton.com/privacy-and-cybersecurity-law-blog/alabama-becomes-21st-state-with-comprehensive-consumer-privacy-law ; https://privacymatters.dlapiper.com/2026/04/u-s-alabama-becomes-21st-state-to-enact-comprehensive-privacy-law/ ; https://alison.legislature.state.al.us/files/pdf/SearchableInstruments/2026RS/HB351-eng.pdf
- Threshold trackers: https://privacylawmap.com/compare ; https://www.clym.io/blog/us-privacy-law-comparison-map ; https://www.multistate.us/insider/2026/2/4/all-of-the-comprehensive-privacy-laws-that-take-effect-in-2026
- Cal. Bus. & Prof. Code §§ 22575, 22577 (FindLaw): https://codes.findlaw.com/ca/business-and-professions-code/bpc-sect-22575/ ; https://codes.findlaw.com/ca/business-and-professions-code/bpc-sect-22577/
- Cal. Civ. Code § 1798.100: https://leginfo.legislature.ca.gov/faces/codes_displaySection.xhtml?lawCode=CIV&sectionNum=1798.100
- Ala. Code § 8-38 (Justia index, 403 to fetcher; DWT summary used): https://law.justia.com/codes/alabama/title-8/chapter-38/ ; https://www.dwt.com/gcp/states/alabama

**Student-data laws**
- Cal. Bus. & Prof. Code § 22584 (FindLaw): https://codes.findlaw.com/ca/business-and-professions-code/bpc-sect-22584/
- Conn. Gen. Stat. § 10-234aa (FindLaw): https://codes.findlaw.com/ct/title-10-education-and-culture/ct-gen-st-sect-10-234aa/
- KRS 365.734: https://apps.legislature.ky.gov/law/statutes/statute.aspx?id=43327
- Alabama State Board of Education Data Use and Governance Policy: https://www.alabamaachieves.org/wp-content/uploads/2021/01/Data-Use-Governance-Policy.pdf

**Google**
- Gmail API scopes: https://developers.google.com/workspace/gmail/api/auth/scopes
- Restricted scope verification: https://developers.google.com/identity/protocols/oauth2/production-readiness/restricted-scope-verification
- Google API Services User Data Policy: https://developers.google.com/terms/api-services-user-data-policy
- Google Workspace API user data and developer policy (2026-07-22): https://developers.google.com/workspace/workspace-api-user-data-developer-policy
- OAuth verification FAQ: https://support.google.com/cloud/answer/13463817
- Verification requirements: https://support.google.com/cloud/answer/13464321
- Security assessment: https://support.google.com/cloud/answer/13465431
- Manage app audience (testing limits, 100-user cap): https://support.google.com/cloud/answer/15549945
- Unverified apps: https://support.google.com/cloud/answer/7454865
- CASA: https://appdefensealliance.dev/casa ; https://appdefensealliance.dev/casa/casa-tiering
- CASA pricing (vendor blog, indicative only): https://deepstrike.io/blog/google-casa-security-assessment-2025
- Google Cloud Community thread on local-only apps (2026-03-24): https://security.googlecloudcommunity.com/google-security-operations-2/what-happened-to-local-app-gmail-api-access-7138
- Workspace admin app access control: https://knowledge.workspace.google.com/admin/apps/control-which-apps-access-google-workspace-data

**Scraping / CFAA**
- zyBooks Terms of Use (2025-11-07): https://www.zybooks.com/terms-of-use/
- Vista Higher Learning Terms of Use: https://vistahigherlearning.com/terms-of-use (vhlcentral.com/terms_of_use returned 402)
- *Van Buren v. United States*, slip op.: https://www.supremecourt.gov/opinions/20pdf/19-783_k53l.pdf
- *hiQ v. LinkedIn* (9th Cir. 2022) and Nov 2022 district ruling (Morgan Lewis; White & Case): https://www.morganlewis.com/blogs/sourcingatmorganlewis/2022/12/linkedin-v-hiq-landmark-data-scraping-suit-provides-guidance-to-data-scrapers-and-web-operators ; https://www.whitecase.com/insight-our-thinking/web-scraping-website-terms-and-cfaa-hiqs-preliminary-injunction-affirmed-again
- *Nosal II* / *Power Ventures* (Fenwick): https://www.fenwick.com/insights/publications/cfaa-clarity-from-9th-circ-password-sharing-decisions

**Analytics / FTC**
- FTC Act § 5, 15 U.S.C. § 45 (not separately fetched)
- Avast press release (2024-02-22): https://www.ftc.gov/news-events/news/press-releases/2024/02/ftc-order-will-ban-avast-selling-browsing-data-advertising-purposes-require-it-pay-165-million-over
- X-Mode/Outlogic (2024-01-09): https://www.ftc.gov/news-events/news/press-releases/2024/01/ftc-order-prohibits-data-broker-x-mode-social-outlogic-selling-sensitive-location-data ; InMarket (2024-01-18): https://www.ftc.gov/news-events/news/press-releases/2024/01/ftc-order-will-ban-inmarket-selling-precise-consumer-location-data
- OkCupid/Match (Venable summary, 2026-04): https://www.venable.com/insights/publications/2026/04/ftc-okcupid-settlement-deceptive-data-sharing

**Subscriptions / tax**
- ROSCA, 15 U.S.C. § 8403 (LII): https://www.law.cornell.edu/uscode/text/15/8403
- FTC rulemaking status (Jones Day, May 2026; Goodwin, Feb 2026): https://www.jonesday.com/en/insights/2026/05/ftc-revives-clicktocancel-rule-new-risks-for-subscription-businesses ; https://www.goodwinlaw.com/en/insights/publications/2026/02/alerts-practices-ba-ftcs-click-to-cancel-rule-gets-new-life
- Eighth Circuit vacatur (Latham): https://www.lw.com/en/insights/eighth-circuit-vacates-ftc-click-to-cancel-rule-days-before-compliance-deadline
- California AB 2863 (bill text; Cooley summary): https://leginfo.legislature.ca.gov/faces/billTextClient.xhtml?bill_id=202320240AB2863 ; https://www.cooley.com/news/insight/2025/2025-06-04-california-automatic-renewal-law-amendments-take-effect-on-july-1-2025
- New York GBL § 527-a and Colorado SB25-145 (Perkins Coie): https://www.ashurstperkinscoie.com/insights/update/new-york-and-colorado-update-auto-renewing-subscription-requirements
- Stripe FAQ on FTC/California changes: https://support.stripe.com/questions/faq-ftc-california-subscription-law-changes-require-billing-updates
- Ala. Admin. Code r. 810-6-1-.37: https://admincode.legislature.state.al.us/api/rule/810-6-1-.37 ; ADOR 2019 guidance: https://www.revenue.alabama.gov/ador-issues-guidance-on-taxability-of-computer-software/ ; Anrok Alabama page: https://www.anrok.com/saas-sales-tax-by-state/alabama
- KRS 139.010 (FindLaw) and Kentucky DOR Sales Tax Facts (June 2023): https://codes.findlaw.com/ky/title-xi-revenue-and-taxation/ky-rev-st-sect-139-010/ ; https://revenue.ky.gov/News/Publications/Sales%20Tax%20Newsletters/Sales%20Tax%20Facts%202023%20-%20June.pdf ; TaxJar: https://www.taxjar.com/blog/2023-03-is-saas-taxable-in-kentucky

**University policies**
- UA OIT policies index: https://oit.ua.edu/about/policies-and-guidance/
- UA Information Protection Procedure: https://oit.ua.edu/about/policies-and-guidance/information-protection-procedure/
- UA Terms of Use of University Technology Resources (PolicyStat; no text returned to fetcher): https://ua-public.policystat.com/policy/14865470/latest/
- UA HPC Acceptable Use Policy: https://hpc.ua.edu/current-services/acceptable-use-policy/
- UA CIT Blackboard third-party integrations overview (fetch truncated): https://bama.atlassian.net/wiki/spaces/CIT/pages/1709539458/Blackboard+Third-Party+Tools+Integrations+Overview
- UK AR 10:1 (PDF): https://regs.uky.edu/sites/default/files/2024-01/AR_10.1_2018_2024-Update-contact-info_FINAL.pdf ; index page: https://regs.uky.edu/administrative-regulation/ar-101
- UK ITS on linkblue passwords: https://its.uky.edu/news/uk-its-introduces-non-expiring-passwords-linkblue-accounts
- UK Online, Canvas (SLAM): https://online.uky.edu/faculty-resources/canvas
