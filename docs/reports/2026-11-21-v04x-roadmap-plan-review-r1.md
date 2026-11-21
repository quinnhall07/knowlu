FAIL

# Review r1: roadmap v0.4.1 to v0.5.0 (plan mode)

**Document:** `docs/plans/2026-11-21-v04x-to-v050-roadmap.md`, draft r1, on `docs/v04x-roadmap` at `515d13de`.
**Reviewer:** `reviewer`, plan mode, round 1. **Date:** 2026-11-21.
**Inputs read in full:** the plan; the run ledger (`progress.md`); the inventory (all six slices and section 0);
`brief-calendar-write-path.md`, `dossier-gradescope-edstem.md`, `dossier-cengage-stem-homework.md`,
`dossier-pearson-mcgrawhill.md`, `dossier-lms-brightspace-canvas.md`, `salvage-map-o3-long-onboarding.md`,
`term-rollover.md`; on the pricing branch, the v0.4.1 note's sections 7 and 8, the plan's task table, and the
uncommitted managed-payments facts report.

**Verdict: FAIL. 2 Blocker, 12 Important, 9 Minor.**

The order of releases, the frozen-reference handling, the privacy-bump plan and the v0.4.1 slot are mostly
sound. The plan fails on its first step. The Thanksgiving checks are days away, and as written they can't run
or would do harm. The site waves also rest on reachability claims that the two dossiers the planner didn't
read contradict. v0.4.1 can go ahead while these are fixed; nothing below changes its slot.

---

## Blocker

### B1. Step 0, the Thanksgiving checks: "It needs nothing built first" is false. As written, the checks run on production and leave friends' data in Knowlu's cloud

**Where:** §1 item 0 (line 74); §5.2 "What a check is / What it needs built first / What it must not do"
(lines 346-357); §5.5 (line 450); §6 last table (line 1113).

**What is wrong.**
- **Which backend.** The released v0.4.0 talks to production by default (`app/src/account.rs:18`). It reaches
  staging only if `KNOWLU_API_BASE` is set (`account.rs:43`). The plan names neither. Constraint 12 (line 247)
  says live proofs run against staging and production only on Quinn's word.
- **On production**, each friend becomes an account that is not Quinn's, before v0.4.1's T0. That does four
  things:
  - It brings C1 (terms re-acceptance, T5b) into v0.4.1, which changes the slot this plan says it does not
    re-plan.
  - It runs into Quinn's 2026-10-18 ruling "roll it before friends" (`HANDOFF.md:616`). §5.5 puts the key rolls
    at "now / the first account that is not Quinn's" but never makes them a precondition of the checks.
  - It runs into the sign-in email limit. That limit is still the default of about two codes an hour (H-06;
    `HANDOFF.md:618`), so a sitting with several friends stalls at sign-in.
  - It needs either a tester comp grant for each friend, which is a production write, or a checkout. Without
    one, the engine gates `coursework`, `ingest` and `grades` after the grace period, and "one slot runs"
    shows nothing.
- **On staging**, the staging prerequisites apply instead. Staging's functions predate the testers migration
  (H-02: `entitlement` v15). Staging still has the default email limit. Each friend needs a staging comp.
- **What a check can see.** Grades, and so the partial-score observation behind Q6, exist only where a curated
  Blackboard row exists. `app/src/scaffold.rs` has two curated rows, and `grades.rs:57-65` refuses every other
  row. At a friend's non-curated school, "do its scores appear as grades" and "is anything marked done that is
  not finished" can only come back empty. So the claim "Knowlu already reads the first two [routes] at a
  Blackboard school" (line 341) holds for one school only.
- **Cleanup.** "What it must not do" covers the repository and the local scratch profile only. The friend's
  account in the cloud keeps their synced vault copy (real classes and tasks), their feed URL, consent rows
  and telemetry. Removing the scratch profile deletes none of these.

**Failure scenario.** On the first evening of the break, the main session opens the released app. It points
at production. The second friend can't get a sign-in code. The first friend is stopped by the entitlement
gate. v0.4.1's T0 later finds accounts that are not Quinn's and has to grow T5b. Friends' real coursework sits
in the production database with nothing scheduled to delete it. The break can't be repeated, so the plan's
whole site ordering waits until spring.

