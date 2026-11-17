FAIL

# Legal pages rewrite: spec review, round 2 (final)

Reviewer (Opus, high), 2026-11-17. Spec mode, round 2 of 2. Document: `docs/specs/2026-11-17-legal-pages-rewrite.md`,
revision 2, in the `v040-ui` worktree. Code read at `6dd9a7a4`; the code is identical to `origin/main` `84933416`.
Not legal advice. Quinn has already ruled on Pell (U6) and on the private references (U7), and Q1–Q6 are Quinn's, so
none of those count against the note.

**Verdict: FAIL, on five Important findings that are all small text fixes.** Of r1's findings, 23 are closed and 3 are
closed only in part. The revision adds two problems in the publication order against the v0.4.0 plan, two in the
interim-terms option, and four Minor ones. There is no third round. The controller can apply the "fix in text" items
below without another review. The rest is under *For Quinn*.

## 1. The r1 findings

| r1 | Status | Evidence in r2 |
|---|---|---|
| K1 bump A vs v0.4.0 | **Partly closed** | Bump A is now its own PR after T4, T20, T21 and T23, and the cites are re-homed (§2, §5, §7). Open: where and when it merges (N1), and C17 and C29 being placed inside a final plan (N2) |
| K2 Pell | Closed | U6, D14, C26; no Pell text anywhere; test 9 bans "Pell" |
| I1 rulings | Closed | §0 quotes R1–R12 and U1–U7 |
| I2 B and C in one release | Closed | One bump B in v0.5.0; "one privacy bump per release" stated in §5 |
| I3 waitlist | **Partly closed** | Ordered in §5, purpose added, C25 added. Open: what C25 does to the waitlist lane (N5) |
| I4 restore promise | Closed | D16 drops the promise; C27 is at Launch; test 4 bans "we delete again every account" |
| I5 warranty and cap | Closed | "Other than the promises these terms make"; "This limit does not reduce a refund these terms promise"; the operative sentences are bold; lawyer item 12 |
| I6 interim mismatch | Moved to Q3 (Quinn's) | But the interim text it offers has defects (N3, N4) |
| I7 LLC as operator | Closed | "Operates" (D5), Q6, lawyer item 13 |
| I8 pins | Closed | Gmail/Amazon panel pin in §2, §5 and test 6; test 9 allows "There is no summer pause." |
| I9 "only during scheduled runs" | Closed | The new wording matches `onboarding.rs:71-133` and `tray.rs:23` |
| I10 third kept sign-in | Closed | D15 and the O1 row in §5 |
| I11 one commit | Closed | D17: one squash-merged PR, and `contract-engineer` makes the commit |
| I12 private references | Closed | No operating agreement, clause, registration number or insurance remains (grep) |
| I13 lawyer timing | Closed | §5 and §8 heading |
| I14 cancel label | **Partly closed** | C29 keeps the label, but places the work in v0.4.0's T16 (N2) |
| m1–m11 | Closed | Checked against the code: `created_by` and `source` in the key list (`judge_rules.ts:46-47, 56`); the 60-day window (`rule_promotion_fix.sql:165, 191`); C15 lists what still travels; the batches and canaries entry (`sweep.ts:10-14`); the "from Link" email lead; D4 and D2 reworded; the no-"AI" test covers the panel lines; the C16 guard follows `privacy-guard.ps1`'s D18 pattern; C26 for VISION; the school sign-in wording (`privacy.html:69`); the consent bullet's A variant. One leftover: D8 (n2) |

## 2. New problems

### Important

**N1. Bump A's merge point collides with v0.4.0's review and tag gate.** (§5 row "Privacy A": "cut after v0.4.0's T4,
T20, T21 and T23 have merged to `feat/v0.4.0-ui`, and merged before the v0.4.0 tag or on the next tag".)
*Conflicts with:* `docs/plans/2026-11-17-v0.4.0-new-ui-plan.md:110` ("the pre-tag gate runs on the merge SHA") and
`:191`, where T26 (the whole-branch review) and T27 (PR, proof, merge, gate) come after T23.

The sentence does not say which branch bump A targets. Consequences:
- Merged into the feature branch after T26, bump A goes unreviewed by the whole-branch review.
- Merged to `main` after T27's merge, it changes the SHA that the pre-tag gate and the live proof ran on.
- Either way it is a task the v0.4.0 ledger does not have.

*Fix in text.* Pick one:
- (a) Into `feat/v0.4.0-ui` after T23a and before T26, recorded as a plan amendment, so that T26 reviews it and
  proof 9's re-consent step runs at bump A's version.
- (b) On `main` after the v0.4.0 tag, for the next tag.

Recommendation: (b). It keeps a frozen plan frozen. Nothing in bump A is needed by v0.4.0.

**N2. C17 and C29 are put inside v0.4.0 tasks that the final plan does not list.** (§6 C17 "folded into v0.4.0 T21";
C29 "v0.4.0 (T16) … confirm this with the v0.4.0 lane before T16 merges"; test 15.) T21 (`v0.4.0-new-ui-plan.md:419-422`)
has no Google-line change, and T16 (`:396-402`) has no label pin.

*Consequence:* the implementers of T16 and T21 work from their task blocks and will not do either item, so the note's
cancel path and its Google copy depend on work nobody was given.

*Fix in text:* either name both as amendments to the v0.4.0 plan, or move them out of it. Move C29 to bump B (the
"Manage subscription" path publishes only in v0.5.0, and the interim terms say only "in Settings"), and move C17 into
bump A's PR.

**N3. The interim terms rest on an unverified premise and an unnamed write path.** (§4 "Interim terms" →
*Changes to these terms*; §5 row A, "Today that is one account"; Q3, "Nobody but Quinn has an account today";
appendix row "by hand to the one account".)
- r1 Q3 said the accounts were "a handful of testers and founder profiles". HANDOFF's L8 (testers, comp grants on
  prod) and L10 (friends' release) make further accounts likely. The note gives no evidence for the change.
- "The main session records it on Quinn's word" has no endpoint behind it. `recordLaterPrivacy` restamps only privacy
  (`account/handler.ts:223-247`), so recording a later `tos` version means a hand insert into production `consents`.
  That is a production write, and the note does not specify it: the row shape, `subject_hash` and Quinn's go.

*Consequence:* if any tester account exists, the interim terms either leave it on 2026-09-10 with no way to re-accept,
or require hand-written production rows per person. Q3's recommendation would then be wrong.

*Fix in text:* the main session counts production accounts (read-only) before Q3 is put. Spell out the hand-recording
step as a production action on Quinn's word. The guard "C1 must land before any second account" should read "before
any account other than the one that re-accepted by hand needs the interim terms". New sign-ups record `TOS_VERSION` at
sign-up and do not need C1.

**N4. The interim refunds point at something the interim terms leave out.** (§4 "Interim terms" → *Refunds*: "The full
text's 4-week cases (a)–(c) follow, with '4-week' read as 'any shorter period'".) Case (c) refunds a first charge "if we
had not sent you the reminder described above". The interim leaves out *Emails about your subscription*, and no trial
reminder exists until C9 (v0.5.0).

*Consequence:* the reference points at nothing. On its plain reading, every first charge after a trial is refundable
within 30 days, for as long as the interim terms stand.

*Fix in text:* drop (c) from the interim, or restate it as "a first charge after a trial, if you write within 30 days"
and accept that cost knowingly (see *For Quinn*). Also replace "4-week … read as any shorter period" with the actual
words, because published text cannot carry a reading instruction.

**N5. C25 changes another lane's signed plan and its going-live order.** (§6 C25, owned by "the waitlist lane's deploy
owner … its W5"; §5 Waitlist row.) The waitlist design is signed (`waitlist-design.md:3-4, 125`). C25 adds a refusal to
its deploy mode. As a result, the waitlist's own privacy bump (a legal-page change) can no longer go out by the
site-only deploy that W-D3 uses to ship the form without an app release.

*Consequence:* the waitlist form either launches before its privacy text is live, or waits for a release tag. Neither
appears in either document.

*Fix in text:* state that the waitlist's privacy bump rides a release tag under C25, and that this changes W-D3's
order. Add C25 to the waitlist plan as an amendment rather than assigning it from here.

### Minor

- **n1. The forwarding-batches entry undersells what a batch row holds.** A batch also keeps its phase, a "since"
  date, a keyed nonce and the provider's receipt ids (`20261002000200_mail.sql:84-97`;
  `20261020000400_mail_follow_ups.sql:31`). "Neither holds any message text" is true. Add "and the mail service's ids
  for the messages it counted" (the note already flags these columns as unverified).
- **n2. D8 still says the ported homepage line "will fail `site.rs:32-33` … C18 handles that".** That contradicts C18
  ("nothing is owed here") and §2 (T4 lifts the index ban).
- **n3. The interim terms drop the summer-pause paragraph and its resume-notice promise (`site/terms.html:27-29`).**
  `billing-jobs` still pauses monthly subscriptions June to August (`handler.ts:125-161`), so an existing monthly
  subscriber's pause would appear in no published terms. Carry that paragraph into the interim for monthly plans.
- **n4. Two waitlist promises are manual work nobody owns.** *Mail you send us* promises to keep a waitlist address
  "for nothing else" and until "you get your invite or ask to come off the list". The waitlist addresses are emails in
  the support mailbox, so both promises mean deleting by hand. Name the owner in C14's mail rule.

## 3. Checked and holding in the revision

- **The merged bump B** is consistent. Terms, the onboarding answers, C15, the Link variants and the O1 fold all ride
  v0.5.0. There is one key per release. C1 comes before the terms bump, and C16's guard sits in the D18 pattern.
- **C26** is docs-only, and correctly placed before the billing plan.
- **C27** at Launch, with its promise removed from the page, is consistent with D16.
- **C28** is a site-only wording fix after T4.
- **New [TRUE NOW] sentences, checked against the code:**
  - the judgment key list: `judge_rules.ts:46-77`; the event `source` is the feed name, `judge.rs:222`;
  - the forwarding batches and checks with their 30-day retention: `sweep.ts:12-14`; `mail.sql:84-110`;
  - the new sign-in wording: `onboarding.rs:71-133`, `tray.rs:23`;
  - the early-access sentence;
  - the CASA sentence (audit gap 4 citation);
  - the waitlist purpose, against today's `mailto:hello@` (`site-dist/js/site.js:120`).
- **The liability clauses** match U1 and are now internally consistent: the promises carve-out, the refund carve-out,
  bold operative sentences, the $0 trial cap, the savings sentence, and no indemnity.
- **T4's empty-diff check** is satisfied, because bump A is cut after T4 merges (§5 order constraints).

## For Quinn

These are open after round 2. Each is a decision, with a recommendation.

1. **Where bump A merges (N1).** Recommendation: on `main` after the v0.4.0 tag, shipping on the next tag. That leaves
   v0.4.0's frozen plan, review and gate untouched.
2. **The interim terms, refund case (c) (N4).** Recommendation: leave (c) out of the interim terms. With no trial
   reminder built, it would refund every first charge on request.
3. **Q3's premise (N3).** Recommendation: answer Q3 only after the main session has counted production accounts. If
   anyone besides you has one, skip the interim terms and accept the A-to-B gap with lawyer item 14, rather than hand-write
   consent rows for each person.
4. **C25 and the waitlist launch (N5).** Recommendation: the waitlist's privacy bump rides a release tag, and the form
   stays hidden until that tag deploys. Amend the waitlist plan to say so.
