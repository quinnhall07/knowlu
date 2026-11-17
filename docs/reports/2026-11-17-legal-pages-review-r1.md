FAIL

# Legal pages rewrite: spec review, round 1

Reviewer (Opus, high), 2026-11-17. Spec mode, round 1 of 2. Document: `docs/specs/2026-11-17-legal-pages-rewrite.md`
(untracked in the `v040-ui` worktree). The code was read at worktree HEAD `6dd9a7a4`, which is two docs-only
commits ahead of `origin/main` `84933416`, so the code is the same. Not legal advice. Items that need a lawyer are
marked "lawyer list".

**Verdict: FAIL.** 2 Critical, 14 Important, 10 Minor. Most of the [TRUE NOW] text holds against the code. The failures
are collisions with lanes already in flight (v0.4.0, waitlist, O1), a Pell position that contradicts the signed
amendment, one promise that the code cannot keep (re-deleting accounts after a backup restore), and a warranty
clause that contradicts the terms' own guarantee.

**The amendment source.** The working copy in the `waitlist-spec` worktree matches `origin/docs/waitlist-spec` at
`5dac15c7` (`git diff` is empty), so the note quotes the signed text. Amendment 2026-11-10 is **not on `origin/main`**:
main's cloud design still ends at the 2026-10-16 amendments, and `VISION.md:153-156` still says $9.99 a month, a 7-day
trial and the summer pause.

## Critical

**K1. Bump A cannot "ride on v0.4.0" as written.** (Note §5 Publication plan, row "Privacy A"; §6 C17; §7 Bump A.)
*Conflicts with:* `docs/plans/2026-11-17-v0.4.0-new-ui-plan.md:273-274`. T4's acceptance requires
`git diff --stat origin/main -- site/privacy.html site/terms.html …` to be empty. Its §11 item 12 (`:792-796`) says a
legal change moved earlier "is its own Contract work, not part of T4". v0.4.0 also moves every file the note edits:
- The re-consent panel and its tests move to `carried/reconsent.js` and `ui_account_privacy.rs` (T20, `:185`, `:624`).
- The wizard's `PRIVACY` moves to `wizard.js` (T21, `:419-420`).
- `the_privacy_version_constant_is_the_published_pages_date` moves to `bundle_config.rs` (T23, `:607`).
- The Google sign-in line C17 targets (`app/static/index.html:129`) is rebuilt by the new wizard.

*Consequence if executed:* bump A's commit either fails T4's acceptance check or edits `console.js` and
`static_assets.rs` line ranges that v0.4.0 deletes. The result is a merge conflict, or a `PRIVACY_CHANGES` key that
lands in a file the new UI no longer loads, so the panel shows nothing at the new version.

*Fix in text:* make bump A its own Contract PR, merged after v0.4.0's T20, T21 and T23 and before the tag (or on
the next tag). Retarget every cite to the re-homed files. Fold C17 into T21, or retarget it to the new wizard file.
Say how T4's empty-diff check is satisfied: run it before bump A merges, or amend the check.

**K2. Pell has three positions in three documents.** (Note §6 C6, "no Pell price"; §4 *The subscription*, which has
no Pell clause; §3, which has no award-letter data class; the appendix row "$9.99 / $69.99 / 7-day".) *Conflicts with:*
- Amendment 2026-11-10, signed 2026-11-11 (`waitlist-spec` worktree, `docs/specs/2026-09-09-knowlu-cloud-design.md:1033-1081`).
  Rulings (4)–(6) set Pell at 50%, claimed first and then proved by an award letter within 30 days. P9 requires the
  Pell terms to be shown before the card. P7 requires a notice before a shortened year renews.
- The v0.4.0 design note (`docs/specs/2026-11-17-v0.4.0-new-ui-design.md:97`), whose v0.5.0 billing row reads
  "Pell 50% taken on trust when the student states 'I receive/received a Pell Grant this year'".

