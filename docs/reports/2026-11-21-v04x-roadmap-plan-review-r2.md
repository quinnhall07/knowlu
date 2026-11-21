PASS WITH FIXES

# Review r2: roadmap v0.4.1 to v0.5.0 (plan mode, the last round)

**Document:** `docs/plans/2026-11-21-v04x-to-v050-roadmap.md`, draft r2, on `docs/v04x-roadmap` at `515d13de`.
**Reviewer:** `reviewer`, plan mode, round 2 of 2. **Date:** 2026-11-21.
**Read in full:** the plan; review r1; the run ledger (`progress.md`, every QUINN row through round 10);
`dossier-cengage-stem-homework.md`, `dossier-pearson-mcgrawhill.md`, `dossier-gradescope-edstem.md`,
`dossier-lms-brightspace-canvas.md`; `quinn-todo-checklists.md`; `t0-production-read.md`; the inventory's
slice E rows. **Read for the script:** `scripts/site-check/README.md` and `feed-shape.mjs` on
`tools/site-check-kit` (PR #85). **In part:** the pricing note's section 8 on `docs/v041-pricing` (tip
`f516c28b`).

**Verdict: PASS WITH FIXES. 0 Critical, 8 Important, 20 Minor. List A has 28 fixes that need no
judgment; list B has 5 decisions for Quinn.**

Both Blockers are closed, and so are all twelve Important findings from r1. The rulings are applied
faithfully, with five exceptions where the plan writes its own words into a ruling (A11 to A14). Nothing
here holds v0.4.1. Two fixes must land before the first Thanksgiving sitting, because the sitting is days
away and the plan describes it wrongly. A1: the plan describes a script that is not the one PR #85 built.
A2: the sheet as written can carry the school's host into a public report. The rest can be applied with
the plan's next commit.

---

## A. Fixes that need no judgment

Each fix gives where it applies, what is wrong, and the exact correction. "I" marks an Important fix;
every other fix is Minor.

**A1 (I). 5.2 "What it needs first" item 1 and "What the protocol needs from the script"; 11.2; 11.3.**
The plan says the script "has not been seen". The script exists (PR #85, `scripts/site-check/`), and five
of the plan's requirements describe something it does not do:
- It follows up to 5 redirects to any `https` host (README, "What is and is not kept"), so it does not
  contact "only the link's own host".
- By default its class lines read `class 1`, `class 2` with counts, taken from `CATEGORIES` only. Real
  class names appear only with `--show-titles` (`feed-shape.mjs:425-428, 493`).
- It prints no list of the hosts named in item links. It matches items against a fixed table of thirteen
  sites, the eleven homework sites on the strip plus zyBooks and VHL (`feed-shape.mjs:26-40`), and counts
  hits per site.
- It does not run Knowlu's paste check (`looks_like_ics`, `app/src/lms_link.rs:302-307`).
- It prints no explicit count of the gradebook-item shape or the noise marker. Both can be read from the
  UID shapes, because masking keeps the words `gradableitem`, `blackboard` and `calendar`.
- `--show-titles` shows every matched title and up to 40 unmatched ones, not "one item's title when the
  friend asks".

Correction: replace the bullet list with what the README states. Add one line: "The paste check and a list
of unknown link hosts are not in the script. F1 rests on its fixed site table, and 'which class uses which
site' is asked by eye, or with `--show-titles` on screen only." Delete "This plan has not seen it
(`unverified`)" and the 11.2 clause "the step-0 script, which is not in this worktree". Replace the 11.3
row "The step-0 script does what 5.2 asks" with "Read against PR #85's README in review r2 (A1)".

**A2 (I). 5.2 "The sheet" and "Where the results go".** The README says a UID shape still shows the host
after the UID's last `@` when at least two items share it, "usually the school's site host". The sheet
records "an id shape with digits masked", so a school's host can reach
`docs/reports/<date>-site-route-checks.md`. Also, the report is limited to "counts of Y and N … nothing
else", yet 5.5 adds sender domains "once step 0 … shows it". Correction to the sheet: "an id shape with
digits masked, and any host in it written as `<host>`, or as 'the school's own' when it names the school".
Correction to the report: "counts of Y and N per site and route, and the vendor sender domains M2
recorded; nothing else".

**A3. 5.2, "The proof, later", item 1; 11.3 row 3.** "Whether the engine the app starts inherits them"
is the wrong question. The engine reads `api_base` from the vault's cloud config
(`engine/src/cloudmodel.rs:123`). Onboarding writes that value from `account::api_base()` when it creates
the vault (`app/src/onboarding.rs:587-592, 732`). Correction: "The scratch vault is created with both
variables set. Its cloud config then names staging for every engine step, and the main session reads that
file before the first slot."

**A4. 5.3 map, row 5 (Expert TA).** Two cells disagree with the dossier.
- "which the dossier expects to be common" is not in the dossier, which says only that the instructor "may
  or may not set" the date. Correction: "A date the instructor left blank (the dossier: the instructor may
  or may not set it)".
- The cell leaves out that grades go back through the instructor's "Resend Grades" button (SD-08, "Proof
  of automatic passback"). Add to "What the first route is known to miss": "Scores that wait for the
  instructor to resend them".

**A5. 5.3 map, row 8 (Pearson), Fallback.** "S only for a course that is not linked" claims more than the
dossier does. The dossier lists "Signing in directly to Pearson shows the LMS-linked course" as still to be
confirmed. Correction: "K for a linked course, which is registered from inside the school's system. S for
an unlinked course, and for a linked one only if step 0's P2 shows the course at Pearson's own address."

**A6. 5.2 per-site table, Revel row.** The dossier's Revel list asks "Whether a direct sign-in shows the
course". The plan's P2 asks this for MyLab and Mastering only. Add "R3: signed in at Pearson's own address,
is the Revel course there?"

**A7. 5.17, the readers bullet.** "The dossiers expect: Smartwork, Expert TA, Gradescope, Aktiv"
contradicts the Cengage dossier. That dossier ranks Aktiv third-cheapest, because its Homework and Quiz
columns carry due dates by default (SD-05). It also contradicts the plan's own Q19 list (Smartwork,
Edfinity, Expert TA, Gradescope). Correction: "The dossiers expect: Smartwork (ST-11), Expert TA (ST-09),
Gradescope (ST-06); Aktiv (ST-08) only if step 0 shows its columns lack due dates."

