PASS

# Review, round 2 of 2: the draft Amendment 2026-11-10 (the price)

**Reviewed:** `docs/specs/2026-09-09-knowlu-cloud-design.md:993-1172`, uncommitted on `docs/waitlist-spec`, base `5828b361` (`git diff --stat`: that one file, +181). Checked against round 1 (`docs/reports/2026-11-10-price-amendment-review-r1.md`) and against the code the consequences name.
**Mode:** spec/plan, round 2 of 2, the last round.
**Verdict: PASS with fixes. 0 Critical, 4 Important, 10 Minor.**
- All 25 round-1 findings are closed. Three closures (I8, M4, M8) opened new gaps, listed below.
- The new 4-week ruling is recorded faithfully, and "every 4 weeks" appears wherever the amendment states the short plan's price.
- The draft should not be signed until I-A to I-D are fixed. I-A is a display ruling, so it goes to Quinn. The other three are one or two sentences each. No third round: once the fixes match the wording below, Quinn signs.

---

## Round-1 findings

| # | State | Where answered |
|---|---|---|
| I1 | Closed | `:1063-1066` (P9): before the card, both amounts, the 6-month outcome, the consent record |
| I2 | Closed | (a) `:1033`, "6 months after its first charge"; (b) `:1049-1051` (P4) |
| I3 | Closed | `:1046-1048` (P3); `:1139` (one trial per account). The day count is now wrong, see M-1 |
| I4 | Closed | `:1040-1045` (P1, P2) |
| I5 | Closed | `:1059-1060` (P7); `:1145`. The window's floor is in M-3 |
| I6 | Closed | `:1056-1058` (P6). Its interaction with P3 is in I-C |
| I7 | Closed | `:1006-1008`, `:1137-1158`. One remaining entry point is in M-5 |
| I8 | Partly closed | `:1141-1145` (price id, amount billed, `academic_year` rows). Billing cadence and the kept `monthly` label are missing, see I-B |
| I9 | Closed | `:1010-1011` |
| I10 | Closed | `:1013-1014`, `:1079-1080` ("tests it" is now defined), `:1162-1172` |
| I11 | Closed | `:1116-1122`. The consent log is in M-7 |
| I12 | Closed | `:1145` |
| I13 | Closed | `:1061-1062` (P8), `:1142` |
| M1 | Closed | `:1001-1004` |
| M2 | Closed | `:1005-1007` |
| M3 | Closed | `:1082-1083`. The yearly multiple is in M-2 |
| M4 | Closed, but the answer is now wrong | `:1070-1071` (P11), see I-A |
| M5 | Closed | `:1100-1101` |
| M6 | Closed | `:1091`, `:1108-1111` |
| M7 | Closed | `:1067-1068` (P10) |
| M8 | Closed, with a new gap | `:1041-1042` (P1), see I-D |
| M9 | Closed | `:1068-1069` (P10). The pause for grandfathered subscriptions is in M-6 |
| M10 | Closed, but the instant is undefined | `:1052-1053` (P5), see I-C |
| M11 | Closed | `:1054-1055` |
| M12 | Closed | `:1066` |

## Arithmetic (recomputed)

Serving is $0.60 × 12/13 = $0.5538 per 4-week charge, and $7.20 a year.

