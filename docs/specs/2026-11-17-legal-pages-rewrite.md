# Legal pages rewrite: `site/privacy.html` and `site/terms.html`

Planner, 2026-11-17. **Size: Contract** (the privacy page, money, and `app/src/account.rs` on the contract list).
Status: revision 3, the final text pass, answering review r2 (`docs/reports/2026-11-17-legal-pages-review-r2.md`)
on top of r1. There is no further review round. Not legal advice; section 8 is what a lawyer checks.

### What changed in the final pass (r3)

Each of review r2's decisions is written in as a provisional default. Quinn's further answers of
2026-11-17 (U8–U12 in §0) settle N3, N4 and five of the old questions. What is still open is §9: Q1
(the accounts into the LLC) and Q2 (confirming the N1, N2 and N5 defaults).
- **U8–U12:** the forum clause is exclusive to Jefferson County, Kentucky, small claims included (terms
  *Disputes*, lawyer item 16). [MAILING ADDRESS] stays a placeholder until publication. The interim
  terms are approved, subject to the account count. The unintended-charge definition and email-matching
  titles are approved. Every price sentence says "Sales tax is added at checkout where it applies."
- **N1:** bump A merges to `main` after the v0.4.0 tag and ships on the next tag, which leaves the
  v0.4.0 plan, its whole-branch review and its pre-tag gate untouched (§5).
- **N2:** C17 moves into bump A's PR and C29 into bump B, so no v0.4.0 task changes (§2, §6, §7).
- **N3:** the interim terms ship only if a read-only production account count, run by the main session
  on Quinn's word, shows that nobody but Quinn has an account. Otherwise they are skipped. The step that
  records Quinn's re-acceptance by hand is spelled out as a production write, and the C1 guard is
  reworded (§4 interim, §5). Quinn approved the interim terms under this gate (U10).
- **N4:** case (c) is out of the interim terms, and the cases are written out in their actual words.
- **N5:** the waitlist's privacy bump rides a release tag. C25 becomes an amendment the waitlist lane
  makes to its own plan (C25b); this note does not edit that plan (§5, §6).
- **n1:** the batch entry names the mail service's ids.
- **n2:** D8 no longer contradicts C18.
- **n3:** the interim terms carry the summer-pause paragraph for monthly plans.
- **n4:** C14 names who deletes waitlist addresses.
- §9 now has two questions (Q1, Q2) and the lawyer's background on the cap. Question numbers in the r2
  block below are r2's own.

### What changed in r2

- **K1:** bump A is now its own Contract PR. It merges after v0.4.0's T4, T20, T21 and T23, and every
  cite points at the files v0.4.0 moves things into (§5, §6, §7). C17 folds into T21.
- **K2:** Pell is dropped by Quinn's ruling of 2026-11-17 (U6, D14). There is no Pell text on either
  page. C26 records the ruling as an amendment.
- **I1:** the rulings are quoted in the new §0.
- **I2:** the billing lane is v0.5.0, so the old bumps B and C are now one bump B (§5).
- **I3:** the waitlist lane is ordered in §5. A new rule (C25) lets the legal pages deploy only on a
  release tag. The waitlist's purpose is added to *Mail you send us*.
- **I4:** the backup-restore promise is removed. A deletion log outside the database (C27) would bring
  it back later.
- **I5:** the warranty and cap clauses except the terms' own promises and refunds, and their sentences
  are now bold.
- **I6, I7:** these are open questions with recommendations (§9 Q3, Q6). The pages say KNOWLU LLC
  "operates" Knowlu.
- **I8:** §7 adds the Gmail/Amazon panel test and fixes test 9.
- **I9, m10:** the sign-in wording now matches the code.
- **I10:** the third kept sign-in has its own bump.
- **I11:** each bump is one squash-merged PR.
- **I12:** the private entity references are removed (U7).
- **I13:** the lawyer reads before the first pilot student or paid sale, whichever comes first.
- **I14:** the cancel path is written against the new Membership row (C29).
- **m1–m11:** fixed in place. The appendix gains the rows they named.

**Inputs.** The audit, `docs/reports/2026-11-17-legal-pages-audit.md` ("the audit"); the current pages
(`site/privacy.html` version 2026-10-20, `site/terms.html` version 2026-09-10); Quinn's rulings and
updates, quoted in §0; the cloud design's Amendment 2026-11-10, signed 2026-11-11 ("the amendment"), as
U6 changes it; the v0.4.0 design and plan (`docs/specs/2026-11-17-v0.4.0-new-ui-design.md`,
`docs/plans/2026-11-17-v0.4.0-new-ui-plan.md`); the waitlist design
(`waitlist-spec:docs/specs/2026-11-09-waitlist-design.md`); `VISION.md`; `CLAUDE.md`.

**Limits of what I checked.**
- I had no shell and no web tools. The amendment text is the `waitlist-spec` worktree's copy
  (`docs/specs/2026-09-09-knowlu-cloud-design.md:993-1189`), which review r1 found identical to
  `origin/docs/waitlist-spec` at `5dac15c7`. **It is not on `origin/main` yet**, and `VISION.md:153-156`
  still says $9.99 a month (m9; C26).
- **I did not verify anything on Stripe's site.** Every statement about what Link does under Managed
  Payments comes from the audit's citations (docs.stripe.com, "how it works" and "update checkout") and
  is marked `unverified (Stripe)`. C5 (section 6) turns them into facts with a test-mode Managed
  Payments checkout before the terms publish.
- Law I state that the audit does not cite is marked `unverified (law)`. That covers the notice windows
  for a price change and a yearly renewal, and the Kentucky items. Section 8 sends each one to the lawyer.
- Code facts are cited `path:line` relative to this worktree, and I opened each one.

**How the page text is marked.** Every paragraph, list item and definition in sections 3 and 4 carries
one marker:
- **[TRUE NOW]**: true of `main` today and publishable now. `· unchanged` means the words are the
  current page's.
- **[NEEDS CODE: Cn]**: true only once code change Cn in section 6 lands. It publishes with that change.
- **[v0.5.0]**: ships with v0.5.0, in bump B.
- `Q1` and `Q2` flag text that waits on an answer in section 9. (U1 and U2 settled both of R9's and
  R10's `[QUINN TO CONFIRM]` marks, and U8–U12 settled the rest, so no `[QUINN TO CONFIRM]` remains.)
- `[MAILING ADDRESS]`, `[PRIVACY DATE A/B]`, `[TERMS DATE]`, `[BILLING SWITCH DATE]`,
  `[BACKUP WINDOW]` and `[AUTH LOG WINDOW]` are placeholders. Each must be filled before its paragraph
  publishes, and C21 fills the last two.

**How the markdown maps to HTML.** A `#`/`##` heading is `<h1>`/`<h2>`; a paragraph is `<p>`; a `-`
item is `<li>` in a `<ul>`. A definition-list entry is written as a bold line `**dt:** Term`
followed by the `<dd>` paragraph. `**bold**` is `<strong>`, `*italic*` is `<em>`, `` `code` `` is
`<code>`, and an email address is a `mailto:` link. The two date lines at the top of each page keep
`class="date"`. Markers and bracketed notes are never published.

## 0. The rulings this note follows (I1)

Quinn's rulings of 2026-11-17, as the dispatch gave them. Sub-points are joined into one line each.
- **R1.** "Who 'we' is: KNOWLU LLC, a Kentucky limited liability company. Kentucky law governs. The
  student may always use small claims court where they live; otherwise the courts of Jefferson County,
  Kentucky. No arbitration. The postal address is not decided yet, so write [MAILING ADDRESS] wherever
  one is needed (the notices clause, contact, and commercial email under CAN-SPAM). Drop 'an
  independent business in Alabama, run by its founder — one person'. Put nothing about owners or
  members in the pages." **Its venue sentence is overridden by U8.**
- **R2.** "Seller of record: Stripe Managed Payments, so Link is the seller of record. Write the
  payment, receipt, cancellation, refund and dispute clauses around 'Sold through Link', and verify from
  Stripe's docs what Link does and what Knowlu does."
- **R3.** "Prices: the signed amendment, with no Pell discount (Pell is dropped for now). $12.99 every 4
  weeks, or the yearly plan at $84.37 once a year. 14 days free, with the card taken first and a
  reminder before the first charge. No summer pause. Keep the gift year if the amendment has it. Always
  write 'every 4 weeks', never 'a month'. Billing is moving to these prices in a later lane, so this
  terms text is published together with that lane."
- **R4.** "Refunds: a 30-day money-back guarantee on the yearly plan, plus a refund of a 4-week charge
  the student didn't mean to make (define this tightly). knowlu.com now says '30-day money-back
  guarantee on the yearly plan. Cancel anytime.' Follow 16 CFR 239.3: state every condition plainly."
- **R5.** "Contact: support@knowlu.com, security@knowlu.com and hello@knowlu.com. All three are
  monitored."
- **R6.** "Authorization clause. The student authorizes Knowlu to act for them in their mailbox and on
  their school and homework portals with their own sign-ins: setting forwarding, forwarding older mail,
  removing Knowlu's Sent copies, and using the kept sessions. The student confirms their school and
  providers allow it. Add a non-affiliation line: Knowlu is not affiliated with Blackboard, zyBooks,
  VHL, Google, Microsoft or any school."
- **R7.** "Accuracy and integrity. Knowlu is not the official record, so check due dates with the course
  site or the instructor. Estimates can be wrong. Knowlu never submits or does work."
- **R8.** "FERPA positioning: Knowlu works for the student, not the school, and is not a school
  official. Do not claim FERPA compliance."
- **R9.** "Liability cap: the greater of $50 or what the student paid in the 12 months before the claim.
  No indemnity from the student. … mark it [QUINN TO CONFIRM]." **Overridden by U1.**
- **R10.** "Retention limits: telemetry and corrections deleted after 2 years. Consent records deleted 3
  years after the account closes, which needs a sweep job. Mark it [QUINN TO CONFIRM]." **Confirmed by
  U2.**
- **R11.** "v0.5.0 onboarding: two closed-choice answers (why the student downloaded Knowlu, and where
  they heard of it) are sent to Knowlu. Add them to the collected list as text that ships with v0.5.0,
  behind a privacy version bump and re-consent."
- **R12.** "zyBooks/VHL: the fetching and parsing is done by the app's own scripts. But judge.rs tier 1
  never sets importance, so every item's title, including zyBooks and VHL items, goes to the cloud model
(see the audit). Describe today's truth
  accurately. Add a recommended engine change: send zyBooks and VHL items to the model without their
  titles, only the kind, course and due date." **Its first sentence is corrected by D9:** the server
  parses.

Quinn's updates of 2026-11-17. Where a U differs from an R, the U wins.
- **U1** (overrides R9). "The service is provided 'as is', with no warranties. No liability for missed
  or late assignments, grades, or academic or any other indirect or consequential losses. Total
  liability is capped at what the student paid in the 12 months before the claim, with no $50 floor, so
  a trial user's cap is $0. Carve out only what the law will not let us exclude, 'to the fullest extent
  permitted by law'. No indemnity. In the open questions, note what makes a cap unenforceable for
  consumers."
- **U2** (confirms R10). "Telemetry and corrections are deleted after 2 years, and consent records 3
  years after the account closes."
