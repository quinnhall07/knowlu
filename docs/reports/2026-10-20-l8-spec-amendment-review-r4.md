# L8 friends-testers spec, Amendment 2026-10-20: review r4 (delta)

Spec: `docs/specs/2026-10-18-friends-testers-design.md`, revised after `docs/reports/2026-10-20-l8-spec-amendment-review-r3.md`
(FAIL 0C/2I/4M). Reviewer, spec mode, delta only. Filed by the controller from the reviewer's returned text.

**Verdict: FAIL (0 critical, 1 important, 2 minor).** All six r3 findings are closed. D2 holds: the price buttons are
hidden only for an entitled account (:368-369, pin :1355-1356), and `checkEntitled()` answers false on any failure
(`app/static/console.js:39-43`), so an unpaid student, or one whose check failed, still sees the prices. Step 5b says
who acts and when. The three `WIZ.entitled` sites the spec names are the real ones (`console.js:2814`, `:3052`,
`:3128`), but the on-entry check this revision added is a fourth, and the spec leaves it out.

## Confirmed (the six)

- **I-1:** closed. Amendment block :39-46; check 1b :811-814; the C1b weighing leaves 1b out at :859-860; S7 :1411.
- **I-2:** closed. Step 5's limit :831-834; step 5b :835-837 (no sooner than 24 h; the ledger row closes after it);
  Q1 :636-640; C1b weighing :857-858; S7 :1411; the staging proof needs no 5b (:1367).
- **M1:** closed (§3.4 :368-371; pin :1355-1356). **M2:** closed as worded (:389-394, pin :1357-1358). **M3:** closed
  (:380-383; Q1 :621-623; S8 :1412). **M4:** closed (:395-397, used at :468-469, :623, :829, :1366, :1411; :1393;
  :466-467).
- **Rule 1:** no personal identifier in the new text.

## Critical

None.

## Important

1. **The on-entry check is a fourth place `WIZ.entitled` is set, and the spec pins it to the plain boolean.** §3.4 :368
   says that on entry the step asks `checkEntitled()` first, and the S5t pin requires exactly that call (:1349); the
   same revision says the plan is recorded "wherever `WIZ.entitled` is set", with the helper "at all three" sites
   (:389-392), and pins that every such place uses the helper (:1357-1358). `checkEntitled()` never returns the plan
   (`console.js:41`). *Scenario:* a tester relaunches onto the subscribe step after the grant (:384-385); the on-entry
   `checkEntitled()` is true with no plan, so S5 either renders the paid form (both prices, "your card is taken now",
   "Your subscription is active.", `console.js:2693`: r2's I-3 back on the relaunch path) or waits for a plan that never
   comes. The two pins contradict each other. *Fix:* the on-entry check uses the status-and-plan helper; "all four" at
   :390-392; :1349 pins the helper on entry.

## Minor

1. **The S5t and S5 task rows lag §9 and §3.4.** S5t's row (:1407) omits the price-buttons and helper pins; S5's row
   (:1408) hides the buttons only "for a `tester` plan", while :368-369 and :1355-1356 hide them for any entitled
   account. *Fix:* bring both rows in line with :1355-1358 and :368-369.
2. **Step 5b sets a start time but no deadline** (:835). Q1 claims a trial found this way is cancelled before its first
   charge (:640-641), but nothing makes 5b run within the trial. *Fix:* 5b runs within 24 to 72 hours of the grant;
   word :640-641 and S7 :1411 to match.