**Fix.** Add a "before the break" checklist to §5.2 and make it a precondition in §5.5:
1. Choose staging: the released build with `KNOWLU_API_BASE` set to staging. Redeploy staging's functions
   from `main` (H-02, a main-session act), raise staging's email limit (H-06, Quinn's dashboard), and grant
   each friend a staging comp. If Quinn instead rules production, the key rolls (H-04) and H-06 come first,
   and T0's premise check is told to expect testers.
2. For each friend at a Blackboard school, add a curated campus row (ST-15, a Small change in a dev build) if
   the grades route is to be observed. Otherwise say plainly that the check observes the feed only.
3. End each sitting with Delete my data on the friend's account (or a scripted purge), then remove the
   scratch profile.
4. Take each site's check protocol from the dossiers' "What a real account must confirm" lists (B2).

### B2. The site waves rest on reachability claims that the unread dossiers contradict. Two of the twelve have no route in any release and no decision on Quinn's list

**Where:** §5.2 table and "What the evidence says plainly" (lines 359-398); §5.10 (lines 591-599); §5.14
(lines 685-699); §5.19 (lines 846-859); §2.2's SA-10 row (line 179); ledger slice E (lines 1061-1070).

**What is wrong** (per `dossier-cengage-stem-homework.md`, `dossier-gradescope-edstem.md`,
`dossier-lms-brightspace-canvas.md`):
- **Edfinity (wave 1, "the LMS feed").** The dossier rates it "probably not covered". Edfinity tells
  instructors to keep dates in Edfinity, and students reach it only through school single sign-on, with no
  password. So the only route is a kept school-SSO session reader. That reader needs a ruling that extends
  the cloud design's ruling 12 beyond Blackboard grades (`cloud-design.md:677-700`) and a privacy bump. The
  plan parks it at 0.5.x (SA-10, line 179). Edfinity's terms also say "you will not access the Site through
  automated or non-human means"; the plan's table (line 363) says "Not found".
- **Smartwork (wave 1, "the LMS sets its due dates").** Norton advises against LMS due dates, and the
  student's own list (Grades Accepted Until) is the due date. A reader is possible through the Norton email
  and password. The plan's line 375, "cannot be reached by a sign-in of their own", is wrong for Smartwork.
