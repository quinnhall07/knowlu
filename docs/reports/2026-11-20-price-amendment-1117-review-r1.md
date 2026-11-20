PASS with changes

# Review r1: Amendment 2026-11-17 (draft), cloud design

Reviewer, spec mode, round 1, 2026-11-20.

**Target:** `docs/specs/2026-09-09-knowlu-cloud-design.md:1234-1511` (unsigned draft) and the D2 marker at `:14`. This is the working tree of `docs/price-amendments` on top of `7028d416`. `git diff --stat` touches only that file, so there is nothing for contract-reviewer.

**Judged against:** Amendment 2026-11-10 (`:993-1232`, signed); legal note §0 and D7 (`docs/specs/2026-11-17-legal-pages-rewrite.md:96-180`, `:230-234`, `:640-648`); the v0.4.1 pricing note at `v041-pricing` `a95ca620` (§2, §8); Quinn's words of 2026-11-20 as the dispatch gives them.

**Checked and clean:**
- Every quote of R2, R3, R4, U5, U6 and U12 matches §0 byte for byte. The 2026-11-20 quote (`:1360-1362`) matches the dispatch's text, apart from line wrapping.
- Every price sentence says "every 4 weeks". The amounts are exact: $12.99, and $6.49 every 4 weeks billed $84.37 once a year. "Month" appears only in quotes, in plan keys and in descriptions of old text.
- Public-repo hygiene holds. The draft names no person beyond Quinn, gives no address, no account or project id, and no entity detail. It does not even name the LLC.
- I opened every code and page cite in *Consequences* (billing-checkout, stripe-webhook, billing-jobs, wizard.js, carried/account.js, settings.js, account.rs, site/terms.html, subscribed.html, index.html, release.yml, HANDOFF `:24-26` and `:80`, VISION `:153-154`, `:234`, `:236`) and each one says what the draft says.
- P15 matches the legal note's D7 terms text, cases (a)–(d).
- The D2 marker replaces 11-10's marker instead of appending to it, drops the Pell clause and carries the tax sentence.

## Important

**I1. (7)'s three requirements are presented as Quinn's ruling. They are the drafter's reading.**
- *Where:* `:1368-1374`. The header at `:1244` says "Rulings (6) and (7) are Quinn's ruling of 2026-11-20".
- *Conflicts with:* Quinn's words ask only that "if we change prices in the future (lower or raise), we can make the proper adjustments". "No app release", "one place on the server" and "each later price change is made by its own amendment" are not in them.
- *Consequence:* Quinn signs, and the v0.4.1 lane treats "a price change needs no app release" as a signed requirement. Under I2 below, it cannot meet that requirement. Quinn's sentence would also stand for engineering choices Quinn never made.
- *Fix:* introduce the bullets with "This amendment reads that as:", and send the strict form of the first bullet to *Open for Quinn* (see I2).

**I2. The first requirement of (7) contradicts the design the draft itself recommends, and two settled rules.**
- *Where:* `:1370-1371` against Open item 2 (`:1481-1491`).
- *Conflicts with:*
  - Open item 2's recommendation keeps the amounts in the terms, so every price change is also a terms bump.
  - The legal note's C25 (`:849`) deploys the legal pages only on a release tag.
  - The pricing note's D3 floor ties `CHECKOUT_TERMS_VERSION` to the compiled `TOS_VERSION`. The pricing note says so itself: "every later terms bump ... refuses every older build at checkout".
- *Consequence:* the signed amendment demands what its own recommendation rules out. The lane then either breaks C25 by deploying terms outside a tag, or ships work that fails a signed requirement. The draft leaves the clash for "the lane's design" (`:1490-1491`), which is a third round in disguise.
- *Fix:* recommend one of two things now.
  - (a) A price change rides a release tag. Releases are CI-only, and the floor forces the update. The requirement then reads "no code edit beyond the server's price setting and the terms text".
  - (b) Drop amounts from the terms ("the price shown at checkout"). That is a lawyer's-read item.

  I recommend (a).

**I3. "(7) sets no policy for existing subscribers" is not true once terms are published.**
- *Where:* `:1376-1377` and `:1505-1511`.
- *Conflicts with:* the live terms already promise that a change "will not apply to a period you have already paid for" (11-10's lawyer's read, `:1177-1178`). The terms this amendment sends out with v0.4.1 (legal note `:625`) add 7 to 30 days' notice.
- *Consequence:* a later amendment, reading "each change decides it", rules that a raise reaches yearly subscribers mid-term or with 3 days' notice. That breaks the published contract.
- *Fix:* say that the published terms' price-change clause is the floor every later amendment works within. The *at the time of a change* items then decide only what lies above that floor. Give each item a default recommendation: a raise at the next charge after notice, never inside a paid period; a cut reaches everyone at their next charge.