| Figure | Recomputed | Draft |
|---|---|---|
| 4-week $12.99 | 12.99 − 0.6767 − 0.5538 = 11.759 → $11.76, 90.5% → 91%; at 7.1%: 11.214, 86.3% → 86% | ✓ |
| Yearly $89.70 | 89.70 − 2.9013 − 7.20 = 79.599 → $79.60, 88.7% → 89%; at 7.1%: 75.831, 84.5% → 85% | ✓ |
| Pell 4-week $6.49 | 6.49 − 0.4882 − 0.5538 = 5.448 → $5.45, 83.9% → 84%; at 7.1%: 5.175, 79.7% → 80% | ✓ |
| Pell yearly $44.85 | 44.85 − 1.6007 − 7.20 = 36.049 → $36.05, 80.4% → 80%; at 7.1%: 34.166, 76.2% → 76% | ✓ |
| "about $0.55 per 4 weeks" | $0.5538 | ✓ |
| 13 × $12.99 | $168.87 | ✓ |
| Kept over a year at 2.9% | 13 × 11.7594 = $152.87 (or 168.87 − 8.797 − 7.20) | ✓ |
| Yearly saving | 168.87 − 89.70 = $79.17; 79.17 / 168.87 = 46.9% → 47% | ✓ |
| Pell yearly at double serving | 44.85 − 1.6007 − 14.40 = 28.849, 64.3%; at 7.1%: 26.966, 60.1% | ✓ (64%, 60%) |
| 75% price against 50% | 4-week at $3.25: kept 3.25 − 0.3943 − 0.5538 = 2.302; 5.448 / 2.302 = **2.37×**. Yearly at $22.43: kept 14.280; 36.049 / 14.280 = **2.52×** | "about 2.4 times" (M-2) |
| Trial cost | 0.60 × 14/30 = $0.28; 7 days, $0.14 | ✓ |
| $9.99 × 9 | $89.91, 21¢ above $89.70 | ✓ |
| **"$8.97 every 4 weeks, June and July free"** | $89.70 / $8.97 = 10 charges of 13: 3 periods (12 weeks) free, not 2 months | **wrong (I-A)** |
| **P11's "$4.49 every 4 weeks"** | $44.85 / 10 = 4.485 (the 10-of-12-months framing); a year has 13 periods; $4.49 × 13 = $58.37 | **wrong (I-A)** |
| P3's "about 44 days" | 14 + 28 = 42 under 4-week billing | wrong (M-1) |

## Important

### I-A. The yearly display is false once the short plan bills every 4 weeks (ruling (2), `:1020-1022`; P11, `:1070-1071`)
- **The conflict:** "$8.97 every 4 weeks, June and July free" carries over the old monthly framing (10 paid months of 12, × $8.97 = $89.70). A year holds 13 four-week periods. Ten charges of $8.97 leave three periods, 12 weeks, free, not June and July. A student who takes the line at its word gets 304 paid days, which is 10.86 periods × $8.97 = $97.40, not $89.70. P11's Pell example has the same fault: "$4.49 every 4 weeks" is $44.85 / 10, so over 13 periods it shows $58.37 for a $44.85 plan, and P11 does not carry the "June and July free" qualifier at all.
- **Scenario:** Every yearly buyer sees this line first (ruling (2) puts it first). It is a per-period price and a "free" claim that do not match the billed amount. The lawyer item (`:1129-1130`) checks the "free" framing against the FTC's guides, but not this arithmetic, so the error can survive the lawyer's read.
- **Why it is Quinn's:** the display text is part of the ruling as recorded. **Recommendation:** show "$6.90 every 4 weeks, $89.70 billed once a year" and, for Pell yearly, "$3.45 every 4 weeks, $44.85 billed once a year". Both divide exactly by 13 (6.90 × 13 = 89.70; 3.45 × 13 = 44.85), so P11's rounding rule becomes moot and the FTC "free" item drops out. If Quinn wants the summer framing kept, the true form is "$8.97 every 4 weeks, 12 weeks of summer free", and P11's example becomes "$4.49 every 4 weeks, 12 weeks free" (rounded up as P11 says).

### I-B. The consequences do not carry the 4-week cadence into the reminder, or keep the grandfathered `monthly` label apart (`:1141-1145`; P10, `:1068`)
- **Code:**
  - `billing-jobs/handler.ts:71-74`. `priceOf` writes the cadence as well as the amount ("$9.99 per month"), and its text goes into the California-required annual reminder (`:84`).
  - `console.js:2219` shows `entitlements.plan` raw in Settings.