- **WebAssign and MindTap.** The dossier finds that both sync due dates and grades to Blackboard, and ranks
  WebAssign the cheapest of all. Aktiv puts a due date on its LMS columns by default. So line 386 ("no second
  route"; "the largest risk to all twelve") has it backwards: these are wave-1 candidates at Blackboard
  schools, and Edfinity and Smartwork are the largest risk.
- **Ed.** Discussion has no due dates. Announcements arrive by mail from `edstem.org`. Whether Lessons carry
  dates is unconfirmed. Two-factor sign-in breaks a stored password. The dossier recommends Ed as a mail
  source, not a reader. "Its work shows up" (ruling 6) has no defined meaning for a site with no dated work,
  and nothing on the list of 17 asks Quinn what it means.
- **Gradescope.** LMS date sync works only where the school turned it on, and extensions and late due dates
  never sync. A direct read needs a third kept sign-in (SD-09: a Quinn ruling and a privacy bump), which
  neither v0.4.9 nor §5.1 plans.
- **Brightspace.** Some schools give one feed per course. The `sources` key is `(account_id, kind)`, so more
  than one feed is a migration (LMS dossier SD-14). D2L's EULA §6(e) bars automated extraction. The plan's
  line 361 says only that D2L's terms "defer to the school's".
- **The check protocol** (§5.2, line 346) records four observations. The dossiers list about thirty concrete
  confirmations that only a live account gives, among them: Gradescope's late date absent from the feed, the
  Smartwork GAU page, ALEKS holding grades until the due date, Revel's sync type, Pearson extensions, and each
  vendor's sender, subject and List-Unsubscribe. None is in the protocol.

**Failure scenario.** Ruling 2 says v0.5.0 waits for all twelve. Wave 1 ships v0.4.5 expecting Edfinity and
Smartwork in the feed. The checks show they aren't there. No release, ruling or decision exists to reach
them, and v0.5.0 is stuck, with the cause found only after the break. Meanwhile waves 2 and 3 build sign-in
readers for WebAssign and Cengage that the LMS already carries.

**Fix.**
- Rebuild §5.2's table from the dossiers: WebAssign, MindTap and Aktiv go to wave 1 (proof of the LMS route).
  Edfinity goes to a kept-SSO reader. Smartwork goes to a Norton sign-in reader or the LMS route, whichever
  the check shows. Ed is defined as a mail source unless Lessons show dates.
- Add two decisions to §7:
  - Q-new-1: may a kept school-SSO session read a homework site (extending ruling 12; SA-10 out of 0.5.x)?
  - Q-new-2: what "working" means for Ed.
- Correct the terms column for Edfinity and D2L.
- Name SD-14's possible migration under v0.4.5.
- Paste each dossier's confirmation list into the check protocol before the break.

---

## Important

### I1. The Mac spike can't answer its main question on an unsigned debug build, so v0.4.11 waits on Apple enrollment without the plan saying so

**Where:** §5.3 "The spike" (line 417); §4 v0.4.11 depends on "the Mac spike's webview answers" (line 275);
§5.5 (lines 448, 452).

**What is wrong.** The spike's first question is whether the Keychain lets two binaries share an item (M-05).
The answer depends on code-signing identity. Slice F's planner note 4 recommends "a spike on a signed build".
An ad-hoc-signed debug artifact gets a different identity on every build, so a "yes" or "no" from it settles
nothing. A signed build needs the Developer ID certificate, which needs the company's enrollment. Enrolling
as an organization needs a D-U-N-S number, which has its own lead time. The notarization credentials (API
key, certificate) must also become GitHub secrets. §5.5 lists only "enroll".

**Failure scenario.** The spike runs on a debug build and answers "no prompt". v0.4.15's design rests on that,
and the signed build then prompts on every engine read. Or the app track waits at v0.4.11's spec for the Mac
spike, which waits for Apple.

**Fix.** Make the Keychain part of the spike run on a signed build. Split the spike so v0.4.11's webview
questions can run on an unsigned build. Add D-U-N-S and "notarization secrets into GitHub" to §5.5.

### I2. The sales and legal gate is treated as decided while Q10 is open, and Q10 itself is incomplete

**Where:** ruling 12's gloss (lines 57-60); §1 (line 91, "The lawyer holds nothing"); §2.3 (line 187); §5.5
(lines 449, 457); ledger LW and H-07 rows (lines 1003, 1027); Q10 (lines 1281-1288).

**What is wrong.**
- Quinn's words, as the run ledger quotes them at 08:14, are "ready to sell; i'm willing to take the risk. we
  won't be pushing for people to buy just yet, but it should be working." "The lawyer, tax and signing run
  alongside and never hold a release" is the plan's gloss. It is then repeated as settled in at least six
  places, while Q10 asks whether it is so.
- Amendment 2026-10-16b also says outside buyers may buy only "from the pilot's start (spring 2027)"
  (`cloud-design.md:807-808`). That is a timing gate a pre-December v0.5.0 would break, and Q10 doesn't ask
  about it.
- `VISION.md`'s "Open decisions" row ("a lawyer before the first non-founder paid sign-up … Amendment
  2026-10-16b") would also need Quinn's amendment. Q10 doesn't name it.
- Q10's own advice ("finish the tax registration … because that one is a legal duty") is a legal claim the
  plan cannot make. The facts report says Managed Payments makes Link the merchant of record, which "files and
  remits" tax. Whether a state registration is still owed is a question for the lawyer or an accountant.

**Failure scenario.** The next session reads §1 and §5.5 as settled. It builds v0.5.0's checkout open to
strangers before spring 2027 and before Quinn has signed anything.

**Fix.** Make every "holds no release" line conditional on Q10. Extend Q10 to cover 10-16b's timing clause
(2) and the VISION row. Reword the tax advice as a question for the lawyer or accountant.

### I3. Missing decision: moving Stripe to the company before sales open

**Where:** §5.5 (line 457: "Stripe's account set-up (LW-13) … Hold no release"); ledger LW-13 (line 1004).

**What is wrong.** The v0.4.1 note's §8 item 4 recommends deferring a new Stripe account until after v0.4.1,
"but before 10-16b's gate opens sales; after that it means recreating every paying subscription". The facts
report (Q8) adds that a change of business type may require a new account. Ruling 12 opens buying at v0.5.0,
so this becomes a deadline. The plan instead says it holds no release.

**Failure scenario.** v0.5.0 sells under the current Stripe account, and the company move later forces every
paying subscription to be recreated.

**Fix.** Add to §7: "Move Stripe to the company before v0.5.0's tag?" Recommendation: yes, after asking Stripe
Support (facts report, Q5).

### I4. §10.3 makes the `CLAUDE.md` wording the main session's edit

**Where:** §10.3 (lines 1455-1458).

**What is wrong.** The v0.4.1 note's §8 item 13 requires "CLAUDE.md's Direction line, in your words", meaning
Quinn's. `CLAUDE.md` and `VISION.md` wording is Quinn's alone (the brief, check 2).

**Fix.** "Quinn words each line; the main session applies it in the PR that makes it stale."

### I5. Q7 doesn't name the signed texts that rulings 1 and 5 overturn

**Where:** Q7 (lines 1241-1248); constraint 7 (lines 230-234); §5.16 (lines 738-741, 753-755).

**What is wrong.**
- The cloud design is the authority. Its ruling 12 says "no password is kept, only the session" and "Knowlu
  never asks for campus SSO credentials" (`cloud-design.md:684-685, 698-700`; also `:256`). Main's tests encode
  the same rule (`app/tests/ui_wizard.rs:478-492`: "Knowlu never asks for a campus credential"). Q7's draft
  amendment names only `VISION.md`'s line and the 10-23 step table.
- The branch's RQ16 ("school password saved by default") is superseded by ruling 5 ("make it opt-in"). The
  salvaged `school_password` code (salvage map, units P1a and P1b) saves by default.
- The plan also says the two tests "pin the sentence 'nowhere in the app to type one'" (line 755). They pin
  the behaviour (exactly two password fields, no LMS credential field), not the sentence.

**Failure scenario.** The v0.4.11 spec is reviewed against §1 and fails, or passes on chat. The salvaged code
then ships with saving on by default.

**Fix.** List in Q7 the exact sentences it amends: `cloud-design.md:256, 684-685, 698-700`; `VISION.md:157`'s
onboarding line; RQ16 marked superseded by opt-in. In §5.16, say that the salvaged `school_password` is changed
to opt-in and that the two tests change with it.

### I6. Four releases have the wrong size, or carry a hidden trigger

- **v0.4.3** (line 267: "Lane"). T-04 writes a `status:` word that Q17 itself says "becomes vault format the
  day it is first written" (lines 1180-1181). That is a vault byte format, so Contract. Fix: Contract for T-04
  (`contract-engineer`, a one-page plan), as the plan already does for the M-03 rider.
- **v0.4.5** (line 269: "Lane"). A real Brightspace feed sample placed under `engine/tests/fixtures/**` is a
  contract-list edit (LMS dossier SD-01). Per-course feeds need a migration (SD-14). Fix: write inline test
  feeds, or go through `contract-engineer`; name the migration as a stop-rule-2 trigger.
- **v0.4.2** (lines 266, 504). It bundles B2 (privacy page and `account.rs`), C3 and C4 (two migrations and
  cloud jobs) and possibly C1 (terms re-acceptance: `account.rs`, the account handler, `reconsent.js`). C1 is
  terms, not privacy. That is two unrelated Contract pieces, against §4's own rule (line 290). Fix: if C1 is
  needed, give it its own release, or say plainly that v0.4.2 breaks the rule and why.
- **F-24 Open folder** (line 502, "Small"). The only ShellExecuteW opener is in `app/src/account.rs:646-669`,
  which is on the contract list (inventory F-24). Reusing or editing it makes F-24 Contract, and it then
  serialises with B2 on `account.rs`. Fix: write F-24 in a new module that never touches `account.rs`, or size
  it Contract.
- **Also:** v0.4.7's reason ("notes written by a new command") is no Contract trigger. Its size holds only
  because the one-off "No" adds a synced decline-marker key namespace (calendar brief, slice 3 and Q6). Say
  that instead.

### I7. The salvage map isn't applied: the `meets` edit already exists on the branch, and v0.4.11 lacks its own prerequisites

**Where:** §5.12 (lines 627-656); §5.16 (lines 747-749); ledger OB-01 (line 966).

**What is wrong.**
- Salvage slice 7 (F7e and F5d: the console may set `meets`, tested and green) is v0.4.7's F-04. It needs
  A19, which is signed only on the branch. v0.4.7 plans to build it fresh.
- v0.4.11 carries the kept-folder work in `lms_link` and `grades`. That work sits on slice 5 (the C8 link, the
  `read_hidden` seam, `lms_sign_in`). The branch's `lms_link.rs` also still has F8la as `todo!` (lines
  1124 and 1129 on the branch).
