PASS with changes

# Review r2: Amendment 2026-11-17 (draft), cloud design

Reviewer, spec mode, round 2 (the last), 2026-11-20.

**Target:** `docs/specs/2026-09-09-knowlu-cloud-design.md:1234-1565`, commit `b04fe772` on `docs/price-amendments`. Judged against round 1 (`docs/reports/2026-11-20-price-amendment-1117-review-r1.md`), the controller's rulings on it, and Quinn's words of 2026-11-20 as the dispatch gives them. No contract-list file is in the diff.

## Round 1, finding by finding

- **I1** resolved. (7) is now Quinn's words alone (`:1375-1376`). The header says the reading is not Quinn's (`:1244-1246`), and *Reading (7)* opens "This amendment reads Quinn's words as" (`:1391`).
- **I2** resolved by the controller's ruling. Whether a change also needs an app or terms release is left to the lane's runbook (`:1395-1397`). One residue remains in the bullet above it: see N2.
- **I3** resolved. *The promises every later price change keeps* (`:1401-1410`) sets the published terms as the floor. *Open for Quinn, at the time of a change* (`:1558-1565`) decides only what lies above it. Two of its wordings slip below the floor: see N3 and N4.
- **I4** resolved (`:1382-1386`). If production holds a paying subscription that is not on Quinn's accounts, the lane stops and asks.
- **I5** resolved. *The Pell claim* is cited at `:1217-1219` (`:1301`), and `:1302` keeps the rest of the block standing, the annual reminder included.
- **I6** resolved. `:14` is untouched by `b04fe772`. The new marker text waits under *The markers* "at signing, and not before" (`:1328-1334`, `:1255-1256`).
- **I7** resolved.
  - The markers say "the pricing note's §8 item" (`:1249-1251`), and the amendment's own items are O1–O5 (`:1533-1534`).
  - Each of (8)–(10) carries its "no" text (`:1449-1452`, `:1462-1464`, `:1473-1476`).
  - References outside the blocks are now neutral: "(8)'s and (9)'s to settle" (`:1507`), "(10)'s to settle" (`:1515`). Either answer is a swap.
- **M1** resolved (`:1372-1374`).
- **M2** resolved (`:1379-1389`). It says when (6) takes effect and that 10-16b's gate holds until then. Bump A carries privacy only.
- **M3** resolved (`:1283-1284`, `:1286-1287`, `:1290-1291`, `:1301`).
- **M4** resolved. O1's marker is conditional (`:1537-1540`).
- **M5** resolved by *Refunds by hand* (`:1426-1432`).
- **M6** resolved. The pricing note is cited by section throughout (`:1248`).
- **M7** resolved (`:1322-1325`).
- **M8** resolved (`:1245`).

## Checked and clean