- **The draft says:** only "`priceOf` reads the amount actually billed".
- **What it leaves open:** P10 keeps old subscriptions at their old price, so a real $9.99 `monthly` row survives beside the new 4-week plan.
- **Scenario A:** A planner keeps the label `monthly` for the 4-week price, to avoid migrating rows. A 4-week student then gets the annual reminder reading "$12.99 per month", and Settings shows "monthly". Both break ruling (1)'s "never 'a month'", and the first misstates "how often" in the reminder ARL requires (`site/terms.html:36`).
- **Scenario B:** A planner maps every old row to the new plan. A grandfathered subscriber is then told "$12.99 every 4 weeks".
- **Fix, at `:1144`:** "`priceOf` reads the amount and the interval actually billed ('every 4 weeks', 'per month', 'per year')". At `:1142-1143`, add: "the 4-week plan's label is not `monthly`; rows on a kept $9.99 monthly price stay `monthly`."

### I-C. P5's and P6's claims have no instant, and P3 can block P6's re-check (`:1046-1058`)
- **P1** defines the claim instant only for Checkout (`:1040`).
- **P5** says a Settings claim "applies from the next charge, with the same 30 days", but not 30 days from when.
  - Scenario: a $89.70 yearly subscriber claims in month 3, and their next charge is 9 months away.
  - Counted from the claim, the 30 days end before any Pell charge exists, so (6)'s "first charge" has nothing to apply to.
  - Counted from the charge, a student pays $44.85 for a year that (6) then shortens, 9 months after they asked.
- **P6** treats the first charge after "verified through" as "a new claim with its own 30 days".
  - P3 says "after a claim lapses, the Pell price needs an accepted letter first", and that is per account.
  - Scenario: a student whose first claim lapsed in 2026, who was later proved, reaches the July 2027 re-check. Read literally, P3 denies them the re-check's 30 days. Read the other way, P6 is a second unproven claim.
- **Fix:**
  - P5: "A Settings claim's instant is the claim. If the deadline passes before the next charge without an accepted letter, that charge is at the regular price and the claim has lapsed (P3)."
  - P6: "The re-check's instant is that charge, and a re-check is not the unproven claim P3 counts."

### I-D. P1 stops the reminders on cancellation, so a cancelled Pell yearly loses half its year without notice (`:1041-1042`)
- **The case:** Cancelling a yearly plan stops the renewal and leaves the year running (`:1123-1124`). P1 stops the reminders when the subscription "is cancelled". P7's notice fires only before a renewal.
- **Scenario:** A student claims Pell yearly and pays $44.85 on day 14. On day 20 they turn off renewal in the portal. They get no day-23 or day-29 reminder and send no letter. The plan then ends at month 6 rather than month 12, and nothing after day 0 told them. That is within (6)'s letter, but the reminders exist for exactly this case.
- **Fix:** P1 reads "…or the subscription ends; a cancellation that leaves a paid period running stops nothing." On a shortened yearly that will not renew, P7 sends an access-end notice in place of the renewal notice.

## Minor

- **M-1. P3's "about 44 days" (`:1047-1048`)** is 14 + 30 from monthly billing. Under 4-week billing it is 42.
- **M-2. "about 2.4 times on what is kept" (`:1082-1083`)** holds for the 4-week plan (2.37×). Yearly is 2.52×. Say "about 2.4 to 2.5 times".
- **M-3. P7's "at least 7 days" (`:1060`)** reads as a floor a planner can build to. California's renewal-notice windows for a one-year term may be longer than 7 days, as I understand them; the lawyer confirms. Say instead: "the lawyer sets the window; it is configurable, and nothing ships before it is set".
- **M-4. The Stripe interval is not named.**
  - A 4-week price is `recurring[interval]=week`, `interval_count=4`.
  - Today the webhook maps only `month` and `year` (`stripe-webhook/handler.ts:67`). A 4-week subscription therefore lands with `plan: null`, and `billing-jobs` counts it `failed` every day and never reminds it (`billing-jobs/handler.ts:119-123`).
  - The price-id rule (`:1141`) fixes this. Name the interval in `:1138`, so the plan has a test for it.