- Slice 2 (the bounded uid pin: fewer misattributed items, which a student can see) and slices 1 and 3 have
  no home and no deferral reason.
- F2h's `checkout_url_at` against v0.4.1 isn't mentioned.

**Failure scenario.** v0.4.7 builds `meets` editing a second time, and the v0.5.0 merge collides with it.
v0.4.11's carry doesn't compile, or arrives with red tests.

**Fix.** Make v0.4.7's spec start from slice 7 and re-sign A19. Put slice 5 and F8la in v0.4.11. Give slices
1, 2 and 3 a home: slice 2 as a Small in v0.4.3, or a named deferral.

### I8. The Gradescope and Expert TA proofs soften ruling 6, and the sign-in readers' privacy cost is missing

**Where:** §5.2 table (lines 366-367); §5.14 boundary (lines 687-688); §5.1 B2 (line 315).

**What is wrong.**
- Ruling 6 says each site is "proven with a real student account". A self-made instructor course, or a
  14-day trial, is not that. A real student's Gradescope runs through school sign-on, LMS links and a
  regional host. Creating a course to test automated reads also sits badly with Gradescope's terms ("means
  other than those provided or authorized").
- B2 describes sites by category, but the privacy page also lists kept sign-ins one by one ("both kept
  sign-ins", `site/privacy.html:30, 69`). A reader that keeps a session per site needs another bump in v0.4.9
  or v0.4.14, beyond the five planned.

**Fix.** Use a self-made course only to develop and fixture the reader. The proof needs a student in a live
course. Add "kept sign-ins described by category" to Q2's two wordings, or plan the bumps.

### I9. "Ready to sell" collides with Google's Testing mode, and no decision covers it

**Where:** §2.1 A (lines 117-118); §5.5 (line 456); ledger ST-17 and H-10 (Launch).

**What is wrong.** The live page says Gmail "works for at most 100 people, each named in advance as a test
user" (`site/privacy.html:103`). A stranger who buys at v0.5.0 can't connect Gmail until Quinn adds them by
hand. That is a per-user hand step, against success 6 and rule 1. If Gmail and Calendar share one consent
(O-09), Calendar may sit behind the same limit; which publishing mode the project is in is unverified. The
first-install note records the unverified-app warning. 10-16b accepts the 100-user cap, but not a hand step
for each buyer. `calendar.readonly` verification (X-4) is listed for "strangers" but not tied to v0.5.0.

**Fix.** Add to §7: either submit `calendar.readonly` before v0.5.0 and keep Gmail as the named-tester option
behind forwarding, or have the onboarding's Google step degrade to forwarding with the cost named. Tie X-4 to
v0.5.0's tag.

### I10. The plan isn't honest about the winter break: the proofs need live spring courses

**Where:** §1 (lines 94-100); §5.5 "A second sitting with the same friends" (line 451); §5.18 (lines 823-828);
§8.1 (line 1315).

**What is wrong.**
- Fall courses end in mid-December. Each later wave's proof needs "a student in a live course", and the new
  term's T-07 check needs published spring courses, so both wait until spring courses are live (mid-to-late
  January at most schools). The "same friends" will be in different courses by then.