- **U3** (confirms R12's change). "Students still see every title in the app and the vault; only what
  goes to the model drops the title."
- **U4** (adds to R1), stated without its private details per U7. KNOWLU LLC is a Kentucky limited
  liability company. If Knowlu shuts down, it tells customers, refunds prepaid time pro rata, and
  deletes student data as the privacy policy promises. The terms carry a shutdown clause and the
  privacy page the matching deletion promise, each [NEEDS CODE] where no mechanism exists. No member
  names, ownership details or street addresses appear in the pages.
- **U5** (confirms R2). "Keep Link as the seller of record in the draft." Its costs are being checked
  separately.
- **U6** (K2; overrides the amendment's Pell rulings). "Get rid of the Pell Grant deal for now, we'll
  reapproach it later." This supersedes the amendment's rulings (4)–(6) and the Pell parts of P1–P9 and
  P11. It also supersedes the v0.4.0 design note's v0.5.0 row "Pell 50% taken on trust"
  (`docs/specs/2026-11-17-v0.4.0-new-ui-design.md:97`).
- **U8** (overrides R1's venue sentence; Quinn, 2026-11-17). "Anyone suing Knowlu must do it in
  Kentucky." This is an exclusive forum-selection clause: the state or federal courts in Jefferson
  County, Kentucky, with small claims in Jefferson County's small claims division. "Small claims where
  the student lives" is removed. Kentucky law governs.
- **U9** (r2's Q1). The registered agent's office and the mailing address are different, and the pages use
  the mailing address. [MAILING ADDRESS] stays a placeholder in this note and is filled in when the
  pages are published. No street address goes in this note.
- **U10** (r2's Q3). The interim terms with bump A are approved, still subject to N3's production account
  count. The recommendation in r2's Q3 is taken with it: today's `academic_year` plan counts as "yearly" for
  the money-back line.
- **U11** (r2's Q4, Q5). The unintended-charge definition (D7, cases (a)–(d)) is approved. Titles in email
  matching stay as they are.
- **U12** (confirms R2 and U5). Managed Payments, with Link as seller of record, is confirmed. Prices are
  tax-exclusive: "Sales tax is added at checkout where it applies." Every price sentence follows that.
- **U7** (I12). The repository is public. This note cites no internal company document, names no
  members and gives no registration details. It states only the public facts: KNOWLU LLC, a Kentucky
  limited liability company.

## 1. Decisions, each with its reason

**D1. One page version per release that changes what is true, not one big rewrite.** There are two
privacy bumps. A is true now, and it is its own Contract PR after v0.4.0's re-homing. B goes with v0.5.0,
which carries the billing lane (`v0.4.0-new-ui-design.md:24-26, 97`), the onboarding answers and C15.
The terms get one new version with B, plus an interim version with A (approved, U10) if the production account count allows it (N3). The waitlist and O1
bumps are ordered in §5. *Reason:* `PRIVACY_VERSION` is compiled into the app and must equal the page's
Effective date (`app/tests/static_assets.rs:1677-1686`, which T23 moves to `bundle_config.rs`), and a
build carries exactly one `PRIVACY_CHANGES` key (`static_assets.rs:3459-3472`). So a release can carry
only one privacy bump. The page and the app publish together only while `site/` deploys on the release
tag (`release.yml:294-298`). The waitlist's site-only deploy (W-D3) would break that, so C25 keeps the
legal pages tag-only.

**D2. The one-line summary stays word for word, and the next sentence says what "shared" means.**
*Reason:* the sentence is pinned in three places: `engine/tests/site.rs:15` (on both `privacy.html` and
`index.html`); `console.js`'s `PRIVACY`, which `app/tests/static_assets.rs:1047-1061` checks against the
page; and the page itself. v0.4.0 drops the index pin (plan T4, A5) and moves `PRIVACY` to `js/wizard.js`
(plan T21, `:419-420`), which still checks it byte for byte. The audit is right that "never shared" is
only true in privacy law's sense, because the processors on the list handle the data. A qualifying
sentence right after the summary fixes the claim without touching the wizard. A new summary sentence
would be cleaner wording but touches more files, so I recommend the qualifier.

**D3. No company count.** "Seven companies" becomes "Each company below does one job". *Reason:* the
count was wrong (Resend was unnamed). Naming Resend, Google (the support mailbox) and later Link changes
it at every bump, and a count is one more number to go stale. `site.rs:125` only bans "Six companies".

**D4. Never the word "AI", and "FERPA" on neither page.** The privacy page and the re-consent panel's
lines say "model calls" where they said "AI calls" (the panel copy at `console.js:3129` too, m7).
*Reason:* the no-"AI" rule is Quinn's. `site.rs:32-33` already bans "FERPA", "compliant", "certified"
and "guarantee" from `privacy.html`. The not-a-school-official statement (R8) is on both pages without
the acronym. "Not a school official" is the substance, and naming the statute invites a reader to treat
the line as a compliance claim. The lawyer may add it (section 8).

**D5. Who "we" is.** The pages say KNOWLU LLC, a Kentucky limited liability company, and give
[MAILING ADDRESS]. Every founder and one-person line goes, including the Security section's "its
founder". The pages carry no member names, ownership details or street address, and this note cites
no internal company document (U7). The privacy page says KNOWLU LLC **operates** Knowlu, rather than
"makes and runs" it. The terms name it as the party to the agreement. *Reason:* R1, U4 and U7. Review r1
(I7) points out that the business accounts (Stripe, Supabase, Cloudflare, Google Cloud, AWS, Resend,
OpenRouter) may not be in the LLC's name yet. "Operates" holds either way; §9 Q1 covers moving them. This
also removes a single-person assumption from published text.

**D6. Payment clauses are written around "Sold through Link", and they are `NEEDS CODE: C5`.**
*Reason:* R2 makes Link the seller of record. But checkout today does not create Managed Payments
sessions: it passes `automatic_tax[enabled]` and `customer_update[address]` and no Managed Payments
parameter (`cloud/supabase/functions/billing-checkout/handler.ts:51-72`). Until C5 lands and a
test-mode checkout shows what Link sends, the Link sentences are not true.

**D7. A tight definition of "a 4-week charge you didn't mean to make" (R4).** It has four cases: three
are our error, and one is a forgotten renewal, which is once in 12 months, asked within 7 days, and
cancelled at the same time. *Reason:* each case can be checked from Stripe's records alone, with no
usage data. That keeps the refund promise free of telemetry, as VISION's telemetry rule requires. Quinn
approved the definition, the forgotten-renewal case included (U11).

**D8. The money-back clause states every condition in one place, next to the promise (16 CFR 239.3).**
The conditions are: yearly plan only, within 30 days of any yearly charge, the whole charge, access
ending the day of the refund, and one per account in 12 months. *Reason:* R4, the amendment's P12, and
the audit's gap 3. "Guarantee" appears only in `terms.html`, which `site.rs` does not scan. The live
knowlu.com line ("30-day money-back guarantee on the yearly plan") is safe as well: v0.4.0's T4 lifts the
"guarantee" ban from `index.html` when it ports the site (`v0.4.0-new-ui-plan.md:262-266`), and the ban
stays on `privacy.html` (n2).

**D9. The coursework claims are corrected to what the code does, and the code change goes on the
list.** This corrects ruling R12's premise. The app does fetch zyBooks and VHL on the device. But on an
account, the app posts the raw portal reply to `/ingest-coursework`, and **the server parses it**
(`engine/src/coursework.rs:1036-1060`, `cloud/supabase/functions/ingest-coursework/handler.ts:1-18`).
The server stores none of it: its only log line is an error class name (`handler.ts:197`). The page
says so. Every task title, zyBooks and VHL included, goes to the model, because tier 1 never sets
importance (`engine/src/judge.rs:440-451`). The judge request carries the title and body
(`engine/src/cloudmodel.rs:356-366`), and the prompt prints the title
(`cloud/supabase/functions/_shared/judge_prompts.ts:247`). The page says that too, and C15 changes it
in v0.5.0. The old terms' claims of a reading pace and stopping at the first block are dropped. The
audit found no throttle and no block detection, and I found none either.

**D10. The privacy page gains entries the audit found missing.** They are: the judgment record and the
per-account rules (title words, organizer, series, sender), the sign-in security log, mail sent to our
addresses, Resend by name, and Google as the support mailbox's host. *Reason:* the page promises a
closed list (P:33). The judgment record keeps `title_prefix`, the first three words of a title
(`judge_rules.ts:13-17, 62-78`). Promoted rules keep the same keys per account
(`migrations/20260911000400_rule_promotion.sql:129-130`). Mail to all three addresses is forwarded by
Cloudflare Email Routing to a Google-hosted mailbox (`HANDOFF.md:56`).

**D11. Liability: the strongest version that should still hold for a consumer, with no indemnity (U1,
which replaces R9).**
- **"As is", no warranties.** The implied warranties are disclaimed by name: merchantability, fitness
  for a particular purpose, non-infringement.
- **No liability for indirect losses.** Missed or late assignments, grades, and academic or any other
  indirect, incidental, special or consequential losses are excluded by name. The student is told
  plainly that Knowlu will sometimes miss or misdate work, and that the course site is the record.
- **The cap is what the student paid in the 12 months before the claim, with no floor**, so a trial
  user's cap is $0.
- **Only what the law will not let us exclude is carved out**, "to the fullest extent permitted by
  law". The clause also has a savings sentence: where a state forbids a limit, the limit applies only as
  far as that state allows.
- **The terms' own promises stand outside both clauses (I5).** The disclaimer is of warranties "other
  than the promises these terms make", so it cannot swallow the money-back guarantee, the refund cases,
  the shutdown refund or the notice promises. The cap says it "does not reduce a refund these terms
  promise", so the under-18 refund of everything paid can exceed 12 months of payments.
- **Conspicuous.** The disclaimer's operative sentence and the cap's sentences are bold (`<strong>`),
  not just their labels.

*Reason:* the system will sometimes fail, and the company cannot pay for missed work. What could make
this clause fail is in §9, for the lawyer. The order of the pages helps it hold. The accuracy section, the "official
record" warning and the warranty disclaimer sit before the cap, in bold where they matter, so the risk
is disclosed rather than buried. "No indemnity" means the student never promises to pay Knowlu's legal
costs. An indemnity from a consumer adds little and reads as hostile.

**D12. Retention limits (R10, confirmed by U2).** Interaction events and corrections are deleted
2 years after they are recorded (C4). Consent records are deleted 3 years after the account is deleted
(C3). *Reason:* California's auto-renewal law asks for 3 years, or 1 year after the contract ends,
whichever is longer (audit gap 2). Deleting 3 years after closing always meets both. C3 needs a
closing date on each surviving row, because today's purge nulls `account_id` and `ip` and records no
date (`cloud/supabase/functions/account/index.ts:108-114`). C4 must leave rule promotion working: it
reads judgments and corrections over the last 60 days (`20260911000500_rule_promotion_fix.sql:165, 191`;
`20261002000200_mail.sql:282, 298`), well inside 2 years (m2).

**D13. Placeholders, not guesses, for facts that live in a console.** These are the database backup
window, how long Supabase Auth keeps its sign-in log, the AWS one-day expiry, Gmail's testing mode and
the site's cookies. The sentences marked `[TRUE NOW — unverified, C21]` publish only after C21 has
recorded the fact in the repo. *Reason:* the audit could not verify any of them (P:83, 103, 106, 163).
C21 also records each processor's region and whether a data processing agreement is in place (m4). The
page gains a region sentence only after that.

**D14. No Pell text anywhere (K2, U6).** Neither page has a Pell price, claim, award letter or
shortened year. The privacy page has no award-letter data class, and the lawyer list drops the Pell
item. *Reason:* Quinn's ruling U6 removes Pell for now. The billing lane must not build a Pell price, so
the amendment's P9 has nothing to disclose. U6 has to be on record as an amendment to 2026-11-10 before
the billing plan starts (C26), or the billing plan would follow the signed text and contradict these
terms.

**D15. "Two kept sign-ins" stays until O1 adds a third (I10).** The onboarding draft's shared school
profile (`docs/specs/2026-10-23-onboarding-redesign-draft.md:91-92`) would be a third kept sign-in.
Whichever lane builds it owns a privacy bump that rewrites the pinned sentences
(`site.rs:226-227`, `:239-245`) and the *Kept sign-ins* bullet. Until then the pages say two, which is
true.

**D16. No promise about restoring backups (I4).** Bump A says only that backups hold a deleted account
until they roll off. *Reason:* the one record of a deletion is the `deleted_accounts` tombstone. It lives
in the same database, so a restore rolls it back too, and it lasts only 90 days
(`migrations/20260910000300_deletion.sql`; `account/index.ts:155-156`). Nothing could name the accounts
to delete again. C27 (a deletion log outside the database) would make the stronger promise keepable. It
is M-sized, not small, so it goes to Launch.

**D17. One bump, one squash-merged PR (I11).** The version tie fails at any commit where the page and
the constant differ, so each bump is one PR: red pins first, then the page, the constant and the
`PRIVACY_CHANGES` key. It is squash-merged, so `main` sees one commit, and CI gates the PR head.
`contract-engineer` makes the commit that moves the constant (`app/src/account.rs` is on the contract
list) and carries the other agents' files into it.

## 2. What must not break

- **Every pinned privacy sentence stays word for word, except one listed edit.** The pins are in
  `engine/tests/site.rs:15-298`: the summary sentence, "encrypted at rest", `<dt>Your tasks and notes</dt>`,
  "400 days", the journal and computer-name sentences, the kept-sign-in sentences, the forwarded-mail
  phrases, the export sentence, the grades sentences, the Gmail-disconnect and decision-record
  sentences, the security sentence, "Settings → Delete my data", the syllabus parenthesis and the
  smaller-change sentence. The text in section 3 keeps all of them. **The one edit:** `site.rs:205`
  pins "We keep daily counts of your AI calls and cloud requests". D4 changes it to "model calls", and
  the pin moves in the same commit (C18).
- **Banned words on `privacy.html`** (`site.rs:32-33`): FERPA, compliant, certified, guarantee. The page
  also carries no `<script>` (`site.rs:22`). v0.4.0 T4 lifts these bans from `index.html` only.
- **The version tie.** The privacy page's `Effective {v}.` and `This page is version <strong>{v}</strong>`
  equal `PRIVACY_VERSION` (`app/src/account.rs:25`; `static_assets.rs:1677-1686`). The build's
  `PRIVACY_CHANGES` has exactly one key, and it is that version (`static_assets.rs:3417-3472`). Its lines
  carry no typed year and must name "Gmail" and "Amazon" (`static_assets.rs:3474-3482`). The "Last
  changed" date is no earlier than the version (`site.rs:186-199`). The privacy page's "Your account"
  entry names the Google account id, the name and the picture link (`static_assets.rs:1692-1698`).
- **Where those pins live after v0.4.0 (K1).** These cites are today's files. Per the v0.4.0 plan, the
  re-consent panel and its tests move to `app/static/carried/reconsent.js` and
  `app/tests/ui_account_privacy.rs` (T20, `:185`). The wizard's `PRIVACY` moves to `js/wizard.js` (T21,
  `:419-420`). The version-tie test moves to `app/tests/bundle_config.rs` (T23, `:446`). Bump A is cut
  from `main` after the v0.4.0 tag (N1). It edits the moved files, never the old line ranges, and it
  changes no v0.4.0 task. The exact new locations of
  `PRIVACY_CHANGES` and of the panel tests are `unverified` until T20 merges.
- **`TOS_VERSION` equals the terms' date** (`app/src/account.rs:21-24`, whose comment says to bump both
  in one commit). No test checks this today; C18 adds one.
- **The cancel path.** Today the terms tell the student to click "Manage subscription", the Settings
  button's exact label (`static_assets.rs:1081-1083`). `billing-jobs/handler_test.ts:153` pins the same
  words in the billing emails. v0.4.0 rebuilds Settings with an "Account, Membership" row that has
  `open_portal` (`v0.4.0-new-ui-design.md:303`), and T16 re-homes the label pin to its own sections.
  C29, in bump B, gives that row the label "Manage subscription" and pins it, so the terms and the
  emails are true when the cancel path publishes in v0.5.0 (I14, N2). Until then, the interim terms say
  only "in Settings".
- **Both versions move in `app/src/account.rs`, which is on the contract list.** Every bump in section 5
  is a `contract-engineer` task that `contract-reviewer` reviews.
- **No promise the code does not keep.** Each `NEEDS CODE` paragraph publishes only in the release
  whose code makes it true (D1).
- **VISION holds.** Knowlu never sends anything to anyone else, and never submits or does work
  (commitment 3, "Knowlu reads"). Portal credentials never leave the device. Telemetry stays closed
  choices with no free text. No single-user assumption appears in published text (D5).

## 3. The new `privacy.html`

This is the text as of bump B (v0.5.0), with every paragraph marked. Bump A publishes the [TRUE NOW]
paragraphs. Where a paragraph has an older version that publishes first, both are given: "A:" is the
bump A text and "B:" is its v0.5.0 replacement.

---

# Privacy policy

*(date)* Effective [PRIVACY DATE]. This page is version **[PRIVACY DATE]** — the version Knowlu writes down beside your name on the day you accept it. **[TRUE NOW]**

*(date)* Last changed [PRIVACY DATE]. **[TRUE NOW]**

Your tasks and notes live in a plain-text folder on this machine and in your Knowlu account, so every computer you sign in on opens on the same day; our servers keep them encrypted at rest, beside your account, the judgments made for you and what you correct, and none of it is ever sold or shared. **[TRUE NOW · unchanged; pinned, D2]**

That is the whole policy in one line. "Sold or shared" means what privacy law means by it: nobody buys your data from us, and nobody gets it to use for themselves. The companies named under *Who else touches it* each handle part of it only to do one job for Knowlu. The rest of this page is the same promise said slowly, so you can check it. **[TRUE NOW]**

## Who we are, and how to reach us

Knowlu is operated by KNOWLU LLC, a Kentucky limited liability company ("Knowlu", "we"). There is no support department and no ticket queue: write to support@knowlu.com and a person reads it. Questions about this policy, a request to see or correct or delete what we hold, and anything you think we got wrong all go to that address. Security problems go to security@knowlu.com, and anything else to hello@knowlu.com. A person reads all three. By post: KNOWLU LLC, [MAILING ADDRESS]. **[TRUE NOW ("operates", D5; §9 Q1); [MAILING ADDRESS] is filled at publication, U9]**

We answer a request about your data within 30 days. If we turn one down, we say why, and you can ask us to look again by replying; we answer that within 30 days too. **[TRUE NOW]**

## What stays on your machine

Knowlu is a desktop app first. These things live on your PC and are yours. Your vault also has a copy in your account, described below, so that every computer you sign in on shows the same day; your coursework logins, the kept sign-ins and your backup folder never leave this machine; app data leaves only in an issue report you choose to send, and a syllabus only as the lines described further down, in the section on what Knowlu does not collect. **[TRUE NOW · unchanged]**

- **Your vault** — the folder Knowlu makes at `%USERPROFILE%\Knowlu\`. *(rest of the bullet unchanged: plain Markdown, the journal, the commitment questions and the series list that stay on this computer)* **[TRUE NOW · unchanged]**
- **App data** — `%LOCALAPPDATA%\knowlu\`: which profiles exist, your settings, when you last looked, the logs of about your last twenty runs (Knowlu keeps the newest eighty log files), and a short record of each time Knowlu closed. **[TRUE NOW]** *(was "the last sixty run logs", which is false: `app/src/scheduler.rs:30-35`)*
- **Coursework logins** — *(unchanged)* **[TRUE NOW · unchanged]**
- **Kept sign-ins, if you use them.** *(unchanged, all of it, including the cache sentence)* **[TRUE NOW · unchanged; pinned]**
- **Your backup folder** — *(unchanged)* **[TRUE NOW · unchanged]**
- **Your syllabi.** *(unchanged, with its parenthesis)* **[TRUE NOW · unchanged; pinned]**

**Kept sign-ins at every school, for now.** *(unchanged)* **[TRUE NOW · unchanged; pinned]**

## What we collect, and why

By category, with the reason it exists. If a category is not on this list, we are not collecting it. **[TRUE NOW · unchanged; true again once D10's entries are added]**

**dt:** Your account
*(unchanged, every word, and add at the end:)* Our hosting provider's sign-in system also keeps a security log of each sign-in: when it happened, the IP address it came from and the app or browser that made it. **[TRUE NOW · unverified, C21]**

**dt:** Your subscription
**A:** *(unchanged)* **[TRUE NOW · unchanged]**
**B:** Your subscription is sold through Link, Stripe's checkout service, which is the seller of record. Link and Stripe take the payment and hold the card. **We never see a card number.** Link also asks for your billing address, because whether a state charges sales tax on Knowlu depends on where you are, and Link holds that too. What we keep is a Stripe customer id, which plan you are on, whether it is active, and when the current period ends. We also keep a record of the agreement itself — the plan, the price, the renewal terms you were shown, the version of the terms, the moment, and the IP address it came from — because California requires a subscription seller to be able to show what the customer agreed to. **[NEEDS CODE: C5, C7; Link's role unverified (Stripe)]**

**dt:** Your tasks and notes
*(unchanged)* **[TRUE NOW · unchanged; pinned]**

**dt:** Your calendar links
*(unchanged)* **[TRUE NOW · unchanged]**

**dt:** Your coursework logins
**A:** **On your machine only, in Windows Credential Manager, never on our servers.** Knowlu signs in to zyBooks and VHL from your PC, with your credentials. The site's reply, which is its own record of your assignments in its own format, is sent to our server. The server reads the assignment list out of it so that one parser can serve everyone, sends the list back to your computer, and keeps none of the reply. The password stays on the machine, and there is no column in our database for one. **[TRUE NOW]** *(`engine/src/coursework.rs:1036-1060`; `ingest-coursework/handler.ts:1-18, 197`)*

**dt:** Your Google connection, if you make one
*(unchanged)* **[TRUE NOW · unchanged; pinned]**

**dt:** Email you forward to Knowlu, if you set it up
*(unchanged)* **[TRUE NOW · unchanged]**

**dt:** Your forwarding addresses
*(unchanged)* **[TRUE NOW · unchanged]**

**dt:** Forwarding batches and delivery checks
When Knowlu forwards older or missed Gmail mail, it keeps a record of each batch: when it was sent, how far back it reached, how many messages were sent and received, and the mail service's ids for the messages it counted, so it can tell when a batch is complete. It also keeps a record of each delivery check it sends to a mailbox. Neither holds any message text. Both are deleted 30 days after the batch closes or the check is sent. **[TRUE NOW]** *(m4, n1; `cloud/supabase/functions/mail-process/sweep.ts:10-14`; the batch row's phase, "since" date, keyed nonce and receipt ids per review r2: `20261002000200_mail.sql:84-97`, `20261020000400_mail_follow_ups.sql:31`)*

**dt:** What you do in Knowlu
*(unchanged)* **[TRUE NOW · unchanged]**

**dt:** Why you came, and where you heard of Knowlu
When you set Knowlu up, it asks why you downloaded it and where you heard of it. You answer each question by picking from a short list, never by typing. We keep your two answers with your account, kept and deleted like the interaction events above, to learn which reasons and places bring students to Knowlu, and for nothing else. **[v0.5.0; C16]**

**dt:** Daily counts
We keep daily counts of your model calls and cloud requests, never what they were about, to keep the service's cost in bounds. They are in your export and are deleted with your account. **[TRUE NOW; "AI" → "model", pin edit C18]**

**dt:** Free tester access, if Knowlu gives it
*(unchanged, except the last sentence:)* Knowlu pays for a tester's model calls, so we look at your daily counts and what they cost. **[TRUE NOW]**

**dt:** Your corrections
*(unchanged)* **[TRUE NOW · unchanged; pinned]**

**dt:** Judgments, and the rules learned from them
For each judgment Knowlu makes about one of your items, we keep a record: the item's id, what kind of item it was, which fields the judgment set and to what, whether a rule or a model answered, and when. So that Knowlu can learn rules for your account, the record also keeps a few short keys: the first three words of the title of a task or an event, what created a task (you, or one of Knowlu's own imports), the feed an event came from, an event's organizer and series, and an email's sender address — never an email's subject or its text. When the same answer for the same key keeps holding with no correction from you, Knowlu turns it into a rule for your account, so a model is not asked again. A rule keeps the key and the answer, and a rule learned from your items is made for your account only. Both are kept until you delete your account. **[TRUE NOW]** *(m1: `created_by` and `source` too, `judge_rules.ts:13-17, 46-47, 56, 62-78`; `migrations/20260911000100_judgment_service.sql:61-76`; `migrations/20260911000400_rule_promotion.sql:129-130`)*

**dt:** Diagnostic logs
*(unchanged)* **[TRUE NOW · unchanged]**

**dt:** Gmail, if you connect it
*(unchanged)* **[TRUE NOW · unchanged]**

**dt:** Mail you send us
If you write to support@, security@ or hello@knowlu.com, we keep your message and our replies, so that we can answer you and keep track of what we said we would do. If you write to join the waitlist, we keep your address to send you an invite, and for nothing else. Mail to those addresses passes through Cloudflare's email routing and is kept in a mailbox Google hosts. We delete a conversation, or take you off the waitlist, when you ask. **[TRUE NOW]** *(`HANDOFF.md:56`; I3: today's waitlist is a `mailto:hello@`, `.superpowers/demo/site-dist/js/site.js:120` per review r1. The waitlist lane's own bump replaces this sentence when its form ships.)*

## What we do not collect

- **No birthdate.** *(unchanged)* **[TRUE NOW · unchanged]**
- **No card number.** *(unchanged)* **[TRUE NOW · unchanged]**
- **No campus password.** *(unchanged; it already names both kept sign-ins)* **[TRUE NOW · unchanged; pinned]**
- **No more of your text sent to a model than a judgment needs.** *(unchanged, then one sentence added after "…and grade weights;")* That includes the titles of the zyBooks and VHL assignments Knowlu brings in. **[TRUE NOW]**
  **B:** *(that sentence becomes)* When Knowlu judges a zyBooks or VHL assignment, a model is sent what kind of assignment it is, its course and its due date, with the same course and preference details any task is judged with, but never its title or its text; you still see every title in Knowlu and in your vault. When an email might be about one of them, its title and due date can still be among the course items the email is matched against. **[NEEDS CODE: C15; v0.5.0 (U3 confirmed). The last sentence stays: Quinn kept titles in email matching (U11)]**
- **What Knowlu never does with a syllabus:** *(unchanged)* **[TRUE NOW · unchanged]**

## Who else touches it

Each company below does one job, and only two of them ever have any content of yours read by a model. **[TRUE NOW]** *(was "Seven companies"; D3)*

- **Supabase** — *(unchanged)* **[TRUE NOW · unchanged]**
- **Stripe** — payments and the subscription. Stripe sees your card; we do not. **[TRUE NOW · unchanged]**
- **Link** — Stripe's checkout service, and the seller of record for your subscription: it takes the payment, holds your card and billing address, adds sales tax where it applies, sends your receipts, and handles payment disputes and any refund it grants. **[NEEDS CODE: C5; unverified (Stripe)]**
- **Cloudflare** — this website, the downloads, and passing on mail sent to our own addresses. **[TRUE NOW]**
- **OpenRouter** — *(unchanged)* **[TRUE NOW · unchanged]**
- **CoreWeave** — *(unchanged)* **[TRUE NOW · unchanged]**
- **DeepInfra** — *(unchanged)* **[TRUE NOW · unchanged]**
- **Amazon Web Services** — *(unchanged; "at most about two days")* **[TRUE NOW · unverified, C21]**
- **Resend** — sends our account and billing email (sign-in codes, reminders, notices) and the delivery checks Knowlu sends to a mailbox you forward from. It handles each of those messages whole, to deliver it; nothing from your vault or your mailbox goes through it. **[TRUE NOW]** *(`cloud/supabase/config.toml:81-90`; was the unnamed "provider" of P:86)*
- **Google** — hosts the mailbox our three addresses deliver to, so it holds any mail you send us. **[TRUE NOW]**

CoreWeave and DeepInfra are **the only two places your content is ever sent to be read by a model.** What they are sent is held for the length of the call and kept by neither of them; what we write down from a judgment is the answer. **[TRUE NOW · unchanged; pinned]**

Nobody else gets anything. There is no advertising network, no data broker and no analytics company anywhere on this list. We hand data to anyone else only when the law makes us, such as under a court order, and we tell you first unless the law forbids it. **[TRUE NOW]** *(replaces P:86's "One more…" paragraph, whose provider is now the Resend bullet)*

If KNOWLU LLC is ever sold or merged, your data passes to the new owner only under this policy, and anything that came from your Google account passes only with your consent, asked first. **[TRUE NOW]** *(audit gap 4, Workspace policy)*

## We do not sell or share your personal information

*(unchanged)* **[TRUE NOW · unchanged]**

## Gmail

*(All four paragraphs and the list unchanged, the Limited Use sentence included.)* **[TRUE NOW · unchanged; pinned]** The testing-mode paragraph is **[TRUE NOW · unverified, C21]** (100 users and seven-day expiry are Google Cloud console state). One sentence is added to the end of it: "Before Gmail opens to everyone, Google also requires an independent security assessment of how Knowlu handles Gmail data, repeated every year." **[TRUE NOW]** *(m4, audit gap 4: Google's restricted-scope verification)*

## Email you forward to Knowlu

*(Everything unchanged: both paragraphs, the table, the list of what Knowlu does inside the mailbox, and the closing paragraphs.)* **[TRUE NOW · unchanged; pinned]** The "one-day expiry" phrases are **[TRUE NOW · unverified, C21]**.

## How long we keep it

- **Account data** — until you delete it. Deleting is immediate, and it is a purge rather than a flag. Our hosting provider keeps backups of the database, and a deleted account stays in them until they roll off, within [BACKUP WINDOW]. **[TRUE NOW · C21 fills the window]** *(I4, D16: the restore promise is gone until C27)*
- **Your tasks and notes** — *(unchanged)* **[TRUE NOW · unchanged; pinned]**
- **Gmail** — *(unchanged)* **[TRUE NOW · unchanged; pinned]**
- **Email you forward to Knowlu** — *(unchanged)* **[TRUE NOW · unchanged]**
- **A tombstone** — after an account is deleted, a fingerprint (a SHA-256 hash) of the email address and the date stay for **90 days**, so that we can tell whether an address has deleted an account recently. The fingerprint cannot be read back into an address, but someone who already has an address could check whether it matches. After 90 days it is gone too. **[TRUE NOW]** *(`_shared/crypto.ts:29-34`)*
- **Issue reports** — *(unchanged)* **[TRUE NOW · unchanged]**
- **Consent records** — the record of which versions of the terms and this policy you accepted, and when; your 18+ attestation; and the record of the subscription agreement described above. A deletion does not take them: each loses your account id and the IP address it came from, and keeps the fingerprint of your email address, the version or the price, and the date — enough to answer "what did the seller show that day", and no name, no address and no account to attach it to. **[TRUE NOW]** *(`account/index.ts:108-114`: every row, not just the subscription one)* They are deleted **3 years after your account is deleted**, because California's subscription law requires a seller to be able to show what the customer agreed to for 3 years, or 1 year after the contract ends, whichever is longer. **[NEEDS CODE: C3]** **A, if C3 has not landed:** "They are kept for at least three years, because California's subscription law requires a seller to be able to show what the customer agreed to for that long." **[TRUE NOW]** *(m11: today's wording, with no upper limit)*
- **Interaction events and corrections** — deleted with the account, and in any case **2 years** after they were recorded. Until then they stay, because the corrections are what the judgments learn from. **[NEEDS CODE: C4]** *(A, if C4 has not landed: "Events and corrections" bullet unchanged.)*
- **Judgments and the rules learned from them** — until you delete your account. **[TRUE NOW]**
- **The sign-in security log** — [AUTH LOG WINDOW], kept by our hosting provider. **[TRUE NOW · unverified, C21]**
- **Mail you send us** — until you ask us to delete the conversation; a waitlist address, until you get your invite or ask to come off the list. **[TRUE NOW]**
- **Forwarding batches and delivery checks** — 30 days after the batch closes or the check is sent. **[TRUE NOW]** *(`sweep.ts:13-14`)*

**If Knowlu closes down.** If KNOWLU LLC ever stops offering Knowlu or is wound up, we tell you by email at least 30 days before the service stops, so that you can export what our servers hold. Then we delete it all, as this page describes for a deleted account. Only the consent records stay, without any account attached, for the time the law requires, and then they are deleted too. Your vault stays on your computer, because it is yours. **[NEEDS CODE: C13]** *(U4)*

## Your rights, wherever you live

*(Intro unchanged.)* **[TRUE NOW · unchanged]**

- **Know and access** — *(unchanged)* **[TRUE NOW · unchanged]**
- **Export** — *(unchanged, pinned sentences included)* **[TRUE NOW · unchanged; pinned]** **B:** insert "your two setup answers," after "your events,". **[v0.5.0; C16]**
- **Correct** — *(unchanged)* **[TRUE NOW · unchanged]**
- **Delete** — *(unchanged through "…the 90-day tombstone and the consent records described above")* — and Stripe's own record of the payments it took… *(rest unchanged)* **[TRUE NOW; "consent record" → "consent records"]** **B:** "— and Stripe's and Link's own records of the payments: your email address, your billing address, the card and the charges. Those are payment records with their own retention rules, and deleting them is not something this button does; Stripe's privacy policy and Link's are where to read what they keep and for how long." **[NEEDS CODE: C5]**

## Knowlu works for you, not your school

You chose Knowlu, and it works for you. It has no agreement with your school, it is not a school official, and your school does not see what Knowlu holds about you. What Knowlu reads from your school's systems, it reads as you, with your own sign-ins, for your own use. **[TRUE NOW]** *(R8; D4: no acronym on this page)*

## Knowlu is for people 18 or older

*(unchanged)* **[TRUE NOW · unchanged]**

## Security, and what happens if it fails

Everything between the app and our servers goes over HTTPS. What our database stores is encrypted at rest, and your calendar links are encrypted a second time before they are written, with a key the database does not hold; only our own server code that saves and fetches your feeds can use it. Portal passwords are the clearest case of all: they are never sent to us, so there is no server anywhere holding one. Your Knowlu sign-in lives in Windows Credential Manager, not in a file; the two kept sign-ins described above are the exception, kept in a browser profile on this computer. **[TRUE NOW]** *(the key sentence fixed: `account/index.ts:261-263` holds it too; the last sentence pinned)*

security@knowlu.com reaches the person responsible for security at Knowlu directly. If you find a way in, that is the address, and you will get an answer from a person. **[TRUE NOW]** *(founder line removed, D5)*

No system is perfectly safe, and this page is not going to pretend otherwise. If we determine that a breach has reached information about you, **we will tell you within the shortest time the law where you live requires, and in no case later than 45 days** after we determine it, with what happened, what was in it, and what to do about it. Some states allow 45 days, some 30, and some say only "without unreasonable delay"; we hold ourselves to the shortest that applies to you. **[TRUE NOW; C14 backs it]** *(audit: CO, FL and WA require 30 days)*

## Do Not Track

*(unchanged)* **[TRUE NOW · unverified, C21]** *(the live site's `<head>` and Cloudflare's own cookies are unchecked)*

## Changes to this policy

*(unchanged, the pinned last sentence included)* **[TRUE NOW for the in-app ask (`console.js:3127-3160`, `carried/reconsent.js` after v0.4.0 T20); the email is NEEDS CODE: C2. For bump A it goes by hand to every account holder (§5)]**

*(footer)* Back to Knowlu · Terms

---

## 4. The new `terms.html`

The terms publish as **one new version, with the billing lane in v0.5.0** (bump B). C1 (terms
re-acceptance) and C6 (the signed prices) gate the whole document. The markers show which paragraphs
are true today regardless and which wait on further items. Every price says "every 4 weeks" or "once a
year", never "a month" (R3). An interim version made of the [TRUE NOW] paragraphs ships with bump A
first (approved, U10), if N3's account count allows it. It is specified after the full text below.

---

# Terms

*(date)* Effective [TERMS DATE]. These terms are version **[TERMS DATE]** — the version Knowlu records beside your name on the day you accept them. **[NEEDS CODE: C1]**

This is the agreement between you and KNOWLU LLC, a Kentucky limited liability company ("Knowlu", "we", "us"). It is written to be read, once, by the person it binds. Where a sentence here is about money, cancelling or what we are responsible for, it is the sentence we will be held to. The privacy policy, which says what we collect and why, is part of this agreement. **[TRUE NOW; §9 Q1 on the processor accounts]**

## What Knowlu is, and what it is not

Knowlu is a Windows app and a service behind it, and it answers one question: what's next. It reads your school's calendar feed, the coursework sites you connect, your grades on Blackboard and, at schools Knowlu supports, your class schedule; your calendar; and, if you turn it on, your email, through forwarding to a private Knowlu address or through Gmail's own permission. From those it ranks the work you already have and puts the day in front of you. The ranking runs on your computer. The judgments it ranks with — how long something will take, how much it matters — are made by a language model in our cloud. What it finds is kept as plain files on your computer and in your account. **[TRUE NOW]** *(replaces T:16, which predates forwarding, grades, the registrar and the kept sign-ins)*

It is not a tutor, a grader, or a way of doing the work. **Knowlu never submits, answers or completes anything for you**, never posts to a course site, and never sends anything to an instructor or to anyone else. What it does change in your accounts is listed, in full, under *What you allow Knowlu to do*. **[TRUE NOW]** *(fixes T:17 "It never posts"; VISION "Knowlu reads")*

## Knowlu is not the official record

Knowlu copies due dates, grades and class times from your school's systems, and it estimates how long work will take and how much it matters. **Knowlu will sometimes be wrong.** A copy can lag or be wrong, a site can change without warning, a sign-in can expire, and an estimate is only a guess. **Your course site, your syllabus and your instructor are the official record.** Check a due date with them before you rely on it, and especially before you skip something because Knowlu did not show it. You decide what you do; Knowlu is a tool for planning it. **[TRUE NOW]** *(R7; supports D11)*

## Knowlu works for you, not your school

You chose Knowlu, and it works for you. It has no agreement with your school, it is not a school official, and it does not act for your school. Knowlu is not affiliated with, endorsed by or sponsored by Blackboard, zyBooks, Vista Higher Learning (VHL), Google, Microsoft or any school. Their names appear here only to say which services Knowlu works with. **[TRUE NOW]** *(R6's non-affiliation line; R8)*

## Who can use it

*(T:20 unchanged: the 18+ attestation.)* **[TRUE NOW · unchanged]**

One account is one person's. See *What we ask of you* below. If Knowlu gives you free tester access, these terms apply to you except the parts about payment. **[TRUE NOW]**

Knowlu is in early access and still changing: features may be added, changed or removed, and a source can stop working when a site changes. When a source stops working, Knowlu says so in the app rather than going quiet. **[TRUE NOW]** *(m4, audit gap 14; VISION "a source that breaks later fails visibly")*

## What you allow Knowlu to do

Knowlu works by using your own sign-ins on your own computer. By connecting a source, you authorize Knowlu to act for you, in your name, in the ways below and only in these ways: **[TRUE NOW]** *(R6)*

- **Your school and homework sites** (your school's sign-in, Blackboard, zyBooks, VHL, and your registrar where Knowlu supports it): use the session from your school's own sign-in page, where you type your password yourself and Knowlu never sees it; use the zyBooks and VHL logins you saved on this computer; or use the kept Blackboard sign-in you allowed. With those, read your assignments, grades, course list and class schedule. **[TRUE NOW]** *(m10; `site/privacy.html:69`)*
- **Your mailbox, if you set up forwarding:** turn forwarding to your private Knowlu address on, and off again when you disconnect; turn it back on if it was switched off, and then tell you; for Gmail, forward older mail, and mail that missed Knowlu, to that address, and move the copies those forwards leave in your Sent folder to your Trash; send the mailbox an occasional delivery check; and do these things later, without you there, with the saved mail sign-in you allowed. **[TRUE NOW]** *(P:123-130)*
- **Gmail and Google Calendar, if you grant Google's permission:** read them, as that permission allows. **[TRUE NOW]**

Knowlu does nothing else in those accounts. It never sends mail to another person, never deletes anything except the Sent copies of its own forwards, never turns forwarding back on for a mailbox you turned off in Knowlu's Settings, and never changes forwarding that points somewhere else without asking you. **[TRUE NOW]**

**You confirm that you are allowed to.** Your school, Blackboard, zyBooks, VHL, Google and Microsoft each have their own rules for your account. By connecting one, you confirm that the account is yours, and that its rules let you use a tool like Knowlu with it, including saved sign-ins and, for a school mailbox, automatic forwarding. zyBooks' and VHL's own terms restrict automated access to their sites; read them before you connect either. Knowlu signs in to them as you, from your computer, with credentials that never leave it, and only when it runs: on its schedule, when you connect a site, or when you ask it to. If you are not sure, ask your school, or skip the step: everything else in Knowlu works without it. If a site or your school tells you to stop, disconnect it. **[TRUE NOW]** *(I9: onboarding's `coursework-discover`, `app/src/onboarding.rs:71-133`, and the tray's Run now, `app/src/tray.rs:23`, per review r1. T:50's "discourage", "the pace a person would read at" and "stops at the first sign" are dropped, D9)*

You can withdraw this permission at any time: disconnect the source in Settings, turn off the saved sign-in, or delete your account. **[TRUE NOW]**

## The subscription

**Two plans, each renewing automatically until you cancel. The yearly plan: $6.49 every 4 weeks, billed $84.37 once a year. Renews yearly. Or $12.99 every 4 weeks, charged every 4 weeks. Sales tax is added at checkout where it applies. 14 days free first: your card is taken when the trial starts and is not charged until it ends, and we email you at least 3 days before the first charge. Cancel any time.** **[NEEDS CODE: C6, C9]** *(amendment rulings (1), (2), (7), P9, P11, P14, without their Pell parts (U6); yearly shown first, per ruling (2))*

**Automatic renewal.** Your subscription renews automatically at the end of each period — every 4 weeks, or every year — and charges the card on file the price of your plan until you cancel. There is no minimum term: you can cancel at any time, and cancelling stops the next charge. Each account gets one free trial. There is no summer pause. **[NEEDS CODE: C6]**

**Sold through Link.** Knowlu's subscription is sold through Link, Stripe's checkout service, which is the seller of record. Link takes the payment and holds your card, adds sales tax where it applies, sends your receipts, and handles payment disputes. You will see "Sold through Link" at checkout and on your receipts. KNOWLU LLC provides the app and the service and makes the promises in these terms, including the refunds below. Link has its own terms for the payment itself, which you accept at checkout. **[NEEDS CODE: C5; unverified (Stripe)]** *(R2, U5)*

If we ever change a price, we email you before the change takes effect, between 7 and 30 days ahead, with the new price, the date and how to cancel. A price change never applies to a period you have already paid for. **[NEEDS CODE: C10; window unverified (law)]**

**If you subscribed before [BILLING SWITCH DATE],** your plan keeps its price and its terms, the summer pause included, until you cancel, and these terms apply to everything else. Before a paused subscription starts charging again, we email you at least 7 days ahead with the amount and how to cancel. **[NEEDS CODE: C11]** *(amendment P10)*

## How to cancel

In Knowlu, open Settings, go to Membership and choose **Manage subscription**; cancelling there takes one click. You can also cancel at link.com, signed in with the email you paid with, or reply to any billing email or write to support@knowlu.com and we will cancel it for you. **No survey, no phone call, and nothing to keep clicking through** — cancelling is never harder than signing up was. **[NEEDS CODE: C5, C29. The Membership path is v0.4.0's (`v0.4.0-new-ui-design.md:303`); what "Manage subscription" opens under Managed Payments is unverified (Stripe)]**

Cancelling stops the next charge. You keep what you have paid for until that period ends, unless you take a refund, which ends access the day it is made. Every email we send about billing says how to cancel. **[NEEDS CODE: C12 for the refund half]**

## Emails about your subscription

You get these emails, from us or, where Link sends one for us, from Link: **[NEEDS CODE: C5 decides which come from Link]** *(m5)*

- **When you subscribe:** a confirmation you can keep, with the plan, the price, how often it renews and how to cancel. **[NEEDS CODE: C8]**
- **Before the first charge after your trial, at least 3 days ahead:** the plan, the amount, the date and how to cancel. **[NEEDS CODE: C9]**
- **Before the yearly plan renews, between 15 and 45 days ahead:** the amount, the date and how to cancel. **[NEEDS CODE: C10; window unverified (law)]**
- **Once a year, on either plan,** a reminder that you are subscribed, even though nothing has changed: what the subscription is, what it costs, how often it renews, and how to cancel. California requires this reminder; we send it to everyone. **[NEEDS CODE: C6 — the job exists for every plan (`billing-jobs/handler.ts:130-134`, audit) but its text still says "per month"]**

## Refunds

**30-day money-back guarantee on the yearly plan.** Ask within 30 days of a yearly charge, the first one or any renewal, and we refund the whole charge. Your access ends the day of the refund. Every condition is in this paragraph. It covers the yearly plan only, not the 4-week plan. The request must reach us within 30 days of the charge. An account gets one money-back refund in any 12 months. Ask by replying to a billing email or by writing to support@knowlu.com; you do not need to give a reason. **[NEEDS CODE: C5, C12]** *(R4, amendment P12, 16 CFR 239.3; whether "the whole charge" returns the tax under Managed Payments is unverified (Stripe))*

**A 4-week charge you did not mean to make.** The 4-week plan has no money-back guarantee. We do refund a 4-week charge in full in any of these four cases: **(a)** it came after you had cancelled or deleted your account; **(b)** you were charged twice for the same period; **(c)** it was the first charge after your trial and we had not sent you the reminder described above; **(d)** you forgot to cancel: you write within 7 days of the charge, you cancel at the same time, and you have not had a refund under (d) in the last 12 months. For (a) to (c), write within 30 days of the charge. After a refund under (d), your access ends the day of the refund. **[NEEDS CODE: C12; approved by Quinn, U11]** *(R4, D7)*

Except in the cases above, when we end your account (see *Ending it*), when Knowlu closes down, or where the law requires a refund, we do not refund the rest of a period. Link may also refund a charge under its own policy, and nothing here limits that. **[NEEDS CODE: C5, C13]**

If you were under 18 when you subscribed, tell us: we will refund what you paid and close the account. **[TRUE NOW · unchanged]**

If you think a charge is wrong, write to us first, because a refund from us is usually faster than a dispute with your bank. If you dispute a charge, Link handles the dispute, and while it is open the charge does not count as paid. **[NEEDS CODE: C5, C12]** *(amendment P8)*

## A year for someone else

You can buy a year of Knowlu for a student: one payment of $84.37, which never renews. Sales tax is added at checkout where it applies. You get a code to give them. We never ask you for their email or anything else about them. They redeem it on their own account, and near the end of the year we ask them whether they want to subscribe. Codes never expire, and an unredeemed code is refundable to you within 30 days of purchase. **[NEEDS CODE: C24 (gift lane)]** *(amendment ruling (9), P13; this paragraph publishes only when gifts ship)*

## Your data is yours

*(T:43-44 unchanged.)* **[TRUE NOW · unchanged]**

To run Knowlu, you let us store, copy and process what you put in Knowlu and what it brings in for you, only to provide Knowlu to you as the privacy policy describes. That permission ends when you delete your account, except for what the privacy policy says survives a deletion. **[TRUE NOW]** *(audit gap 14)*

## The app

We give you a personal permission, which you cannot transfer, to install and use the Knowlu app on computers you use while your account is active. Knowlu's source code may be published for anyone to read; that gives no permission to copy, change or redistribute it. Knowlu checks for new versions and offers to install them, and it can start with Windows; you can turn that off in Settings. Knowlu works with services that are not ours — Google, Microsoft, Stripe, Link and the sites you connect — and each is provided under its own terms. **[TRUE NOW; the source-code sentence is on the lawyer list]** *(the repo has no LICENSE file and no `license =` in any `Cargo.toml`)*

## What we ask of you

- **Don't share an account.** *(unchanged)* **[TRUE NOW · unchanged]**
- **Use Knowlu within your school's rules.** Knowlu does not do your work, and using it to get around an academic-integrity rule is between you and your school. **[TRUE NOW]** *(T:49's "We never ask for your university sign-in" is dropped as partly false)*
- **Don't attack the service.** *(unchanged)* **[TRUE NOW · unchanged]**

## Availability, and what does not work offline

*(T:55-56 unchanged.)* **[TRUE NOW · unchanged]**

## Ending it

**You** can cancel at any time, and delete everything at any time (Settings → Delete my data). Cancelling keeps the account and the vault; deleting removes both. Neither one is a refund of a period already paid for, except in the cases under *Refunds*. **[TRUE NOW · unchanged]**

**We** can close an account if the subscription goes unpaid, if the account is shared or used to attack the service, if you break *What we ask of you*, or if we stop offering Knowlu. Except where an account is doing active harm, we will email you first and give you time to get your data out. If we are the ones ending it, we refund the unused part of what you paid, counted by the day, and we keep what our servers hold for 30 days so you can still ask for an export before it is deleted. Your vault is on your computer either way, and closing an account never reaches into it. **[NEEDS CODE: C13]**

**If Knowlu closes down.** If KNOWLU LLC stops offering Knowlu or is wound up, we tell you by email at least 30 days before the service stops, and we refund the unused part of any period you have paid for, counted by the day. Then we delete what our servers hold, as the privacy policy says. The app keeps ranking your vault on your computer as far as it can without our cloud. **[NEEDS CODE: C13]** *(U4. The last sentence matches T:56's offline behaviour after the 72-hour grace, `engine/src/entitle.rs`.)*

## Limitation of liability, and the rest

**No warranties.** **Knowlu is provided "as is" and "as available". Other than the promises these terms make, and to the fullest extent permitted by law, we disclaim all warranties, express or implied, including the implied warranties of merchantability, fitness for a particular purpose and non-infringement.** We do not promise that Knowlu is free of defects, that it will find every assignment or date, that what it shows is accurate or current, or that it will be available when you need it. **[TRUE NOW]** *(U1; I5: the promises carve-out, and the operative sentences bold)*

**Limitation of liability.** **To the fullest extent permitted by law, Knowlu is not liable for any indirect, incidental, special, consequential or punitive loss, and in particular not for a missed or late assignment, a grade, any other academic consequence, lost time or lost data, even if we were told it could happen. To the fullest extent permitted by law, our total liability to you for anything arising out of these terms or Knowlu is limited to the amount you paid for Knowlu in the 12 months before the claim. If you have paid nothing, for example during a trial, that amount is zero.** This limit does not reduce a refund these terms promise. These limits do not apply to anything the law does not allow to be limited or excluded. Where a state limits them, they apply as far as that state allows. **[TRUE NOW]** *(U1, D11; I5; the risks are in §9, for the lawyer)*

**Governing law.** These terms are governed by the laws of the Commonwealth of Kentucky, without regard to its conflict-of-laws rules. This does not take away any protection that the law of the state where you live gives you and that an agreement cannot remove. **[TRUE NOW]** *(R1)*

**Disputes.** Write to support@knowlu.com first and give us 30 days to put it right; most things end there. If that fails, **any claim arising out of these terms or Knowlu must be brought in Kentucky: in the state or federal courts located in Jefferson County, Kentucky, or, for a claim within its limits, in the small claims division of the Jefferson District Court. Those courts alone may hear it, and both of us agree to their jurisdiction.** This applies to the extent the law where you live allows it. **We do not require you to arbitrate**, and if that ever changes we will tell you before it does and give you a way to say no. **[TRUE NOW]** *(U8, replacing R1's venue; the forum clause is bold so that it is conspicuous; "Jefferson District Court" as the home of Jefferson County's small claims division is `unverified (law)`; lawyer item 16)*

**Notices.** We send notices to the email address on your account and in the app, and you agree to receive them that way. Send notices to us at support@knowlu.com, or by post to KNOWLU LLC, [MAILING ADDRESS]. **[TRUE NOW; the address is filled at publication, U9]**

**Things outside our control.** We are not responsible for a delay or failure caused by something we cannot reasonably control, such as an outage at a provider we depend on, a site changing how it works or blocking access, or a natural disaster. **[TRUE NOW]**

**The rest.** These terms and the privacy policy are the whole agreement between us about Knowlu. If one part turns out to be unenforceable, the rest still stands. Not enforcing something once does not waive it. The parts that should outlast the agreement — refunds owed, *Your data is yours*, *No warranties*, *Limitation of liability*, *Governing law* and *Disputes* — do. You may not transfer this agreement. We may transfer it if the business is sold or merged; we will tell you first, and anything that came from your Google account moves only with your consent, asked first. **[TRUE NOW]** *(audit gap 4 fixed: T:67 said only "we will tell you")*

## Changes to these terms

When we change these terms materially, we email the address on your account at least 14 days before the change applies to you, and Knowlu asks you to accept the new version, with its date, before you go on. Your acceptance is recorded with that version. If you do not want the new terms, cancel; and if the change costs you money, cancel and we will refund the rest of the period. Smaller changes, such as a clearer sentence or a corrected address, get a new date and nothing more. A price change follows the notice rule under *The subscription*. **[NEEDS CODE: C1, C2]** *(fixes T:70: re-acceptance is not built, `account/handler.ts:223-247`)*

*(date)* Knowlu terms, version [TERMS DATE], effective [TERMS DATE]. **[NEEDS CODE: C1]**

*(footer)* Back to Knowlu · Privacy

---

### Interim terms with bump A (approved by Quinn, U10, subject to the account count)

These close review r1's I6 gap. Without them, bump A's privacy page would name a Kentucky LLC while the
live terms still say "between you and Knowlu", name Alabama law and courts (`site/terms.html:13,
65-66`), and say "We do not refund a month you have already used" (`:39`) beside a homepage that
promises a yearly money-back guarantee.

**The gate (N3), which Quinn's approval keeps (U10).** Before bump A is cut, the main
session runs one **read-only** count of production accounts (`select count(*) from public.accounts`,
`accounts.sql:11-21`), on Quinn's word, and reports the number. The interim terms ship only if that count
shows nobody but Quinn has an account. If anyone else does, the interim terms are skipped. The gap
between A and B is then accepted and goes on the lawyer list (item 14). Nobody hand-writes consent rows
for other people.

- **Version.** `TOS_VERSION` → [INTERIM TERMS DATE], in bump A's commit (contract list; D17).
- **Paragraphs carried from the full text above, word for word:** the opening paragraph (entity);
  *What Knowlu is*; *Knowlu is not the official record*; *Knowlu works for you, not your school*; *Who
  can use it*, with the early-access paragraph; *What you allow Knowlu to do*; *Your data is yours*;
  *The app*; *What we ask of you*; *Availability* (unchanged); the "You" paragraph of *Ending it*; and
  every paragraph of *Limitation of liability, and the rest*.
- **The subscription, interim wording:** "**Knowlu is a subscription that renews automatically until
  you cancel, at the price and on the schedule shown at checkout. Sales tax is added at checkout where it
  applies.**
  Stripe takes the payment and holds the card; we never see a card number. Cancel any time in Settings,
  or reply to any billing email and we will cancel it for you." **[TRUE NOW]** *(Stripe, not Link: C5
  has not landed)*
- **Refunds, interim wording:** "**30-day money-back guarantee on the yearly plan.** If your plan
  renews once a year, ask within 30 days of a yearly charge, the first one or any renewal, and we refund
  the whole charge. Your access ends the day of the refund. It covers yearly charges only. An account
  gets one money-back refund in any 12 months." Then: "**A charge you did not mean to make.** On a plan
  that renews more often than once a year, we refund a charge in full if it came after you had cancelled
  or deleted your account, or if you were charged twice for the same period. Write within 30 days of the
  charge." Then the under-18 paragraph, unchanged. **[TRUE NOW as a promise kept by hand; C12 automates
  the access end]** *(N4: the full text's case (c) is left out, because no trial reminder exists until
  C9, so (c) would refund every first charge on request. Case (d) is bump B's, approved in U11.)* Today's
  checkout sells `monthly` and `academic_year` (`billing-checkout/handler.ts:14-16`). Whether
  `academic_year` counts as "yearly" here (U10).
- **The summer pause, interim wording (n3):** "**The summer pause, on the monthly plan.** If you are on
  the monthly plan, billing stops for June, July and August and starts again on 1 September, and the app
  keeps working the whole time. On the academic-year plan nothing pauses. When the pause ends, we email
  you what you will be charged and how to cancel." **[TRUE NOW]** *(`billing-jobs/handler.ts:125-161`.
  The old "before the charge comes back" is not used, because the notice and the resume share one run
  (audit; C11). "Monthly" names today's plan as it is billed; R3's "every 4 weeks" rule governs the new
  prices in bump B.)*
- **Changes to these terms, interim wording:** the full text's paragraph without "Knowlu asks you to
  accept". **Recording Quinn's re-acceptance by hand (N3)** is a production write, done by the main
  session only on Quinn's go, after Quinn has accepted in writing. It is one insert into
  `public.consents` (`accounts.sql:36-45`): `account_id` = Quinn's account id; `subject_hash` = the
  SHA-256 of the account email, lowercased, exactly as `sha256Hex` makes it (`_shared/crypto.ts:29-34`);
  `kind` = `'tos'`; `version` = [INTERIM TERMS DATE]; `accepted_at` = `now()`; `price_cents` and `ip`
  null. Then one update of that account's `tos_version` and `tos_accepted_at` (`accounts.sql:15-16`). The
  server's own path cannot do this, because `recordLaterPrivacy` restamps only privacy
  (`account/handler.ts:223-247`). **C1 must land before any account other than the one that re-accepted
  by hand needs the interim terms.** New sign-ups record `TOS_VERSION` at sign-up and do not need C1.
- **Left out until bump B:** *The subscription*'s plan prices, *Sold through Link*, *How to cancel*'s
  Link path, *Emails about your subscription*, *A year for someone else*, the "We" and shutdown
  paragraphs of *Ending it* (C13).

## 5. Publication plan

**Every bump is one squash-merged Contract PR (D17)** that carries the same pieces: the page text and its
`Effective`/`Last changed` lines, the version constant in `app/src/account.rs` (contract list), one new
`PRIVACY_CHANGES` key with short lines (no typed year; they name "Gmail" and "Amazon";
`static_assets.rs:3430-3443, 3474-3482`), and the pin edits in C18. It goes live only with the release
tag that carries it (`release.yml:294-298`); C25 keeps any site-only deploy off the legal pages. **One
privacy bump per release**, because a build carries one `PRIVACY_CHANGES` key
(`static_assets.rs:3459-3472`).

| Bump | Rides on | What it publishes | Version moves | Re-consent it triggers |
|---|---|---|---|---|
| **Privacy A** (and the interim terms, if the account-count gate passes) | Its own Contract PR, cut from `main` **after the v0.4.0 tag** and shipped on the next tag (N1; provisional default, §9 Q2). v0.4.0's frozen plan, its whole-branch review (T26) and its pre-tag gate on the merge SHA (`v0.4.0-new-ui-plan.md:110, 191`) are untouched. It also carries C17 (N2). | Every [TRUE NOW] paragraph in section 3, with C3's and C4's retention sentences if they land first (recommended, both small; otherwise the "A" variants). This covers the entity, the three addresses and the waitlist purpose, Resend and Google, the judgment and rule entry, forwarding batches, the sign-in log, the coursework-parsing correction, the zyBooks/VHL titles sentence, the log count, the tombstone, key and breach sentences, and "model calls" (page and panel). If the account-count gate passes: the interim terms (approved by Quinn, 2026-11-17) | `PRIVACY_VERSION` → [PRIVACY DATE A]; if the gate passes, `TOS_VERSION` → [INTERIM TERMS DATE] | **Material** under the page's own rule, because categories and companies are newly named. The in-app panel (re-homed to `carried/reconsent.js` by T20) asks every account below the version. The page's email promise is C2, which is not built, so for A the email goes by hand to every account holder that the production count (N3, §4 interim) finds. C21 must fill [BACKUP WINDOW] and [AUTH LOG WINDOW] first |
| **Waitlist** | The waitlist lane (`waitlist-spec:docs/specs/2026-11-09-waitlist-design.md:48`, Q2), **on a release tag** (N5; provisional default, §9 Q2) | Its own data class: people who are not account holders | `PRIVACY_VERSION` | Its own. **It rides a release tag, never the site-only deploy (C25).** That changes W-D3's going-live order (`waitlist-design.md:125`): the form stays hidden until the tag carrying its privacy text deploys. The waitlist lane records this as an amendment to its own signed plan (C25b); this note does not edit that plan. **Order:** if it ships in the same release as A or B, the two become one bump with one key and one combined set of lines. Otherwise whichever merges second rebases its date past the first. Bump A's *Mail you send us* waitlist sentence is replaced by the waitlist lane's text |
| **Terms + Privacy B** | **v0.5.0**, which carries the billing lane (`v0.4.0-new-ui-design.md:24-26, 97`). This merges r1's B and C (I2) | All of section 4 (the terms). In section 3, every "B" variant: Link as seller, the consent record's plan and renewal terms, Link in Delete, the onboarding answers (C16) and "your two setup answers" in Export, and the C15 zyBooks/VHL sentence | `TOS_VERSION` → [TERMS DATE]; `PRIVACY_VERSION` → [PRIVACY DATE B] | **Terms:** every account accepts the new version before going on (C1; the server today never records a later terms version, `account/handler.ts:223-247`). New checkouts record it through checkout consent (C7). Subscriptions from before the switch keep their price and pause (P10). **Privacy:** the panel and the email (C2). The privacy guard (C16) blocks a release whose tree sends the onboarding answers while `PRIVACY_VERSION` is below B. The site's money-back line links to *Refunds* from this release on |
| **O1 third kept sign-in** (D15) | Whichever lane builds the shared school profile | The *Kept sign-ins* bullet and the pinned "two kept sign-ins" sentences (`site.rs:226-227, 239-245`), rewritten | `PRIVACY_VERSION` | Material: panel and email. Folds into B if it ships in v0.5.0 |
| **Gift** | The gift lane (C24) | *A year for someone else* in the terms, plus the giver's purchase record on the privacy page (amendment P13) | A terms bump and a privacy bump | Terms re-acceptance through C1, and the privacy panel |

**Order constraints.**
- A comes before B, so B's diff stays small and A's truths don't wait for billing.
- C1 lands before any terms bump that reaches more than one account.
- C26 (U6 recorded as an amendment, and Amendment 2026-11-10 merged to `main` with VISION's price line)
  lands before the billing plan starts, so VISION and the terms never disagree in a release (m9).
- C5's test-mode proof (Link's receipts, reminders, cancellation and refund role) comes before B's Link
  sentences are final. If any Stripe fact differs, section 4 is amended.
- The lawyer reads the B text before the first pilot student or the first paid sale, whichever comes
  first (`VISION.md:231-233`; Amendment 2026-10-16b; I13). A lawyer-read version that changes wording is
  another bump with re-consent.
- **T4's check (K1).** v0.4.0's T4 accepts on an empty
  `git diff --stat origin/main -- site/privacy.html site/terms.html …` (`v0.4.0-new-ui-plan.md:273-274`).
  Bump A is a separate PR cut from `main` after the v0.4.0 tag (N1), so T4's check runs and passes long
  before bump A exists. Plan
  §11 item 12 (`:792-796`) already says a legal change moved earlier "is its own Contract work, not
  part of T4". This note follows that rule.

**Who runs each bump.**
- `console-ui` drafts the page HTML from this note (extracted by heading; `site/` is its area per
  CLAUDE.md) and the `PRIVACY_CHANGES` lines in the re-homed file.
- `test-writer` writes the pins red first.
- `contract-engineer` makes the single commit that moves the constant, carrying the page, the key and
  the pins with it (D17).
- `contract-reviewer` and `reviewer` review.
- The main session runs the live check on staging before the PR is called ready: a scratch profile on
  an older recorded version sees the panel, accepts, and the server records the new version.

## 6. Code changes

The sizes are S (one or two files), M (three to five) and L (more). "Contract list" means a file named
in CLAUDE.md's list. "Contract workflow" means money or credentials, which CLAUDE.md sizes as Contract
even when no listed file is touched. Items C1–C14 and C17–C20 come from the audit's section 5; C15 is
R12/U3; C13 widens for U4; C16, C21–C24 are new here; C25–C29 are new in r2; C25b is new in r3.

| # | Change | Size | Contract list? | Version | Agent, and why |
|---|---|---|---|---|---|
| C1 | **Terms re-acceptance.** When the build's `TOS_VERSION` is later than the recorded one, the app asks, the way the privacy panel does. The server records a later `tos` version and row (today `recordLaterPrivacy` restamps only privacy, `account/handler.ts:223-247`) | M (account.rs, account handler, the re-homed panel in `carried/reconsent.js`, tests) | **Yes** (`app/src/account.rs`) | v0.5.0 (billing lane); before any terms bump that reaches a second account | `contract-engineer` (account.rs, xhigh), `cloud-engineer` (handler), `console-ui` (panel) |
| C2 | Email every account on a material policy change, from the version table | S–M (one function, one cron) | No | before bump B (A's email is sent by hand to the one account) | `cloud-engineer`: a `cloud/` function |
| C3 | **Consent sweep.** The purge stamps a closing date on each surviving consent row (`account/index.ts:114` patches only `account_id` and `ip` today), and a daily job deletes rows 3 years after it | S–M (migration, account function) | No | with bump A if possible; at the latest before its sentence publishes | `cloud-engineer` |
| C4 | Delete `telemetry_events` and `corrections` rows over 2 years old (daily job); rule promotion reads only the last 60 days (`20260911000500_rule_promotion_fix.sql:165, 191`; `20261002000200_mail.sql:282, 298`), so it is unaffected (m2) | M (migration, test) | No | with bump A if possible | `cloud-engineer` |
| C5 | **Managed Payments on.** Checkout passes the Managed Payments parameter and drops `automatic_tax` and `customer_update[address]` (`billing-checkout/handler.ts:51-72`). A test-mode run then records what Link sends (receipts, trial reminder), where a student cancels, what "Manage subscription" opens, whether Link refunds on its own, and whether a refund returns the tax | S code plus a spike | No; **Contract workflow** (money) | billing lane | `cloud-engineer` for the code; the main session runs the test-mode proof (no subagent holds a session) |
| C6 | **Signed prices.** $12.99 every 4 weeks (`interval=week`, `interval_count=4`), $84.37 a year, 14-day trial, one trial per account, no pause for new subscriptions, no Pell price (U6). The billing-email text ("per month"; `billing-jobs/handler_test.ts:150` pins "$9.99") and the checkout tests (`billing-checkout/handler_test.ts:21, 31-32`) move with it. v0.4.0 already takes prices out of the app (`v0.4.0-new-ui-design.md:24-26`) | L (≥8) | **Yes** (`app/src/account.rs`) | v0.5.0 (billing lane) | `contract-engineer` plus `cloud-engineer` and `console-ui`, under the billing plan |
| C7 | Checkout consent text that states the renewal terms, and a consent record that keeps the plan and the text shown (today: price, version, IP only, `billing-checkout/handler.ts:120-127`). The app's subscribe step states the plan, the billed amount, the every-4-weeks billing and, on yearly, the money-back terms before Checkout opens (amendment P9 without its Pell parts; m4) | S–M | No; **Contract workflow** | billing lane | `cloud-engineer` |
| C8 | California acknowledgment email after checkout, unless C5 proves Link's receipt meets the requirement (lawyer) | M | No | billing lane | `cloud-engineer` |
| C9 | Trial reminder 3 days before the first charge (amendment P14), without doubling Link's own, if C5 finds one | S–M | No | billing lane | `cloud-engineer` |
| C10 | Yearly renewal notice 15–45 days ahead, and a price-change notice path (7–30 days); both windows `unverified (law)` | S–M | No | billing lane | `cloud-engineer` |
| C11 | Pause-resume notice at least 7 days before the charge comes back (today the notice and the resume share one run, `billing-jobs/handler.ts:146-161`, audit). Old subscriptions only | S | No | billing lane | `cloud-engineer` |
| C12 | The webhook learns of refunds and disputes. A refunded or disputed charge is not a paid period (P8), and access ends the day of a refund | S–M | No; **Contract workflow** | billing lane | `cloud-engineer` |
| C13 | **Knowlu-initiated closure and shutdown** (widened for U4). An admin path closes one account with a 30-day hold, then purges, and refunds the unused days pro rata. A shutdown runbook covers every account: the notice email, the pro-rata refunds, the bulk purge, and the consent rows' later deletion | M–L (admin function, pro-rata refunds through Link, bulk shutdown, runbook) | No; **Contract workflow** | billing lane (before the first paid sale) | `cloud-engineer` (function); `docs-keeper` (runbook in `docs/reference/`) |
| C14 | Incident plan (30/45-day clocks, attorney-general notices, Kentucky's reasonable-security duty). A process note: the three addresses move to a business mailbox, with a rule for keeping mail. **The mail rule names its owner (n4):** the person who answers support@ and hello@ deletes a waitlist address once its invite is sent or its owner asks to come off the list, and uses waitlist addresses for nothing but the invite. (The restore rule left with I4; see C27) | S (docs) | No | before the first pilot student or paid sale | `docs-keeper`, with Quinn for the mailbox |
| C15 | **zyBooks and VHL items judged without titles** (R12, U3). `task_request` (`engine/src/cloudmodel.rs:343-375`) sends an empty title and body, plus a coarse kind, for an item with `effort_source: vendor` (the vendor marker, `judge.rs:437-447`). `Item` has no kind field (`judge.rs:48-58`), so the change derives one; the derivation rule is `unverified`. The prompt leaves out an empty `Title:` line (`judge_prompts.ts:247`). Changing the prompt changes its hash, so the eval runs again on `cloud/eval/seed/tasks.jsonl`, and importance must not regress on vendor items. The vault and app keep every title (U3). Accepted cost (m3): vendor items lose `title_prefix`, so tier-2 rules keyed on it stop applying to them; a test pins that their judgment record carries no `title_prefix`. The rest of the request (preferences, weights, known courses, `source_uid`, `created_by`, `cloudmodel.rs:356-372`) still travels. Mail-resolve candidates (`cloudmodel.rs:983-993`) are out of scope; Quinn kept titles there (U11) | M (engine 1–2 files and tests; cloud prompt and eval) | No | v0.5.0 | `implementer` (engine, test first), `cloud-engineer` (prompt, eval) |
| C16 | The two onboarding answers: closed-choice values only, stored like telemetry, included in export and deletion, kept 2 years (C4). **Guard (m8):** `scripts/ci/privacy-guard.ps1` gains a marker for the answers' code, in the D18 pattern (`privacy-guard.ps1:7-11, 41-44`), so a release that carries it while `PRIVACY_VERSION` is before [PRIVACY DATE B] fails. `engine/tests/workflows.rs` runs it against synthetic trees | S–M | No | v0.5.0 | the v0.5.0 lane's owner; `cloud-engineer` for storage |
| C17 | The app's Google sign-in line (`app/static/index.html:129` today, "email address and your name, and nothing else") matches the page's three things. **Moved into bump A's PR (N2):** after the v0.4.0 tag, the new wizard's line in `js/wizard.js` is changed there. No v0.4.0 task changes | S | No | bump A (after the v0.4.0 tag) | `console-ui`, inside bump A's PR |
| C18 | **Pins.** Update `site.rs:205` ("AI calls" → "model calls"); add pins for bump A's new sentences; extend the no-"AI" test to the `PRIVACY_CHANGES` lines (m7); keep the panel naming "Gmail" and "Amazon" (`static_assets.rs:3474-3482`, re-homed by T20); add a terms test (§7 items 8–9). The "guarantee" ban on `index.html` is lifted by v0.4.0 T4 (`v0.4.0-new-ui-plan.md:262-266`), so nothing is owed here | S | No | with each bump | `test-writer` (test first) |
| C19 | Export adds judgments, the Google connection's details and Gmail proposals (`account/index.ts:157-189`); the page already states the gap | S–M | No | Launch | `cloud-engineer` |
| C20 | Keyed hash (HMAC with a server secret) for the tombstone and consent rows (`_shared/crypto.ts:31-33`). Old rows cannot be rehashed without the email, so they age out | M | No | Launch | `cloud-engineer` |
| C21 | **Capture console facts in the repo:** the database backup window, the Supabase Auth log fields and how long they are kept, the AWS one-day lifecycle rule (`mail-process/sweep.ts:5`, audit), Gmail's testing-mode state, the live site's `<head>` and Cloudflare cookies, and each processor's region and data processing agreement (m4) | S (docs) | No | before bump A | the main session (consoles), `docs-keeper` writes it down |
| C22 | Commercial email (waitlist and launch) carries KNOWLU LLC, [MAILING ADDRESS] and an opt-out (CAN-SPAM, audit gap 13) | S | No | the waitlist lane | `cloud-engineer` |
| C23 | *Not a change:* the coursework throttle and block detection the old terms claimed. The claim is dropped (D9). Build them only if the lawyer asks | — | — | — | — |
| C24 | The gift year (amendment ruling (9), P13), through its own spec | L | No; **Contract workflow** | the gift lane | under its own plan |
| C25 | **Legal pages deploy only on a release tag (I3).** The waitlist's site-only deploy mode (W-D3, `waitlist-design.md:125`) refuses to run, or leaves those files alone, when `site/privacy.html` or `site/terms.html` differ from the latest published release's tag. An `engine/tests/workflows.rs` pin holds it. **Consequence (N5):** the waitlist's own privacy bump rides a release tag, and the form stays hidden until that tag deploys | S | No; but it is a release workflow, so **Contract workflow** | before any site-only deploy runs | the waitlist lane builds it once C25b is signed; `contract-reviewer` reads it, as that lane already requires |
| C25b | **Amendment to the waitlist plan (N5), a separate item.** The waitlist lane amends its own signed design: W-D3's site-only deploy refuses to publish a legal-page change (C25), so the waitlist's privacy bump rides a release tag and the form stays hidden until then. This note names the amendment but does not edit that plan | S (docs) | No | before the waitlist lane's deploy task (its W5) | the waitlist lane's owner, with Quinn's signature |
| C26 | **Record U6 (Pell dropped) as an amendment to 2026-11-10, and merge Amendment 2026-11-10 to `main`** with VISION's price line (`VISION.md:153-156`) in Quinn's words. The v0.4.0 design's "Pell 50% taken on trust" row is struck (`v0.4.0-new-ui-design.md:97`) | S (docs) | No | before the billing plan starts | the main session (Quinn's words), `docs-keeper` for the markers |
| C27 | **Deletion log outside the database (I4).** The account function writes a keyed hash of each deleted account's id, with the date, to storage that a database restore does not roll back. A restore runbook re-runs the purge for each id logged after the backup. With it, the privacy page can promise re-deletion after a restore | M | No | Launch | `cloud-engineer` |
| C28 | **Site FAQ wording (m4).** The site says data "goes to Knowlu's servers to plan your day" (site-dist `index.html`, per review r1), but the ranking runs on the computer. Reword it to match the terms | S | No | any site release after v0.4.0's T4 | `console-ui` |
| C29 | **"Manage subscription" on the Membership row (I14).** The Membership row's `open_portal` button carries the label "Manage subscription", pinned in `ui_settings.rs`, because the terms and the billing emails (`billing-jobs/handler_test.ts:153`) name it. **Moved into bump B (N2):** the cancel path publishes only in v0.5.0, and the interim terms say only "in Settings". No v0.4.0 task changes | S | No | v0.5.0 (bump B) | `console-ui`, under the billing plan |

## 7. Tests that prove it

Each test is written first and fails on today's page, then passes on the bump.

The pins below are in today's files. After v0.4.0, the panel and wizard pins are in
`app/tests/ui_account_privacy.rs` (T20) and `ui_wizard.rs` (T21), and the version tie is in
`app/tests/bundle_config.rs` (T23). Each test goes to whichever file holds its subject when bump A is
cut (K1).

**Bump A** (`engine/tests/site.rs` and the re-homed app tests):
1. Every existing pin still passes, except `site.rs:205`, which now expects "model calls".
2. A new test: no standalone word "AI" in the privacy page, the terms, or the `PRIVACY_CHANGES` lines
   (m7). Tokenize on word boundaries so that words like "email" pass.
3. New pins: "KNOWLU LLC, a Kentucky limited liability company"; the three addresses; "to send you an
   invite"; "Resend"; "the first three words of the title"; "Forwarding batches and delivery checks";
   "keeps none of the reply"; "about your last twenty runs"; "the shortest time the law where you live
   requires"; "someone who already has an address could check".
4. The old claims are gone: "the last sixty run logs", "an independent business in Alabama", "one
   person", "Seven companies", "only the code that fetches your feeds can use", "cannot be turned back
   into an address", and "we delete again every account".
5. The banned-word test (`site.rs:32-33`) still passes on `privacy.html`.
6. The version tie, the one-key `PRIVACY_CHANGES` test and **the panel names "Gmail" and "Amazon"**
   (`static_assets.rs:3474-3482`, I8) all pass at [PRIVACY DATE A].
7. `app/tests/account.rs` passes unchanged. It reads the constant and pins no date.
   `app/tests/grades.rs:183, 256` passes for any later version (review r1).
8. If the account-count gate passes (interim terms): a new test that the terms' `Effective {v}.` and
   `These terms are version <strong>{v}</strong>` equal `TOS_VERSION`. The interim terms contain "KNOWLU
   LLC", "Commonwealth of Kentucky", "Jefferson County", "shown at checkout", "Sales tax is added at
   checkout where it applies" and "The summer pause, on the monthly plan". They contain no "Alabama",
   no "small claims court where you live", and no "had not sent you the reminder" (N4).
8a. C17, inside bump A: the wizard's Google line names the email address, the name and a link to the
   profile picture (`ui_wizard.rs`, re-homed by v0.4.0 T21).

**Bump B (v0.5.0: terms, billing, onboarding answers, C15):**
9. The terms' date equals `TOS_VERSION` (as in 8). The terms contain "every 4 weeks", "$12.99",
   "$84.37", "Sold through Link", "30-day money-back guarantee on the yearly plan", "Other than the
   promises these terms make" and "does not reduce a refund these terms promise". They contain no "a
   month", "$9.99", "$69.99" or "Pell". "summer pause" appears only in the sentence "There is no summer
   pause." and in the paragraph that starts "If you subscribed before" (I8).
10. C1's tests are in its own plan: a later `tos_version` is recorded once, an equal or older one gets
    the silent 200, and the panel shows only on an exact `needs_accept === true`. This is the same shape
    as the privacy tests at `static_assets.rs:3405-3414`.
11. C6, C7, C9–C12: their own plans' handler tests. They update `billing-checkout/handler_test.ts:21,
    31-32` and `billing-jobs/handler_test.ts:150`, and keep `:153`'s "Manage subscription".
12. C15: an engine judge-request test. For an item with `effort_source: vendor`, the `/judge-task` body
    carries no title and no body text, and it carries the course, the due date and the derived kind. Its
    judgment record has no `title_prefix`. An item that is not from a vendor is unchanged. The eval on
    `cloud/eval/seed/tasks.jsonl` shows no importance regression on vendor items.
13. A pin for the onboarding-answers entry inside *What we collect* (the same shape as `site.rs:211-219`).
14. C16's guard: `engine/tests/workflows.rs` runs `privacy-guard.ps1` against a synthetic tree that
    carries the answers' marker. It blocks below [PRIVACY DATE B] and passes at it.
15. C29, in bump B: `ui_settings.rs` pins "Manage subscription" on the Membership row.
15a. Both versions of the terms: *Disputes* contains "Jefferson County, Kentucky" and "small claims
    division", and does not contain "small claims court where you live" (U8).

**Release path:**
16. C25 (built by the waitlist lane after C25b): a `workflows.rs` pin that the site-only deploy mode
    refuses, or skips the legal pages, when they differ from the latest published tag.

**The live run** (main session, staging, a scratch profile on a scratch vault). For each bump: start
with a recorded version below the new one, see the panel, accept, and read back the recorded version.
For B, also do a test-mode Managed Payments checkout that shows "Sold through Link", then cancel through
Settings → Membership → "Manage subscription".

## 8. Lawyer list (before the first pilot student or the first paid sale, whichever comes first)

The lawyer reads the B text (terms and privacy), and the interim terms if they ship (U10, N3). Timing follows
`VISION.md:231-233` and Amendment 2026-10-16b (I13). Pell is not on the list (U6).

1. **Link as seller of record.** What the terms must say, Link's legal name, who issues refunds and pays
   them back, whether Link's receipt counts as California's acknowledgment (C8), and whether "Sold
   through Link" plus KNOWLU LLC as the provider reads correctly.
2. **Automatic-renewal disclosures and notice windows** under California, New York, Colorado and ROSCA:
   the trial reminder at 3 days, the price-change window (7–30 days, unverified), the yearly renewal
   notice (15–45 days, unverified), the annual reminder, and the checkout consent text (C7).
3. **The money-back paragraph** against 16 CFR 239.3, including whether the refund returns the tax, and
   the four "didn't mean to" cases.
4. **Liability.** Whether the $0 cap for a trial user and the named exclusion of missed assignments and
   grades hold for consumers in the states that matter; the "as is" disclaimer's wording; and the absent
   indemnity (§9 sets out the risks).
5. **Kentucky law** against out-of-state consumers, and whether to name the Western District of
   Kentucky (see item 16 for the forum clause).
6. **The authorization clause** (agent, saved sign-ins, forwarding, trashing Sent copies) against
   Google's and Microsoft's terms, university forwarding policies and access-authorization law (*Van
   Buren*, *Power Ventures*; landscape note §6). Also zyBooks' ban on automated access and on processing
   its content, against server-side parsing of the raw reply (D9) and, until C15, titles sent to a model.
7. **Kentucky's breach law** (KRS 365.732), any Kentucky reasonable-security duty now that the entity
   is in Kentucky (m4, audit gap 7), whether **Kentucky's consumer data protection act** reaches Knowlu
   (all unverified (law)), and the 45-day promise for everyone.
8. **The shutdown clause** (U4): whether consent rows may outlive a wound-up LLC, and who holds them.
9. **The source-code sentence** in *The app*, for a public repository with no licence file.
10. **Whether to name FERPA** in the terms (D4), and the unverified age-of-majority states (Nebraska,
    Mississippi).
11. **Gift codes** against federal and state gift-card rules, when C24 is ready.
12. **The warranty disclaimer beside the guarantee** (I5): whether "other than the promises these terms
    make" squares the disclaimer with the money-back guarantee, and whether the cap could limit a
    promised refund.
13. **Whether the LLC is the contracting party with each processor** (I7, §9 Q1), and whether "operates"
    is the right word until the accounts move.
14. **The interim period** (I6, U10): the interim terms, or, if the account count stops them, a privacy
    page naming a Kentucky LLC while the published terms name Alabama.
15. **The waitlist record** (I3): what the privacy page must say about people who are not account
    holders.
16. **Forum selection against out-of-state consumers** (U8): whether the exclusive Jefferson County
    clause, small claims included, holds against consumers in other states, and which states limit or
    void it. The correct name of Jefferson County's small claims division is also for the lawyer.

## 9. Open questions for Quinn

**Settled by Quinn on 2026-11-17, and removed from this list:** the mailing address (U9: a placeholder
until publication); the interim terms (U10: approved, subject to N3's account count); the
unintended-charge definition and titles in email matching (U11); the venue (U8); Managed Payments and
tax-exclusive prices (U12). The $0 cap is the lawyer's question (section 8, item 4). The background U1
asked for is kept below.

**Q1. Should the business accounts move into KNOWLU LLC's name before the billing lane (I7)?** Quinn
has not ruled on this yet. Today the LLC exists, but the note does not know whether Stripe, Supabase,
Cloudflare, Google Cloud, AWS, Resend and OpenRouter are held by it. A page that names the LLC as the
party could then name someone other than the one contracting with those processors. *Recommendation:*
yes. Move them in this order: Stripe first, because it holds the money and the seller-of-record setup,
then Supabase, Cloudflare, Google Cloud, AWS, Resend and OpenRouter, all before the billing lane
(v0.5.0). Until then, the pages say the LLC "operates" Knowlu (D5), which holds either way. The terms
still name the LLC as the party to the student's agreement, and the lawyer checks that (section 8,
item 13).

**Q2. The publication order against other lanes (N1, N2, N5): confirm the three provisional
defaults.** Review r2 recommended each one, and the note is written to them. *Recommendation:* confirm
all three.
- **N1.** Bump A merges to `main` after the v0.4.0 tag and ships on the next tag. That leaves v0.4.0's
  frozen plan, its whole-branch review and its pre-tag gate untouched. Nothing in bump A is needed by
  v0.4.0.
- **N2.** C17 (the wizard's Google line) moves into bump A's PR, and C29 (the "Manage subscription"
  label) into bump B. So no v0.4.0 task changes.
- **N5.** The waitlist's privacy bump rides a release tag (C25), and its form stays hidden until that
  tag deploys. The waitlist lane amends its own signed plan to say so (C25b).

**For the lawyer: what can make the $0 cap unenforceable (U1; section 8, item 4).** All of this is
`unverified (law)`; the audit cites none of it.
- **Unconscionability.** A take-it-or-leave-it consumer contract is open to challenge when a term is
  hidden (procedural) and when it leaves the consumer no real remedy (substantive). A $0 cap is the
  clearest case of "no real remedy". A court may then strike the cap entirely, not just raise it, which
  leaves liability unlimited.
- **Exclusions the law forbids.** Most states will not enforce a limit on fraud, intentional harm,
  gross negligence, or a statute's own damages. A data-breach or privacy statute with statutory damages
  is one example.
- **State consumer-protection statutes.** Some make it unlawful even to show a consumer a term that
  breaks a clearly established right; New Jersey's TCCWNA is the known case.
- **Implied-warranty limits.** Some states do not let a seller disclaim implied warranties to consumers
  at all. Where a disclaimer is allowed, it usually must be conspicuous and name merchantability. The
  r2 draft names merchantability and sets the operative disclaimer and cap sentences in bold (I5).
  Whether that is conspicuous enough is on the lawyer list.
- **Hidden terms.** A limit placed far from what it limits, or in small print, can fail as hidden. The
  draft puts the risk up front: *Knowlu is not the official record* sits near the top, in bold.

The question for the lawyer is a narrow one: "does a $0 cap for non-paying users risk losing the whole
cap in our likely states, and would a nominal floor ($25–$50) prevent that?" The savings sentence
("where a state limits them, they apply as far as that state allows") already lowers the risk of the
clause being struck outright. The same reasoning applies to the exclusive forum clause (U8), and lawyer
item 16 covers that.

## Appendix: every audit finding, and where it is fixed

"Words" means the new text fixes it. "Cn" means the text waits for that code change.

| Audit finding | Fix |
|---|---|
| "sixty run logs" (false) | Words: "about your last twenty runs… eighty log files" |
| Closed list missing items | Words: D10 entries (judgments and rules, sign-in log, mail to us, Resend, Google) |
| Judgments keep title words | Words: *Judgments, and the rules learned from them* |
| Email provider unnamed; sees only the address | Words: Resend bullet |
| "Seven companies… nobody else" | Words: no count (D3); Resend, Google; Link with C5 |
| "Deleting is immediate" without backups | Words + C21 ([BACKUP WINDOW]); no restore promise until C27 (I4, D16) |
| Consent record "the one record" | Words: all consent records survive; C3 deletes them 3 years after closing |
| Tombstone "cannot be turned back" | Words; C20 later |
| Calendar key "only the fetching code" | Words |
| Breach "45 days" for everyone | Words: the shortest time the law requires, never past 45; C14 |
| "Never sold or shared" | Words: qualifier after the pinned sentence (D2) |
| Zyp/VHL "reads the list that comes back" | Words: the raw reply goes to the server, which parses and keeps none of it (D9) |
| Material-change email not sent | C2; by hand to the one account for bump A (§5) |
| Terms re-acceptance not built | C1 |
| $9.99 / $69.99 / 7-day | Words + C6 (signed prices, no pause); interim terms say "shown at checkout" (U10) |
| Pell: "taken on trust" vs the award letter; T:25 vs ruling (6); audit Q4; the Pell lawyer bullet | Dropped by Quinn's ruling U6 (D14); C26 records it as an amendment and strikes the v0.4.0 row |
| Pause-resume notice lead time | C11 (old subscriptions only) |
| Annual reminder "for a monthly subscription" | Words: "California requires this reminder"; C6 for the email text |
| Receipts link to the portal | Words around Link; C5 proves it |
| "It never posts" vs mailbox actions | Words: *What you allow Knowlu to do* lists every action |
| What Knowlu does (stale) | Words: new first paragraph |
| "Never ask for your university sign-in" | Dropped |
| zyBooks/VHL "discourage"; titles to the model | Words ("restrict"); titles disclosed now; C15 in v0.5.0; email matching keeps titles (U11) |
| Reading pace and stop-on-block claim | Dropped (C23) |
| Closure: 30-day hold, pro-rata refund | C13 (with shutdown, U4) |
| Gmail testing mode; Amazon expiry; cookies | C21 before bump A |
| Gap 1: entity, address, seller | KNOWLU LLC (D5), [MAILING ADDRESS] at publication (U9), Link (D6, C5, U12) |
| Gap 2: auto-renewal consent and notices | C7, C8, C9, C10; *Emails we send* section |
| Gap 3: money-back limits | *Refunds*, every condition in one paragraph (D8) |
| Gap 4: Google data on a merger | Words in both pages |
| Gap 4: CASA before Gmail leaves testing | Words: the Gmail testing paragraph gains the yearly assessment sentence (m4) |
| Mail batches and canaries, 30 days (`sweep.ts:10-14`) | Words: *Forwarding batches and delivery checks* entry and retention line (m4) |
| Gap 7: security duty | C14 and lawyer list item 7 (Kentucky) |
| Gap 12: age and COPPA | 18+ gate kept unchanged on both pages; Nebraska and Mississippi on lawyer list item 10 |
| P9: disclosure before the card | C7 (the app's subscribe step and Checkout, without Pell) |
| Site FAQ "goes to Knowlu's servers to plan your day" | C28 |
| Gap 5: authority and non-affiliation | *What you allow Knowlu to do*; *Knowlu works for you* |
| Gap 6: accuracy disclaimer | *Knowlu is not the official record* |
| Gap 8: subprocessors | Resend, Google, Link named; regions and data processing agreements recorded by C21 before any region sentence |
| Gap 9: retention limits | C3, C4 (U2 confirmed) |
| Gap 10: response time and appeal | Words: 30 days, ask us to look again |
| Gap 11: FERPA positioning | Words without the acronym (D4) |
| Gap 13: CAN-SPAM | C22, U9 |
| Gap 14: standard clauses | Licence both ways, updates and autostart, electronic notices, notices address, entire agreement, survival, force majeure, testers, early access (m4), disputes |
| Gap 15: cap, changes notice, contacts | U1 cap (D11; lawyer item 4); exclusive Kentucky forum (U8; lawyer item 16); 14-day notice + C1; three addresses |
| Inconsistency: app's Google copy | C17 |
| Inconsistency: live site "guarantee" vs `site.rs` ban | v0.4.0 T4 lifts the index ban; the privacy ban stays |
| Inconsistency: onboarding draft's third kept sign-in | D15: O1's own bump |
| Inconsistency: site mailto hello@ (waitlist) | Words: *Mail you send us* names the waitlist purpose (I3) |
| Inconsistency: v0.5.0 answers not listed | Bump B entry (C16), guarded by `privacy-guard.ps1` |
