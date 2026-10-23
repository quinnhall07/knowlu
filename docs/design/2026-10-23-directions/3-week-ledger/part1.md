# Direction 3: The week as a balance (`week-ledger`)

Brief for the prototype. Written 2026-10-23. Starts from Notion + Stripe; third reference **Shovel**
(semester time arithmetic, the cushion). Bold direction. Sample data: `../shared/sample-student.json`
(Maya Okafor, Halverson State, all invented). Every figure below is computed from that file; where a
figure in `../concepts.md` or the run's direction card disagrees with the data, the data wins and the
difference is named in §0.

Product truth: `../research/product-truth.md`. The ruled spec
(`docs/specs/2026-10-23-onboarding-redesign-draft.md`) is not re-argued here. The direction changes
only what is presented, when the week becomes visible, and what the main page is.

---

## 0. Thesis, and three corrections the data forced

**Thesis.** Under "what's next?" sits the question students actually lose sleep over: *do I have
enough time?* This direction answers it from the first proof onward. A statement of the student's
time opens beside the school step and fills line by line as sources land: work comes in, time
already spoken for goes out, turned-in work is struck through. Payment's order summary is the
statement so far. Your week reconciles it. The reveal is the week's statement. The main page is
**today's budget**: one balance, then the line items in Knowlu's order, then each domain as an
account.

**What it refuses.** The category default for a planner is a to-do list with a calendar beside it.
Here the list is still the product (VISION), but every line carries its hours, every count carries
its hours, and the page opens on whether the day balances.

**Corrections from the data (read before building):**

1. **School proof hours.** The card says "+26 assignments, about 31h". The data says Blackboard
   returned 26 assignments for this term, 4 already turned in, so **22 to do, about 41.5h** once
   effort is estimated.
2. **Estimates arrive last.** In the sample run, effort is estimated at 19:18-19:19, after the
   syllabi, not at the school proof (19:08). The ledger therefore **shows counts first and hours
   when they exist**: the hours column reads "estimating" until `judge` has run for that source.
   It never shows a guessed hours figure. Open question for Quinn (§9, Q1): can `judge` run per
   source as each lands, so the hours settle during onboarding rather than in the wait?
3. **Today's balance.** The card says "must-do about 4.7h against about 5.7h free". The read model
   says: today has **about 12.2h open** (wake to bed, minus commitments); Knowlu planned **6.5h of
   work** (4.7h must-do plus 1.8h recommended); **about 5.7h is yours after that**, from 6:00 pm.
   The governing figure is must-do 4.7h against **6.5h of work room**, and 5.7h is what remains free.
   Putting 4.7 against 5.7 would compare work with time left *after* that work, which is wrong.
4. **Friday, 10.2h.** That is Friday, October 30 as seen on Monday, October 26 (the main page). At
   the reveal (Sunday, October 18) the same Friday holds **about 11.25h due** (progress since then
   brought it to 10.2h). The reveal calls out "Friday, October 30: about 11h due"; the main page
   calls out "about 10.2h".

---

## 1. The ledger: the one object this direction is built on

A **ledger** is a statement of the student's time for a period. It is a render of the read model,
never stored in the UI, and it has the same anatomy everywhere it appears (onboarding pane, payment
summary, the reveal, today's budget, a domain's account). Notion's lesson: one object, many renders
(research/notion.md §2.1).

### 1.1 Line anatomy

```
[tint] Source or account     what it is, in words                 count     hours
```

- **Tint swatch**: a small square in the course or domain tint (identity, never state). Lines with no
  course (a source, a total) have none.
- **Label**: the source ("Blackboard") or account ("Organic Chemistry") in text weight 500.
- **Description**: one short phrase in the student's terms ("22 to do, due through December 4").
- **Count**: tabular, right aligned. Every count has hours beside it (anatomy §3.6).
- **Hours**: tabular, right aligned, one decimal, `h` suffix ("41.5h"). The column header says
  "Hours, estimated" once, so each figure needn't repeat "about". In sentences, "about 41 hours".

### 1.2 Line states (a word and a shape each; colour is never the only signal)

| State | Shape | Text | Meaning |
|---|---|---|---|
| Posted | solid figure | the numbers | read and settled |
| Reading | figure replaced by a three-dot leader `···` that pulses at 1.2 s, reduced motion: static | "reading" | the source is being read; the student may move on |
| Estimating | count shown, hours column shows `···` | "estimating" | counted, effort not judged yet |
| Struck | the line drawn through, figures kept | "turned in, archived" | was work, is not; never a task |
| Pending (skipped) | dashed rule above the line, figures empty | what it would have added | skipped source, its cost named |
| Needs you | a notch cut into the left edge of the line, label in accent ink | "needs you" + the fix | a sign-in to redo, a partial grant |
| Committed | small ruled bracket at the left | "confirmed" | a class or commitment the student confirmed on Your week |

Struck lines carry hours too, so the student sees how much the turned-in filter took away ("4 turned
in, archived · 4.0h").

### 1.3 The perforation

Every ledger has one perforated rule. Above it, **posted** lines; below it, **reading**, **pending**
and **needs you** lines. As lines settle they move up across the perforation (a 200 ms slide). The
perforation is the only decorative edge in the product and it carries information: it is the line
between "known" and "not known yet" (principle 7). When nothing is pending, the perforation is the
bottom edge of the statement and the space below it is empty.

### 1.4 Totals

Totals sit under a ruled line (1px rule, then the total row in weight 600). Double rule only under
the one figure the ledger exists to answer (the week's open time, today's balance). Totals never
appear while a line above them is still reading; instead the total row says "so far" after its label
("Due this week, so far").

---