- The plan says v0.5.0 can't ship before December, but not that ruling 2 (real-account proofs for all twelve)
  puts it in the spring term at the earliest.
- Part 1 of the new semester is placed "right after v0.4.5", although by the plan's own account it needs only
  v0.4.3 and v0.4.4. v0.4.5 is gated on the checks.

**Fix.** Say plainly that waves 2 and 3 and v0.5.0 are proven on spring courses. Move part 1 to "right after
v0.4.4" as the recommendation.

### I11. The public fidelity ledger is keyed to a private file and can't be audited from the repository

**Where:** §6 (lines 917-923); §9.1's "the `H-` rows" (line 1372).

**What is wrong.** "A row never restates the inventory's detail", and the inventory is git-ignored. A reader
of the repository, or a next session without the private folder, can't tell what H-23, SA-08 or BR-14 is,
and can't check HANDOFF's items against the "H- rows".

**Fix.** Add a few words naming each ID, for example "H-23 (P6 parity items)", in the ledger and in §9.1.

### I12. Missing decision: per-site readers on the device, or wait for the C5 relay

**Where:** §5.14 and §5.19.

**What is wrong.** The Cengage dossier asks Quinn whether new password readers should wait for C5. With the
relay, a vendor is "a cloud deploy plus one host allow-list row and a login panel". Without it, each vendor
is a Rust fetcher, a TypeScript parser and a contract-list edit. VISION puts the relay before Launch
(`VISION.md:236-237`; `HANDOFF.md:219`). Building eight device readers in 0.4.x risks doing them twice.