*Consequence if executed:* terms B publish with no Pell terms while the billing lane builds a Pell price (P9 is then
breached at checkout). Or Pell is dropped without an amendment to a signed decision. The audit's Pell rows (§3
"Brief: Pell taken on trust", T:25 vs ruling (6), Q4, and the §6 lawyer bullet) are dropped without a reason.

*Decision for Quinn.* Recommendation: if R-something dropped Pell, write that ruling into the note and record it as
an amendment to 2026-11-10 before the billing plan starts. Otherwise add the Pell clause (rulings (4)–(6), P7), the
award-letter data class and its retention on the privacy page, and lawyer-list item 3's Pell bullet.

## Important

**I1. Rulings R1–R12 are not in the note.** (Header, line 8: "numbered R1–R12 below".) They appear nowhere below.
*Consequence:* nobody can check fidelity to them. Two places override or read them: D9 "corrects ruling R12's premise",
and C6 "no Pell price" (K2). Neither can be checked. *Fix in text:* quote R1–R12 and U1–U5 verbatim in a short
section, and mark where U overrides R.

**I2. If the billing lane is v0.5.0, bumps B and C land in the same release.** (§5 table, rows B and C.) *Conflicts
with:* the v0.4.0 design note's §6 row (`:97`), which puts the billing lane in v0.5.0, and
`static_assets.rs:3459-3472`, which allows exactly one `PRIVACY_CHANGES` key per build. *Consequence:* two privacy
versions in one tag. Either one of them is never asked, or B ships without its own panel. *Fix in text:* state the
release for the billing lane. If it is v0.5.0, merge B and C into one bump. Otherwise order them explicitly by tag.

**I3. Waitlist lane collision.** (§5; §6 C22 is the only mention of the waitlist.) *Conflicts with:*
`waitlist-spec:docs/specs/2026-11-09-waitlist-design.md:6-7, 48, 125`. That lane bumps `PRIVACY_VERSION` for a new
data class (people who are not account holders), and W-D3 adds a **site-only deploy**. *Consequences:*
- D1's premise ("the page and the app publish together", `release.yml:294-298`) stops holding. A legal-page edit
  merged to `main` could go live through a site-only deploy before the release whose `PRIVACY_VERSION` matches it.
  That breaks CLAUDE.md's rule that the version "never ships before a release deploys that page".
- The two lanes contend for the same version constant and `PRIVACY_CHANGES` key.
- Today's waitlist is a `mailto:hello@` (`.superpowers/demo/site-dist/js/site.js:120`). The new *Mail you send us*
  entry gives only "so that we can answer you" as its purpose, not "kept to send you an invite".

*Fix in text:* name the waitlist lane in §5, with an order. Add a rule, and a check, that legal pages deploy only
through a release tag. Add the waitlist purpose to *Mail you send us* in bump A.

**I4. The backup-restore promise cannot be kept.** (§3 *How long we keep it*, "Account data": "if we ever restore from a
backup, we delete again every account deleted after that backup was made", marked [TRUE NOW; C14 holds the restore
rule], with C14 sized "S (docs)".) *Conflicts with:* `account/index.ts:156-157` and
`migrations/20260910000300_deletion.sql`. The only record of a deletion is the `deleted_accounts` tombstone, which is
in the same database, so a restore rolls it back too, and it lasts only 90 days. *Consequence:* after a restore,
nothing in the system can name the accounts deleted after that backup. The promise is unmeetable from the day bump A
publishes. *Fix in text:* either add a code item that keeps a deletion log outside the database (for example the
account function writing a hashed id and date to separate storage), sized and placed before bump A, or soften the
sentence until that exists.

**I5. The liability clauses contradict the terms' own promises.** (§4 *Limitation of liability, and the rest*.)
- "We disclaim all warranties, **express** or implied" sits in the same document as "30-day money-back
  **guarantee**", the refund cases, the shutdown refunds and the notice promises. *Fix in text:* "except the promises
  these terms make".