**A8 (I). Section 4 table, in four places.**
- **v0.4.5's IDs.** The list omits ST-10's first route, which ledger 6.5 places here ("ST-10 Edfinity
  (scores)"), and it lists ST-07 twice. Replace with: "ST-01, ST-03 to ST-06, ST-08 to ST-13 (each by the
  school-system route, where the checks show it), ST-07 (mail), ST-14 (feed half), ST-15".
- **v0.4.1's IDs:** add "C23 (words only)", as in 6.3.
- **v0.4.2's IDs:** add "BR-04's remainder", as in 5.8 and 6.4.
- **The "Depends on" column** misses three dependencies:
  - v0.4.5 also needs v0.4.3 and v0.4.4, because part A of 5.3 is built on F-17 and part B is the site
    table. Add "v0.4.3 (F-17), v0.4.4 (the site table)".
  - v0.4.11 also needs v0.4.5, because the switch reads parts A and C. Add "v0.4.5 (route health)".
  - v0.4.16 also needs v0.4.7 and v0.4.11, because 5.3's part E is built in v0.4.11 "before the first
    fallback reader", and 4's own order rule puts v0.4.7 before the readers. Add "v0.4.7, v0.4.11 (the
    matching rule and the switch)".

**A9 (I). 5.27 (v0.4.21), and 7.3's v0.4.21 row.** The plan treats the backfill as undesigned ("the spec
starts there"; "What the spec must say: how far back it reads"). Its documents exist. One is a signed spec,
`docs/specs/2026-10-10-m365-backfill-redesign.md` (signed by Quinn's delegated go, `HANDOFF.md:182, 615`);
it fixes the window at 120 days (spec line 107). The other is a plan, `docs/plans/2026-10-18-c3-m365-backfill-plan.md`.
Asking how far back to read re-asks a signed decision. Corrections:
- Replace the "What it is" sentence "This roadmap did not read …" and the whole "What the spec must say"
  paragraph with: "Its spec is signed and its plan exists. As with the syllabus (5.18), the plan is re-cut
  against today's `main` and the new UI."
- Delete 7.3's v0.4.21 row.
- Add to its must-not-break: "The spec's fallback for a frozen hidden page sets browser arguments on the
  mail window (spec lines 209-216). The re-cut keeps that inside the one module 5.4's portable rule allows,
  and says what it is on WebKit (M-11)."
- Replace the Privacy line's "whichever has not shipped": B2 always ships first, by v0.4.21's own
  dependency. Write instead: "B2's drafter checks now; if the page needs a sentence, B2 carries it in the
  conditional form."

**A10. 5.6 (row 2), 5.7, 10.1 (BR-01 row).** Two facts have changed since the plan was written:
- The facts report is committed and pushed on `docs/v041-pricing` at `f516c28b` (ledger, 09:05). It is
  no longer "still an uncommitted file". Correction for 5.7's first bullet and for 10.1: "BR-01 merges
  first; the facts report is already committed on it."
- The pricing note now has an item 16, answered as ruling 20. Correction for 5.6 and 5.7: "Answered: 2, 4
  (ruling 28), 8 (ruling 10), 14 (ruling 4), 15 (ruling 31), 16 (ruling 20)."

**A11 (I). Ruling 16's condition: the rulings list (lines 76-78), 2.3, 5.30 "Before the tag", 8.1.** The
ledger records the condition as "the one condition is tax, expected to be met by Stripe as seller under
v0.4.1's managed payments, confirmed by v0.4.1's check". The plan turns this into "If the answer [from the
lawyer or an accountant] is on record and says covered". That adds a gate Quinn did not set. Corrections:
- Quote the condition as recorded in the rulings list and at the top of 2.3.
- Present the question to an accountant and the dated answer line as "the planner's recommendation,
  pending Quinn (review r2, B1)", not as a precondition.

**A12 (I). Ruling 29 (lines 123-125); section 1 (lines 212-213); 2.3 (lines 392-393); 5.30.** The words
"and is tied to v0.5.0's tag" are not in the ledger. They come from r1's I9 recommendation. Corrections:
- Remove them from the ruling's text.
- Where the plan makes Google's verification a precondition of the tag, mark it "the planner's addition,
  pending Quinn (review r2, B2)".

**A13 (I). Ruling 23: the rulings list (lines 98-100), 2.1E, 5.5, 11.1 item 16.** The plan and the ledger
disagree about whose words are whose:
- "The terms carry that condition" (the whole semester) is the controller's note in the ledger, not
  Quinn's ruling. Mark it "the planner's reading; Q18 asks it".
- 11.1 item 16 calls "for as long as the service runs" the planner's words. The ledger records those words
  as the option Quinn approved. Delete item 16, or reword it: "the words of the option Quinn approved".

**A14. Ruling 24 (lines 104-105).** "against staging" is not in the ruling. It is `CLAUDE.md`'s rule for
live runs. Correction: end the quote at "on live spring courses", then add "(against staging, by
`CLAUDE.md`'s rule)".

**A15 (I). 5.6 drafts: ruling 23 has no draft.** "Free forever" touches two signed sentences:
- D2, "No free tier" (`docs/specs/2026-09-09-knowlu-cloud-design.md:14`);
- Amendment 2026-10-16b (1), "they do not pay" (`:806-807`), which ruling 23 extends past the pilot.

Each later price amendment restates "D2's 'no free tier' … stand[s]" (`:776`, `:1035`). Correction: add
ruling 23 to draft B's "Records" column, naming `:14` and `:806-807`, and record the grant as "a grant, not
a tier", as those amendments do. Constraint 8 then covers it.

**A16. Draft C (5.6) and 10.3: two signed passages missing.**
- **Draft C.** Ruling 35 extends ruling 12 to Canvas and Brightspace grades, and draft C carries that
  extension. But C does not name `VISION.md:83-87` ("not an LMS API"; grades "from the LMS grade page"),
  which 10.3 already lists for v0.4.17. Add `VISION.md:83-87` to draft C's list.
- **10.3.** Ruling 2 (all twelve sites in v0.5.0) moves "more homework platforms" out of "Beyond"
  (`VISION.md:239-241`). 10.3 lists only `:240` for GPA. Add a row: "`VISION.md:239-241`, 'more homework
  platforms' under Beyond | v0.4.5 | Ruling 2".

**A17. Q21; 11.3's last row.** `HANDOFF.md:531-535` holds only the Pilot's and Launch's exits. Correct the
other citations:
- the twentieth payer: `VISION.md:234` (Amendment 2026-09-30, A13);
- "MVP reached": `HANDOFF.md:611`;
- the pilot's length: `HANDOFF.md:612`.

Replace "per the inventory" with these.

**A18. 10.3, the slot-order row; 11.3's slot-order row.** This can be settled now. `app/src/scheduler.rs:505`
runs sync → coursework → grades → ingest → judge → rank. Correction: change "Today, possibly … `unverified`;
`docs-keeper` confirms" to "Today (`app/src/scheduler.rs:505`)", and delete the 11.3 row. Quinn still words
the `CLAUDE.md` line.

**A19. Section 1, "What can hold v0.5.0" and 5.6 ("Borrow the Mac four times"); 5.4.**
- The count of four is low. Write: "four times, plus once if the Developer ID certificate is made on it
  rather than on a macOS runner (5.4, step 3), plus once for each floater that ships after v0.4.23
  (section 4)".
- Before 5.4's step 1, add Apple's prerequisites from the checklist file: a public website on the
  company's own domain, a work email on that domain, and a two-factor Apple Account.

**A20. Section 1 item 6; section 4, "The four that float".** The plan says all four "depend on almost
nothing after v0.4.7", but v0.4.22 waits on four things (5.28): ruling 28, draft B, the tax answer and C1.
Correction: "Three float freely. The gift floats once Stripe is the company's, draft B is signed, the tax
condition is met and C1 exists (5.28)."

**A21. 5.23, "Why the boundary".** "Each school system is its own Contract piece and each ships alone"
contradicts one release, v0.4.17. Correction: "Canvas and Brightspace are two Contract PRs in v0.4.17. If
Brightspace's spike is late, v0.4.17 ships Canvas alone and Brightspace takes the next free number. If the
spike says no, it returns to Quinn."

**A22. 7.2 and 7.4 order.** 7.2 says it is "in the order they block", but the questions that block the
sites spec (Q19, Q28) come after Q30, which blocks only v0.5.0. Q23 blocks nothing yet sits seventh.
- Correct 7.2's order to: Q18 (the terms paragraph), Q27 (an act that starts now), Q19, Q28 (the sites
  spec, right after step 0), Q22 (v0.4.6), Q25 (v0.4.9), Q24 (v0.4.20), Q26 (v0.4.22), Q21, Q30
  (v0.5.0's tag), Q29 (v0.4.23, conditional), Q23, Q20 (nothing).
- In 7.4, give the order in which the signatures block: D and A before v0.4.3; C before v0.4.4 ships;
  E before v0.4.10's re-cut; B before v0.4.22 and v0.5.0's spec.

**A23. 5.7, "Must not break" against 5.5 and 5.7's ruling 23 bullet.** 5.7 says "This roadmap changes no
task in it", yet the paragraph is "offered to T5". Correction: "offered to v0.4.1's controller, who
decides whether T5 carries it. If it does not, the paragraph is the first terms version after v0.4.1."

**A24. 2.1A (line 247); 5.30, "Design rules".** Ruling 29 keeps direct Gmail for testers Quinn names, but
the plan says "Forwarding for every new student". Correction: "for every student who is not a tester Quinn
names (ruling 29)". Change "Google asked for Calendar only" to "for those students, Google is asked for
Calendar only".

**A25. "ambiguity 3 / 6 / 8 / 14 / 15" (section 4, 2.3, 5.3, 5.10, 5.17).** No section is called
"ambiguities". The numbers match 11.1's readings. Correction: "reading N (11.1)".

**A26. 2.2, the "Ruled to stay after v0.5.0" table.** Ruling 35 says these items "wait". The table writes
in two things Quinn did not say:
- The instructor's name: drop "or dropped".
- "Teach once" goes to "Beyond" and the waitlist to "after the pilot". Both destinations are the planner's
  placements, so label them that way.

**A27. 5.24 (v0.4.18).** Three things are missing:
- A privacy line. Add: "Privacy: none expected. Class-note fields ride the vault copy the page already
  describes; B2's drafter confirms."
- What a final grade is before v0.4.17. Add: "At a school whose grades Knowlu does not read yet, the final
  grade is the student's confirmed number."
- The confirm card in the size. Name it there: Contract on `approvals.rs` if it is a new card kind.

**A28. 5.6, row 3 (public hygiene).** "A change of business type" and "A new tax form is to be expected"
are entity details. The checklist file already holds them. Correction: "Can the account move to the
company in place while Managed Payments is on? Nothing is changed before Support answers."

---

## B. Decisions only Quinn can make

Each is new in r2. The plan's thirteen (Q18 to Q30) stand as written, after A22's reordering; each has a
recommendation, and none repeats a ruling, except Q27, which re-asks ruling 29's "now" for a stated new
fact (a submission is judged against the live page, and B2 carries the Limited Use statement). I agree
with its recommendation.

**B1. Tax: is v0.4.1's check enough to call tax "covered", or must an accountant's answer be on record
before a stranger is charged?** As the ledger records it, the condition is met by Stripe as seller under
Managed Payments, "confirmed by v0.4.1's check". *Recommendation:* the condition as ruled holds the tag. Ask
the accountant question anyway, because it costs little, and hold checkout only if the answer says more is
owed. *Blocks:* v0.5.0's tag and v0.4.22.

**B2. Is Google's verification of the Calendar permission a precondition of v0.5.0's tag?** The plan makes
it one; ruling 29 did not say so. *Recommendation:* yes, and Quinn may waive it at the tag. An unverified
app in production shows strangers the "unverified app" screen and can add only 100 new users for the life of
the project (checklist file). *Blocks:* v0.5.0's tag.

**B3. Lock in (Q24): a separate pop-out window, or the plan's compact mode?** Ruling 35 picked "Pop-out
Lock in window". The plan recommends a compact mode of the one window, which is not a pop-out: the full app
is hidden while it runs. *Recommendation:* the second window, with 5.26's pin change (its own capability,
an exact command list, a security read), unless Quinn meant only "small and on top". *Blocks:* v0.4.20.

**B4. The step-0 sheet keeps more than yes and no.** Ruling 24 says "only yes/no answers kept". The sheet
also keeps vendor sender domains, page names and masked id shapes. *Recommendation:* allow those three,
with every host masked as in A2, because SA-07 needs the domains and nothing in them names a person or a
school. *Blocks:* the first sitting.

**B5. "Free forever" (fold into Q18): do the terms carry the site's condition, and does it go into draft
B?** The condition reads "Take part for the whole semester" (`site/index.html:392`). *Recommendation:* yes
to both. The condition is the promise as published, and the grant touches D2 (A15). *Blocks:* the terms
paragraph and the pilot's grant.

---

## Review r1 findings

| r1 | Status | Where in r2 |
|---|---|---|
| B1 the checks ran on production | Closed (ruling 24). The new issue in step 0 is A1 and A2 | 5.2 |
| B2 the waves contradicted the dossiers | Closed in structure. Four cells or lists are still wrong (A4 to A7) | 5.2, 5.3, 5.17 |
| I1 the Mac spike on an unsigned build | Closed | 5.4, 5.6 |
| I2 the sales gate treated as decided | Closed by ruling 16. The condition is now overstated (A11) | 2.3 |
| I3 Stripe to the company | Closed (ruling 28) | 5.6, 5.30 |
| I4 the `CLAUDE.md` wording | Closed | 10.3 |
| I5 Q7's signed texts | Closed. One passage is added by A16 | 5.6 draft C, 5.19 |
| I6 sizes and hidden triggers | Closed | 4, 5.8, 5.9, 5.11, 5.14 |
| I7 the salvage map | Closed | 5.9, 5.14, 5.19, 5.30, 6.8 |
| I8 proofs; the readers' privacy | Closed | 5.3, 5.1 |
| I9 Google's testing mode | Closed (ruling 29). The tag precondition is attributed to the ruling (A12) | 5.6 |
| I10 the winter break | Closed | 1, constraint 14 |
| I11 the private ledger | Closed | 6, 9.1 |
| I12 the relay | Closed (ruling 30) | 5.16 |
| m1 wrong facts | Closed. r1's first item was mine and wrong: `:92` is the "Cut" line and the rows end at `:106` | 2, 3, 5.15, 5.6 |
| m2 decision order | Partly closed (A22) | 7.2 |
| m3 words attributed to Quinn | Closed for the three; three new ones (A12, A13, A14) | rulings list |
| m4 the v0.4.1 slot | Closed | constraint 5, 5.7, 5.1 |
| m5 contradictions | Closed | 5.1, 9.2 |
| m6 hygiene | Closed (one new: A28) | rulings 3, 14 |
| m7 the Mac draft | Closed | 5.6 draft A |
| m8 invariant wording | Closed | 5.13, 5.15, section 5 |
| m9 housekeeping | Closed, but now stale (A10) | 5.7, 10.1 |

## Found sound

- **Rulings.** Rulings 1 to 15, 17 to 22, 25 to 28 and 30 to 37 are applied as the ledger records them,
  with nothing softened or re-asked. The planner's readings (rulings 2, 6, 10 and 13; 11.1) are labelled.
  The verbatim quotes of rulings 5, 12, 25 and 33 are exact.
- **Drafts.** Drafts A, C and D name the signed lines they change, and they cite them correctly. Draft A
  changes only the opening words of `VISION.md:150`. Draft B names `:808`, `:816-827` and
  `VISION.md:234, 254`; A15 adds the rest.
- **Quinn's decisions.** Nothing that is Quinn's alone is settled. Money, prices, tags, production,
  `VISION.md` and `CLAUDE.md` wording and legal risk each end in a ruling, a draft or a question.
- **The seven pulled-in releases and the relay.** Each is sized right, the relay included: five Contract,
  v0.4.20 a Lane with a security read, the relay Contract. Each depends on the right things: GPA after the
  term field, the scale and final marks; Canvas and Brightspace grades after the kept sign-in; every
  reader after the relay. The privacy bumps are seven, each in a different release, and each is in a
  release that is Contract anyway. `account.rs` is serialized (constraint 4).
- **Fidelity.** All 233 IDs are present, slice by slice (A 35, B 30, C 51, D 43, E 28, F 25, T 21), with
  no ID homeless and no ID in two places. Every ST- number matches the inventory.
- **Cross-references.** Every section cross-reference resolves after the renumbering, and the count of
  twenty-four is right.
- **Vendor figures.** The plan's numbers for Google, Apple and Stripe match the checklist file.
- **Dossier cells.** Every 5.3 and 5.2 cell not named in A4 to A7 matches its dossier: Brightspace,
  WebAssign, MindTap, Aktiv, Edfinity, Smartwork, Connect, ALEKS, Gradescope and Ed, and the terms table.
  D2L's "do not apply" line is in the inventory.
- **Honesty about time.** The plan is honest about time and the outside dependencies: the spring term, the
  pace check after v0.4.3, the Apple enrollment with its D-U-N-S step, the borrowed Mac (A19 fixes the
  count), and real students in live courses.
- **Hygiene.** There are no absolute paths, project ids, school names or student data. The personal
  details are paraphrased.

**Path:line claims new in r2, checked at `515d13de`: 27 correct, 1 wrong.**
- **App and engine:**
  - `app/src/account.rs:15-17, 18, 42-47, 646-669`
  - `app/src/grades.rs:57-66`
  - `app/src/scaffold.rs:87, 107`
  - `app/src/lms_link.rs:299-301, 302-307`
  - `engine/src/ingest.rs:225, 567`
  - `engine/src/surface.rs:2376-2377`
- **Tests:**
  - `app/tests/mail_window.rs:79-104`
  - `app/tests/ui_shell.rs:809-812`
  - `app/tests/ui_wizard.rs:478-492`
- **Cloud:**
  - `cloud/supabase/migrations/20260910000100_accounts.sql:61-65`
  - `cloud/supabase/functions/_shared/mail_templates.ts:180-185`
- **Docs:**
  - `docs/reference/engine-commands.md:91-101, 94-96`
  - the cloud design: `:30, 256, 444, 554, 679, 683-684, 699-700, 808, 816-827, 829-831`
  - `VISION.md:48, 68-69, 83-84, 150, 182, 234, 236-237, 240, 241, 254`
  - the onboarding draft: `:147, 149, 166-175, 177`
  - the legal rewrite: `:473`
  - the syllabus spec: `:1156`
  - the email spec: `:2093`, with its quote exact
  - the relay design: `:33-45, 52, 98, 353-362, 506-510, 567-577, 609-617`
  - the grades design: `:301`
- **Site and handoff:**
  - `site/index.html:392`
  - `site/privacy.html:26, 30, 69`
  - `HANDOFF.md:182, 454-456, 613, 616, 618`
- **Wrong:** `HANDOFF.md:531-535`, cited for the twentieth payer (A17).

## Not verified

- The script's tests were not run; its behaviour was read from the README and its source.
- The onboarding branch's line numbers (the salvage map's) and the relay's twelve-task plan.
- The M365 backfill plan, beyond its existence and the spec's header, window and fallback lines.
- Whether the privacy page's mail section already covers the M365 backfill.
- The Google project's publishing mode.
- Whether Ed Lessons carry dates.
- The inventory's code lines behind F-, O-, C, M- and T- rows that r2 did not re-open.
- No build, test or trial merge was run.