**Fix.** Add a decision with a recommendation. If the answer is "device readers now", record why.

---

## Minor

- **m1. Facts that are wrong.**
  - Line 203 and line 107 cite `v0.4.0-new-ui-design.md:92` and `:92-106`. The cut heading is at `:94` and
    the table runs `:96-107`.
  - Line 220 says `list` is emitted "only on `all` views" (`surface.rs:2376-2377`). Those lines say it rides
    the four horizon views. The point that holds is "not on the today view".
  - Line 1539 says the syllabus-terms note was "not found". It is `docs/notes/2026-10-20-simple-syllabus-terms-first-read.md`.
  - Lines 1447-1451 list the `handoff-1120` worktree, which the controller removed at 07:33.
  - Line 637 says the "same files". The zone split and the `--now` parse are in `rank` and `surface`, not in
    v0.4.7's commitment files.
  - Q3 omits `cloud-design.md:30`.
- **m2. Decision order.**
  - Q4 blocks v0.4.3 (line 267, lines 554-555) but is 14th.
  - Q7 blocks the new-term spec (ruling 13; T-05 reopens syllabus Q11(a)) but ranks after Q13 to Q16.
  - Q11 has no recommendation. Its "Blocks: nothing" ignores that recruiting has already started
    (`HANDOFF.md:479`) under a public "free forever" promise.
  - H-23 (P6 parity) and H-25 (stage questions, including A13 at the 20th payer, which ruling 12 makes
    reachable) are parked "until Quinn" but are not on the list.
- **m3. Text attributed to Quinn that Quinn didn't say.**
  - Ruling 10 adds "A mailbox is bought before the first marketing email" (line 54). §5.5 (line 458) then
    conflates that postal address with C14's business email mailbox.
  - Ruling 2 adds "v0.5.0 waits until each is proven".
  - Ruling 13 inserts "[plan]".
  - Mark each as the planner's reading.
- **m4. The v0.4.1 slot (not re-planned; small corrections).**
  - Line 485 says v0.4.2's fixes don't touch T4's files. F-24 lands in `pages/settings.js`. T4 also owns
    `ui_wizard.rs`, `ui_settings.rs` and three ui-fixture checks (v0.4.1 plan, line 233).
  - The facts report says T1 should send `managed_payments[enabled]=true` every time, so B1 most likely ships
    in v0.4.1. In §11.3 (line 1535), S1 settles that, not T0.
  - Adding the `20261023000100` check to T0 (line 478) stays read-only within ruling 15. That's fine as
    written: a recommendation.
- **m5. Internal contradiction.**
  - "Only Quinn has the app" (line 331) and "only Quinn has accounts" (line 974) contradict §9.2's "friends
    testing free, since 2026-10-18" (line 1387) and `HANDOFF.md:368`. Mark them `unverified` until T0.
  - §9.2's Pilot row moves the key rolls to "before the first pilot student". The 2026-10-18 ruling says
    before friends' accounts (`HANDOFF.md:616`).
- **m6. Public hygiene.** No absolute paths, project ids, addresses or student data found. Two personal
  details are better paraphrased: "Quinn has no Mac" (line 34), and the quote about meeting friends over the
  break "to get their logins" (lines 62-63). Say instead "proofs run on a borrowed Mac" and "friends' real
  accounts are available over the break".
- **m7. Q3's draft** (lines 1161-1171).
  - Say it replaces only the first sentence of `VISION.md:150`, so the phone clause and the 50 MB budget stay.
  - Recommend a value for the oldest macOS: 14+, because M-10's per-window stores need it.
  - Raise the size gate in the same commit as the universal build (slice F, item 5).