- "Our total liability … for anything arising out of these terms is limited to the amount you paid … in the 12 months"
  can be read to cap promised refunds. The under-18 refund ("we will refund what you paid") can exceed 12 months of
  payments. *Fix in text:* add "this limit does not reduce a refund these terms promise".
- Q2 says the disclaimer is "conspicuous … The draft does both". Only the label "No warranties." is bold. The
  disclaimer and the cap sentences are not. *Fix in text*, or add it to the lawyer list.

Otherwise the clauses match U1: "as is" with merchantability and fitness named; missed or late assignments and grades
excluded by name; a 12-month paid cap with no floor ($0 in a trial); a "fullest extent permitted by law" carve-out
plus the state savings sentence; no indemnity.

**I6. Between bump A and bump B, the two pages disagree.** (§5, D1.) Bump A publishes "KNOWLU LLC, a Kentucky
limited liability company". Until the billing lane, the live terms still say "between you and Knowlu", Alabama law
and Alabama courts (`site/terms.html:13, 65-66`). The terms also still say "We do not refund a month you have already
used" (`:39`), while the ported homepage promises "30-day money-back guarantee on the yearly plan"
(`site-dist/index.html`), on a site where no yearly plan is sold. *Decision for Quinn.* Recommendation: publish an
interim terms version with bump A carrying only the [TRUE NOW] clauses (entity, Kentucky, authorization, accuracy,
liability), which needs C1 first. Or accept the gap in writing and put it on the lawyer list.

**I7. "Made and run by KNOWLU LLC" is marked [TRUE NOW] on the filing date alone.** (§3 *Who we are*; D5.) The note
does not check whether the Stripe, Supabase, Resend, OpenRouter, Cloudflare, AWS and Google accounts and the domain
are held by the LLC. *Consequence:* the page may name a party that is not the one contracting with those processors.
*Decision for Quinn / lawyer list.*

**I8. §7 misses test pins.** See *Pinned tests* below. Most serious:
- `static_assets.rs:3474-3482` `the_panels_changes_name_gmail_and_amazon` fails at every bump unless the new lines
  say "Gmail" and "Amazon".
- Test 9 contradicts the note's own text: *Automatic renewal* says "There is no summer pause.", outside the
  "If you subscribed before" paragraph.

**I9. A false claim in the terms: Knowlu signs in "only during its scheduled runs".** (§4 *You confirm that you are
allowed to*.) Coursework sign-ins also happen at onboarding (`coursework-discover`, `app/src/onboarding.rs:71-133`)
and on the tray's "Run now" (`app/src/tray.rs:23`; `scheduler.rs:186, 729`). *Fix in text:* "when it runs: on its
schedule, when you connect a site, or when you ask it to".

**I10. The O1 third kept sign-in was dropped.** The audit §3 raised the onboarding draft's shared school profile
(`docs/specs/2026-10-23-onboarding-redesign-draft.md:91-92`), which would be a third kept sign-in. The note keeps
"two kept sign-ins" in pinned text (`site.rs:226-227`) and says nothing about it. *Fix in text:* put it under bump C,
or under O1's own bump, with the pin that has to move.

**I11. "One commit" against three agents.** (§5 "Every bump is one commit" vs "Who runs each bump": contract-engineer,
console-ui, test-writer.) The version tie (`static_assets.rs:1677-1686`, and `account.rs:21-23` "in the same commit")
fails at any commit in between. *Fix in text:* one agent (contract-engineer) commits the constant, the page and the
key together, or the PR is squash-merged and that is stated.

**I12. Entity internals in a public repository.** The note (header U4, D5, §8, §9 Q1, "Outside the pages") cites a
draft operating agreement, its clause 13.3, and insurance plans. The repository is public. *Decision for Quinn:* keep
those references in the note, or move them to a private document and cite "Quinn's ruling U4" only. Recommendation:
move them.

**I13. Lawyer timing does not match VISION.** §5 and §8 put the lawyer read "before the first paid sale".
`VISION.md:231-233` says a lawyer reads the page "before the first pilot student", and pilot students do not pay.
*Fix in text:* "before the first pilot student or the first paid sale, whichever comes first".