- **Line 14.** `git diff origin/main` changes `:14` only through `7028d416` (11-10's signed marker). `b04fe772` is one appended hunk (`@@ -1230,3 +1230,336 @@`) and touches no earlier line.
- **Quotes.** Quinn's 2026-11-20 words (`:1365-1367`) match the dispatch byte for byte. (6)'s fragment and (7) are exact substrings of them. R2, R3, R4, U5, U6 and U12 still match the legal note's §0 (`:105-116`, `:155-163`, `:176-177`).
- **Promise quotes.** The two quoted promises are exact: `site/terms.html:25` ("before it happens", "the change will not apply to a period you have already paid for") and the legal note `:625`.
- **Prices and wording.** $12.99; $6.49 every 4 weeks, billed $84.37 once a year; no stray amounts. "a month" appears only inside R3's quote (`:1351`).
- **Cites.** The new cites resolve: `:777-778`, `:830-831`, `:1008-1016`, `:1111-1112`, `:1154-1182`, `:1188-1219`, `HANDOFF.md:24-26`, `CLAUDE.md:199`, `VISION.md:153-154/:234/:236`, the v0.4.0 design `:26-27`, `billing-jobs/handler.ts:130`, `stripe-webhook/handler.ts:66-67`, `billing-checkout/handler.ts:59`, `site/subscribed.html:13`, `site/terms.html:24/:28/:39`.
- **Hygiene.** No person but Quinn is named. No entity, address or account id appears. The grant appears only as "a seed-grant application".
- **Labels.** The promises paragraph's defaults are labelled "*Recommend:*" under a heading that says the change's own amendment decides them (`:1558`). None reads as a ruling.

## Important

**N1. O4 misstates the terms Quinn approved, so its reading clashes with them.** Text fix to O4. It then stays a Quinn decision.
- *Where:* `:1549-1552`, and *Refunds by hand* "(c) is O4" (`:1431`).
- *Conflicts with:* the same approved terms, legal note `:633`: "You keep what you have paid for until that period ends, unless you take a refund, which ends access the day it is made." O4 says the terms end access "only after a (d) refund" (`:648`). That holds for `:648` alone, not for the terms as a whole.
- *Scenario:* Quinn confirms O4 as written. Later, a student cancels and asks for a (c) refund at the same time. The operator, following O4, leaves the subscription running and refunds the charge. That breaks `:633`, the period counts as unpaid under P8, and access continues anyway. Separately, C12's webhook, built from P8 alone, cuts off a (c) student who did not cancel, which is the opposite of O4's reading.
- *Fix:* O4 cites `:633` too and reads as follows.
  - A (c) refund with a cancel ends access that day (`:633`).
  - A (c) refund without a cancel leaves the subscription running. The refunded first period is the one exception to P8, and C12's refund handling must carry that exception.
- *Recommend:* confirm that reading.

**N2. *Reading (7)*'s second bullet cannot hold for the terms.** Text fix.
- *Where:* `:1393-1394`: "every surface that states a price reads it from there".
- *Conflicts with:* the v0.4.1 terms state the amounts as static text (legal note `:620`). C25 publishes legal pages only on a release tag (legal note `:849`). The next bullet leaves the terms release to the runbook (`:1395-1397`).
- *Scenario:* once signed, the lane must either make `site/terms.html` read a price from the server, which breaks C25 and the versioned text tied to `TOS_VERSION`, or ship work that fails the signed reading. That is I2 again, in a smaller form.
- *Fix:* "every surface the code builds (checkout, the consent record, billing emails, the app) reads it from there; the published terms state it in their own version, under the runbook."

## Minor (all text fixes)

- **N3.** `:1560`, "within or beyond the published 7 to 30 days". The terms promise notice "between 7 and 30 days ahead" (legal note `:625`). "Beyond" in either direction breaks that window. Write "within the published 7 to 30 days".
- **N4.** `:1563-1564`, the cut recommendation "everyone, at their next charge". `:625` promises notice 7 to 30 days ahead for any price change, so a cut also needs it. A student whose next charge falls 3 days after the notice gets a cut with less notice than the terms promise. Write "at their first charge at least 7 days after the notice".
- **N5.** `:1462-1464`, (9)'s "no" text. It does not say what the 4-week-only sheet shows, while the "yes" text says "with the subscribe step's lines". Add "with the subscribe step's 4-week line (none if (8) is not signed)".
- **N6.** `:1385` is one unwrapped line, about 170 characters. Rewrap it.

## For Quinn

Everything above is a text fix the controller can apply. The decisions at signing are the draft's own: (8)–(10) and O1–O5, each already carrying a recommendation. Only one recommendation changes in this round.

- **O4, access after a (c) refund.** *Recommend:* use N1's reading. If the student cancels, access ends on the day of the refund, as the approved terms already say. If the student does not cancel, the subscription goes on and the refunded first period stays free. That is the cost of the reminder Knowlu failed to send, and it is the one exception to P8 that the refund webhook must carry.

Verdict: **PASS with changes.** Once N1–N6 are applied, the draft is ready for Quinn's signature. No third round.