- **m8. Wording that could break an invariant.**
  - v0.4.6's "on the next read" (line 621): `surface` never writes. Say "on the next ingest or coursework run".
  - v0.4.8's placement refusals (over a hard commitment, before now) must be the engine's, not
    `commands.rs`'s.
  - Each new Tauri command must recount the `generate_handler!` lists and `docs/reference/app.md`'s counts.
- **m9. Housekeeping** (§10).
  - The `v041-pricing` worktree holds the uncommitted facts report that v0.4.1's checkpoint A reads. BR-01's
    merge should carry it, and the worktree must not be removed first.
  - `fix/date-comment-due` is "not patch-equal", so deleting it loses its own commits. Tag it or confirm
    first.

---

## What each unread supplement changes in the plan

- **`dossier-gradescope-edstem.md`:**
  - Gradescope: a direct read needs a third kept sign-in (ruling and bump; I8). LMS date sync is
    school-optional and never carries late dates. Its mail (`gradescope.com`) and receipt rows come first.
    SD-06's late date as a field is Contract; the dossier recommends the body only.
  - Ed: a mail source, not a reader (B2). Its API token is rejected, as Canvas tokens were.
  - It also finds a slot-order mismatch in `CLAUDE.md` (`scheduler.rs:505`: coursework → grades → ingest),
    which is outside this plan.
- **`dossier-cengage-stem-homework.md`:**
  - WebAssign, MindTap and Aktiv move to wave 1 at Blackboard schools.
  - Edfinity and Smartwork are not feed sites (B2). Edfinity's terms bar bots.
  - C5 relay decision (I12).
  - SD-18 confirms Q6 as a live risk for Smartwork and Edfinity.
- **`dossier-pearson-mcgrawhill.md`** (planner read only its opening):
  - Pearson's EULA §5(B) also bars third-party access through the student's account.
  - ALEKS can hold grades until the due date, which delays turned-in detection.
  - Revel's overall-score sync type can't join item by item.
  - Each is a line in wave 1's proof.
- **`dossier-lms-brightspace-canvas.md`** (opening only):
  - Per-course Brightspace feeds mean a migration (SD-14).
  - A feed fixture under `fixtures/**` is a contract-list edit.
  - D2L EULA §6(e).
  - Canvas's capture endpoint is probably `users/self/profile`.
  - All four bear on I6 and B2.
- **`salvage-map-o3-long-onboarding.md`** (opening only): I5 (RQ16) and I7 (the `meets` edit, slice 5,
  F8la, slices 1-3).
- **`brief-calendar-write-path.md`** (opening only):
  - The decline marker for a one-off "No" is the real Contract trigger for v0.4.7 (I6).
  - Q4 (Do today items as blocks) is unasked.
  - The note body text for a commitment the student made is a spec detail.
  - Otherwise the plan follows the brief's recommendations faithfully.
- **Managed-payments facts report:**
  - B1 likely ships in v0.4.1 (m4).
  - The tax framing in Q10 (I2).
  - The Stripe entity move (I3).
  - Link deleting a customer's data can strip `metadata.account_id` before a cancel event: a C12 case for the
    cloud track.
  - The sandbox sends no trial emails, so v0.5.0's test-mode proof can't show them.
- **`term-rollover.md`** (read in full by the planner): every T- ID has a home (below). T-04's "Contract if
  the word is format" became I6.

## Fidelity sample

All 233 IDs were checked against the inventory's ID lists. Slice counts and row counts match the claim
(A 35/19, B 30/15, C 51/38, D 43/29, E 28/21, F 25/10, T 21/15).
- **Spot-read for a real home:** F-01 to F-29, S-01 to S-06, O-04/06/07/12/23/30/31, OB-01 to OB-06, C1/C5/
  C7/C10/C11/C12/C13/C21/C25/C26, P-1 to P-7, LW-13/14, X-1 to X-5, BR-01 to BR-16, H-01 to H-27, SA-01 to
  SA-10, ST-01 to ST-18, M-01 to M-25, and every T- ID.
- **Missing:** none.
- **Double-booked:** none. The split homes (F-12, F-13, O-07, O-12, O-23, O-30, M-05, M-06, M-07, ST-03,
  ST-12, T-11) are each stated as two halves.
- **Parked without a usable reason or decision:**
  - H-23 and H-25 ("until Quinn", no Q; m2).
  - ST-10 and ST-11, whose stated reason is contradicted (B2).
  - SA-10, parked at 0.5.x though Edfinity needs it (B2).