**I4. (6) treats "zero customers" as a fact the lane never checks.**
- *Where:* `:1363-1367`, `:1395-1397`, and `:1438` ("The old plans map to nothing new").
- *Conflicts with:* the pricing note says "Production's state is not known from the repository" (`:39-40`). It reads production first (§4 step 0), and item 9 covers only Quinn's own accounts.
- *Consequence:* step 0 finds an active `monthly` row that is not Quinn's, for example a friend who paid before the gate. The draft strikes P10, the old-subscriber paragraph and the old price ids, so the webhook gives that row `plan: null`. `billing-jobs` then fails on it every day (`billing-jobs/handler.ts:69-75`), and the student's terms are gone.
- *Fix:* add one sentence to (6) or to the Reading: "If the lane's production read finds a subscription not on Quinn's accounts, it stops and asks Quinn before the switch."

**I5. A cite strikes standing text.**
- *Where:* `:1297`, "*The Pell claim* (`:1188-1219`)".
- *Conflicts with:* that range is the whole Billing, App, Site and Documents consequence block of 11-10. It includes "the annual reminder stays, for every plan" (`:1200`), the test-data item (`:1204-1205`) and the docs items (`:1215-1216`). *The Pell claim* is `:1217-1219`.
- *Consequence:* a docs-keeper or lane reading the cite literally drops the annual-reminder rule, which California requires, and the `academic_year` test-data cleanup.
- *Fix:* change the cite to `:1217-1219`.

**I6. D2's marker is rewritten before Quinn signs.**
- *Where:* `:14`; the draft acknowledges it at `:1254-1255`.
- *Conflicts with:* 11-10's rule at `:1008-1009`, "Once this is signed, those markers are replaced". It also conflicts with CLAUDE.md, which treats §1 and its amendments as Quinn's signed decisions.
- *Consequence:* this branch is docs-only and could be pushed without a go. If it is, §1 cites an unsigned amendment as amending D2. If Quinn then changes (8)–(10) or declines to sign, a reader of §1 has acted on text that never became authority. Reverting depends on someone remembering to do it.
- *Fix:* keep 11-10's marker until signing. Put the new text under *The markers* as "At signing, D2's marker becomes: …". The other choice is to keep the branch unpushed until Quinn signs, and say so in the draft.

**I7. The PENDING markers are ambiguous, and swapping one means edits in other places.**
- *Where:* `:1401`, `:1410` and `:1418`.
- *Ambiguous:* "[PENDING QUINN, item 5]" sits in a document that has its own numbered *Open for Quinn* list, and that list's item 5 is the grant application (`:1500`). Write "pricing note §8 item 5".
- *Not swap-ready:*
  - (8)–(10) are referred to outside their blocks, without markers: `:1305-1307`, `:1317` ("(10) says when its terms publish"), `:1449-1451`, `:1459` ("no gift paragraph (10)") and `:1483`, `:1489`.
  - A "no" on one item needs edits in up to six places.
  - Item 5's "no" branch is already written in the pricing note D9: A2 narrows P9, and the A2 pins gain the figures. The draft does not carry it.
- *Fix:*
  - Mark each outside reference "(if (8) is signed)".
  - Write (8)'s alternative text under its marker, so that Quinn's answer is a swap and not a redraft.

## Minor

- **M1.** (6) says it "answers the pricing note's item 3" (`:1367`) without saying how. Item 3 recommends keeping `academic_year` rows, and (6) takes the opposite course. Write "Item 3's recommendation is not taken: there are no rows to keep."
- **M2.** When (6) takes effect is not stated. Until v0.4.1 ships, `billing-checkout` still sells `monthly` and `academic_year` (`index.ts:16-20`). The live terms state those prices, and U10's interim terms (bump A) were approved at the old prices. Say that (6) takes effect at the v0.4.1 switch, that 10-16b's gate holds until then, and whether bump A's interim terms still publish.
- **M3.** Leftover Pell and P10 text that the replaced list does not name:
  - "Pell yearly included" in *Decided at signing* (`:1150`);
  - "the pause job runs only for P10's kept subscriptions" and "P7's notice" (`:1199-1201`);
  - P10's title, "Codes and old subscriptions".
- **M4.** Open item 1's recommended §5.1 marker says, without condition, that Knowlu "is sold through Link under Stripe Managed Payments" (`:1478-1480`). Open item 3 reads (1) as true only once Managed Payments is in effect. Make the marker conditional, or set it when C5 or S1 proves it.
- **M5.** P8 says "the webhook learns of both", but refunds and disputes in the webhook are not in v0.4.1 (`:1444-1445`). P12 and P15 still publish with v0.4.1, including "access ends the day of the refund". Name the interim manual path: the subscription is cancelled in the Dashboard at refund, so that P8 holds.
- **M6.** The draft cites the pricing note by line (`:74-76`, `:87-92`, `:97`, `:284-298`, `:310-334`, `:420-426`, `:656-691`). That note is being revised for (6) right now, and its D1 will be rewritten. Cite by section (D1, D9, D11, §8 item N), or re-check the line numbers in round 2.
- **M7.** "What still stands" (`:1319-1320`) lists only two open items of 11-10. The parts of the Managed Payments item and the lawyer's read that remain (`:1291-1294`) stand too. Add them to the list.
- **M8.** "quoted in full at (6)" (`:1245`): the quote sits in the heading paragraph above (6). Write "quoted above (6)".

## For round 2

Round 2 checks that I1–I7 and M1–M8 are closed and that the fixes break nothing else. It does not reopen what passed above: the quotes, the prices, hygiene, P15 and the code cites.