- **M-5. `console.js:3908`** sends the wizard's plan id (`"academic_year"` / `"monthly"`) and is not named beside `#up-subscribe` (`:1151`). The plan-name change has to reach it too.
- **M-6. P10 and the pause (`:1068-1069`).** "Keeps its price until cancelled" while the pause goes away is a change to what a $9.99 monthly subscriber was sold (`site/terms.html:28`). State that no outside subscriber exists before the switch, since 10-16b's gate (`:816-823`) blocks outside charges. Otherwise, name the notice they get.
- **M-7. The consent log keeps the Pell claim (P9, `:1065`).** That log is kept 3 years. The privacy open item (`:1116-1122`) recommends keeping only "verified through" and the award year. Name the consent log there, so the privacy page states that the claim itself is kept 3 years.
- **M-8. P4's restore reverses the shortening (`:1049-1050`).** The spike (`:1112-1115`) should prove both directions: moving the period end in, and moving it back out.
- **M-9. P2's new deadline has no notice (`:1044-1045`).** The reminders run on days 23 and 29 of the original deadline. Say that the rejection email names the new date and what happens next, as ruling (5) asks of every reminder.
- **M-10. 10-16b's open item, `:830-831`,** ("The Pell price of Amendment 2026-10-16 applies on top or instead") still names the old Pell price. "What it replaces" (`:1010-1012`) should say that the first open item answers it.

Optional, for honesty rather than correctness: the "Why" paragraph compares $12.99 every 4 weeks with a research ceiling stated per month ($10–12, `:1073-1074`). $12.99 every 4 weeks is $14.07 a month, which widens the stated gap.

## Checked and clean

- **Contradictions between P1–P11 and rulings (5)–(7):** none beyond I-C (P3 against P6) and I-D (P1 against (6) on a cancelled yearly). P2's held deadline is consistent with (6)'s "after the deadline". P8 refines (6)'s "never cuts off access", and does not contradict it.
- **"Every 4 weeks"** is in the title, (1), (4), the table and the open items. The lawyer item names "the every-4-weeks billing and how it is shown" (`:1129`).
- **Section references:** §5.1's Billing line (`:135`), D2 (`:14`), R2 (`:341`), R3 (`:342`) and 10-16b's gate (`:823`) are right, as are the waitlist design's §3.1 (`:227`), §9.1 (`:1112`, bullet `:1121`), §6.1 (`:863`) and W7.
- **Calendar-month code:** `judge_caps.ts` and `judge_pipeline.ts` use "monthly" for the calendar-month inference ceiling, not billing, so nothing there changes.

## Must be fixed before signing

1. I-A, in whichever form Quinn picks (below).
2. I-B: the cadence and the kept `monthly` label, at `:1142-1144`.
3. I-C: the P5 and P6 instants, and P6's exemption from P3.
4. I-D: what stops P1's reminders, and the access-end notice.
5. M-1 to M-10, all text-only. M-1 and M-2 are wrong figures.

## For Quinn

1. **The yearly display (I-A).** "$8.97 every 4 weeks, June and July free" does not add up over 13 periods. **Recommendation:** "$6.90 every 4 weeks, $89.70 billed once a year" (and $3.45 for Pell yearly). It is exact and makes no "free" claim. Otherwise use "$8.97 every 4 weeks, 12 weeks of summer free".
2. **A Settings claim on yearly (I-C).** **Recommendation:** the 30 days run from the claim. Without a letter by then, the renewal is at $89.70 and the claim has lapsed. A student who claims mid-year is then never charged $44.85 for a year that is later halved.
3. **The re-check and the one unproven claim (I-C).** **Recommendation:** the yearly re-check does not count as the one unproven claim. An already-proven Pell student gets the 30 days each year.