## Path:line claims checked against `515d13de`

**Checked and correct (about 60):**
- `VISION.md` lines 45, 88-89, 119, 145-146, 150, 157, 190-198, 240 and 241.
- `site/index.html:91-102` and `:390-392`; `site/privacy.html:12`, `:21` and `:69`; `site/terms.html:16`.
- `app/tests/ui_account_privacy.rs:227-233` and `:235-237`.
- `app/src/account.rs:25` and `:1696`.
- `app/tests/ui_shell.rs:925` and `app/tests/mail_window.rs:66-77`.
- `engine/src/surface.rs:2232-2248` and `:2359-2368`.
- `engine/src/commitments.rs:27`, `:852-870` and `:867-870`.
- `app/src/commands.rs:149`.
- `app/src/main.rs:181` and `:270`: `store_credentials`, `capture_courses`, `open_lms_window` and
  `open_external` are in the wizard's list only.
- `engine/src/coursework.rs:1073` and `:1979`; `app/src/onboarding.rs:1001` and `:1028`;
  `app/src/scaffold.rs:714` and `:842`.
- `HANDOFF.md` lines 28-32, 38-39, 182, 194, 275-286, 288-294, 293, 365, 479, 484, 539, 575-576, 613, 617
  (including "$9.99"), 618, 620, 625 and 626.
- `cloud-design.md:444`, `:554`, `:806-812` and `:816-827`.
- `ci.yml:75`; `release.yml:30` and `:275`.
- `commitment-model-design.md:91` and `:137`.
- `phase2-plan.md:501-502`.
- `email-forwarding-design.md:2093`, with the quote exact.
- `onboarding-redesign-draft.md:142-164` and `:146`.
- `legal-pages-rewrite.md:774-780`, `:797-799` and `:814-815`.

**Wrong:** the five items in m1, plus line 755's "pin the sentence" (I5).

## HANDOFF redraft and housekeeping (checks 11)

§9.1 and §9.3 are accurate against the lines cited, and they're safe to apply after approval (they say so).
Two corrections:
- §9.2's MVP row is accurate (`HANDOFF.md:368`), but its Pilot row softens the "before friends" key roll
  (m5).
- The pointers to "`H-` rows" need a gloss (I11).

§10 is cautious: every removal is preceded by `git status`, and remote deletions wait for Quinn's word. The
caveats are m9 and the stale `handoff-1120` row.

## What I checked and found sound

- The two overriding rules hold. No release regenerates a Python reference, and every new read-model field is
  `all`-only or omitted when empty. The `closed_this_week` premise checks out (section 0, item 5). v0.4.8's
  fallback is a reasoned regeneration. Nothing names a school, person, account or machine, and sites go
  through a table.
- One privacy key per build. Five bumps in five different releases (B1 to B5), each in a release that is
  Contract anyway. The lawyer's extra bump is accounted for.
- `account.rs` is serialised across releases (constraint 4). T4's files land after v0.4.1's tag. Releases are
  CI-only on Quinn's word. Every Lane and Contract release ends in a live proof in a scratch profile.
- v0.4.1 is slotted, not redesigned. Decisions 8 and 14 are marked answered; the seven open items match the
  note's list.
- The engine invariants are respected: `rank` stays model-free and deterministic (v0.4.8), writes go journal
  first through `write`, `commitments` keeps its exit contract, and proposal cards stay local.
- The rulings are applied without re-arguing: twelve sites, opt-in entry with the window's own password saving
  as fallback, a friend's Mac, the company's Apple enrollment, email-only notices, the syllabus host read
  shipped regardless, new semester in scope, the read-only T0.
- Q12 is careful not to treat rulings 5 and 11 as a ruling on the publishers.

## Not verified

- Whether the Google OAuth project is in Testing or Production publishing mode, and whether sign-in, Calendar
  and Gmail share one project (I9).
- Whether the engine child process inherits `KNOWLU_API_BASE` from the released app, so that the whole slot
  runs against staging (B1).
- Whether staging's testers comp works once its functions are redeployed.
- Whether F-24 would actually call `account.rs`'s opener; that depends on the implementer.
- The D-U-N-S lead time today, and whether Apple's organization enrollment needs anything more from the
  company.
- No build or test was run. No trial merge was run. The salvage map's branch line numbers were taken from the
  map, not re-read on the branch.