**I14. The "Manage subscription" pin may not survive v0.4.0.** The terms tell the student to "click the gear, and
choose **Manage subscription**". The label pin (`static_assets.rs:1081-1083`) sits inside
`the_settings_panel_has_its_rows_and_one_way_in`, which v0.4.0 T16 re-homes to assert "only T16's own sections". The
billing account row is now "Membership" (v0.4.0 design `:303`). *Fix in text:* confirm with the v0.4.0 lane that the
label and the path survive, and write the terms' path against the new UI.

## Minor

- **m1. The judgment-record key list is incomplete.** (§3 *Judgments, and the rules learned from them*.) `fields` also
  keeps `created_by` for tasks and `source` for events (`judge_rules.ts:46-47, 56, 70, 76`;
  `migrations/20260911000100_judgment_service.sql:61-63`). Fix in text.
- **m2. The promotion window is verifiable.** It is 60 days for judgments and corrections
  (`20260911000500_rule_promotion_fix.sql:165, 191`; `20261002000200_mail.sql:282, 298`). D12 and C4 call it
  `unverified`. Fix in text.
- **m3. C15's sentence C says a model is sent "only what kind of assignment it is, its course and its due date".**
  The request also carries the preferences, the grade weights, the known courses, `source_uid` and `created_by`
  (`cloudmodel.rs:356-372`). Drop "only", or list them. C15 also leaves vendor items with no `title_prefix`, so
  tier-2 rules stop applying to them. Add a test, or state that it is accepted.
- **m4. Audit items dropped without a reason:**
  - Mail batches and canary records, and their 30-day retention (`mail-process/sweep.ts:10-14`), are missing from the
    closed list and from *How long*.
  - The site FAQ says "goes to Knowlu's servers to plan your day" (site-dist `index.html`), which contradicts the
    terms' "The ranking runs on your computer".
  - Gap 4's CASA verification.
  - Gap 7: the security duty under Kentucky law, now that the entity is in Kentucky.
  - Gap 12 has no appendix row.
  - Gap 14's pilot or early-access status.
  - P9's in-app disclosure before the card.
  - The appendix says regions and data processing agreements are "left to C21", but C21 does not list them.
- **m5. "Emails we send" says "we" where C8 and C9 may end up as Link's emails.** If C5 makes Link the sender, the
  wording has to change with it.
- **m6. D4 and D2 contradict their own text.** D4's heading says FERPA appears in the terms; its body says the terms
  carry no acronym either. D2 says "four places" and lists three, and v0.4.0 drops the index's sentence pin (A5).
- **m7. The panel copy still says "AI calls".** `PRIVACY_CHANGES` (`console.js:3129`) has "AI calls", and the new
  no-"AI" test 2 covers only the pages.
- **m8. C16 has no guard.** "Must not send the answers from a build below C" has no check behind it. Name one like
  `scripts/ci/privacy-guard.ps1`.
- **m9. VISION's price line (`:153-156`) contradicts terms B until Amendment 2026-11-10 merges to `main`.** Order the
  amendment's merge before B.
- **m10. One authorization line is inaccurate.** The school-sites line ("sign in as you, with the login … you gave
  it") does not fit the school sign-in, which the student types on the school's own page (`privacy.html:69`).
- **m11. The consent-records bullet has no "A" variant.** If C3 has not landed, bump A drops today's "at least three
  years" with nothing in its place. Fix in text.

## Truth check: [TRUE NOW] claims verified against code

Each was verified at the cited place:
- Log retention: `scheduler.rs:30-35`, 80 files, about 20 runs, `quit-*` records never pruned.
- Server-side parse that keeps nothing: `coursework.rs:1036-1039`; `ingest-coursework/handler.ts:1-9, 197`, whose
  only log line is the error class name.
- Tier 1 never sets importance: `judge.rs:441-458`.
- The title and body go to the model: `cloudmodel.rs:356-366`; `judge_prompts.ts:247`.
- `title_prefix`: `judge_rules.ts:13-17, 62-78`.
- Rules are per account only, and global rules are never promoted: `rule_promotion.sql:6-8, 129-130`.
- Judgments and rules cascade on deletion: `judgment_service.sql:69, 111`.
- Consents keep `subject_hash`, version, price and date, with `account_id` and `ip` nulled: `account/index.ts:108-114`;
  `accounts.sql:36-45`.
- The tombstone is an unsalted SHA-256: `crypto.ts:31-33`.
- The sources key is used by the account function too: `account/index.ts:261-263`.
- Resend sends the auth mail, the billing mail and the delivery checks: `config.toml:81-90`; `_shared/canary_send.ts:1-15`.
- The three addresses go through Cloudflare to a Google-hosted mailbox: `HANDOFF.md:56`.
- The updater endpoint is on `knowlu.com` (Cloudflare): `tauri.conf.json:26`. Updates are offered, not forced:
  `updates.rs:1-5`.
- Autostart is a Settings toggle: `commands.rs:466-498`.
- Google scopes are read-only: `_shared/google_scopes.ts:5-6`.
- Checkout today: `billing-checkout/handler.ts:51-72`, with no Managed Payments parameter.
- Later terms versions are never restamped: `account/handler.ts:223-247`.

False or overstated: I4 (the restore promise), I7 (the LLC as operator), I9 ("only during its scheduled runs"),
m1 (key list), m3 ("only"). The [TRUE NOW · unverified, C21] items are correctly held back until C21.

## Dependencies

Every [NEEDS CODE] names an item that would make it true, except the restore rule (I4), which C14 cannot make true as
a docs item. Every item has a size. The contract-list items are marked: C1 and C6 (`app/src/account.rs`). Money items
are marked Contract workflow. "Launch" for C19 and C20 is a milestone, not a release; that is acceptable because the
page already discloses the export gap. C13 at M is light for a closure path, a pro-rata refund path through Link and
a bulk shutdown. Consider M–L.

## Pinned tests these page changes break or depend on

**Bump A (privacy page):**
- `engine/tests/site.rs:205` "AI calls". This one is listed.
- `app/tests/static_assets.rs` (today's file; the pins re-home under v0.4.0, see K1):
  - `:1677-1686`, the version tie. Passes only if the constant moves in the same commit.
  - `:3417-3428`, `:3431-3443` and `:3459-3472`: a new single key, and no typed year.
  - **`:3474-3482`, the panel must name "Gmail" and "Amazon". Not listed.**
- `site.rs:186-199`, Last changed no earlier than the version. Listed.
- `site.rs:32-33`, the "guarantee" ban on the privacy page. Holds: the new privacy text has no "guarantee".
- Checked and not broken: `app/tests/grades.rs:183, 256` (`PRIVACY_BUMP_1` passes for any later version), and
  `app/tests/account.rs`, which reads the constants.
- Every other `site.rs` pin keeps its sentence in the §3 text.

**Terms and billing (B):**
- No test reads `terms.html` today, so C18's new test is the first.
- `static_assets.rs:1081-1083`, the "Manage subscription" label (I14).
- Code side, broken by C5 and C6 rather than by the page:
  - `billing-checkout/handler_test.ts:21` (trial "7") and `:31-32` (`automatic_tax`, `customer_update[address]`).
  - `billing-jobs/handler_test.ts:150` ("$9.99" in the resume notice). Its `:153` "Manage subscription" pin stays.

**Note test 9** fails on the note's own *Automatic renewal* sentence (I8).

**v0.4.0 acceptance check:** T4's empty `git diff` on the legal pages (K1).

## Lawyer list: additions

These come on top of §8:
- Whether the express-warranty disclaimer squares with the money-back guarantee, and whether the cap limits refunds
  the terms promise (I5).
- Whether the LLC is the contracting party with the processors (I7).
- The interim period in which the privacy page names a Kentucky LLC and the terms name Alabama (I6).
- The waitlist record (I3).
- Pell, if it is kept (K2).
