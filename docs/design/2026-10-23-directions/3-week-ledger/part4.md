
---

## 5. `O12` Here's your week: the statement

The one authored motion moment: the ledger pane widens to the full window (420 ms, exponential
ease-out; reduced motion: a 120 ms crossfade) and becomes the week's statement. The left pane's last
proof folds into the statement's header. One button at the foot: **Open Knowlu**.

Top to bottom:

1. **Heading:** Here's your week.
2. **What's next** (the first line item, larger than the rest): Reading: Ch. 8, Metabolism is next.
   Due tomorrow at 9:00 am, before BIO 201. About 30 minutes. *Why first:* It's the first thing due
   this week.
   Second line: About 3h free tonight, after 9:00 pm.
3. **The week, day by day** (seven columns, Monday to Sunday). Each column: weekday and date; **due**
   (hours of work due that day) and **open** (hours awake minus commitments); a stack of 30-minute
   blocks for the due hours, drawn against a faint ruled box for the open hours; the day's class and
   commitment blocks listed under it in small type with their tints.

| | Mon 19 | Tue 20 | Wed 21 | Thu 22 | Fri 23 | Sat 24 | Sun 25 | Week |
|---|---|---|---|---|---|---|---|---|
| Due | 1.5h | 2.0h | 2.0h | 4.0h | 4.0h | 0h | 0h | 13.5h |
| Open | 13.1h | 9.5h | 11.3h | 9.9h | 14.3h | 12.0h | 15.0h | 85.1h |

   A column label under Tuesday and Thursday names what takes the time ("Library job 5:00-9:00 pm";
   "BIO 201 lab 2:00-4:50 pm"). Due is never presented as "do it that day": the sentence under the
   table says so.
4. **Sentence under the table** (generated, from `ahead`): Next two weeks: Friday, October 30 is the
   heaviest day, about 11h due. Knowlu spreads it over the days before it; your list each morning
   shows that day's share.
5. **The cushion** (one figure, double-ruled): **Cushion: about 2 days.** Under it: Your tightest
   deadline still has about 2 days of room. Status word beside the figure: steady.
   - Definition: the engine's `runway_days` (`engine/src/render.rs`), the minimum slack over dated
     tasks, with `status_word` (behind < 0 < tight ≤ 1 < steady ≤ 4 < ahead). This is Shovel's
     per-task cushion taken at its tightest point, counted to **due**, never shown as a start date.
   - **Why days, not hours.** Summing hours over the two weeks gives about 94h of study room against
     about 41.25h of work due, a "cushion" of about 53h. That number is true and useless: it hides
     the Friday wall and invites the student to read 53 free hours into a week that has a 11h day in
     it. The tightest deadline is the honest number.
   - The sample file carries the status word ("steady") but no runway figure. The prototype shows 2.0
     days, inside the steady band, and the brief flags it (§9, Q2).
6. **The full statement** (collapsed to its totals; "Show every line" opens it):

```
Work coming in                                      35 to do      63.8h
  Blackboard 22 · zyBooks 5 · VHL 4 · from your syllabi 3 · Gmail 1
Already done, archived (struck)                      9           8.0h
Past items, archived (struck)                        52
Due this week                                        9            13.5h
Due next week                                                     27.8h
Open this week                                                    85.1h
════════════════════════════════════════════════════════════════════════
Cushion                                              about 2 days, steady
```
7. **What Knowlu tidied** (one line each): 52 past items and 9 already turned in were archived. ·
   Effort estimated for 34 assignments. · 2 classes from last term left out.
8. **From your email:** Pay Pre-Health Society dues ($15), due Friday, October 30 at 5:00 pm. *Undo*
9. **Still coming:** Older school email is still coming in; anything it finds will be proposed. Skipped
   sources would be listed here as their pending lines, each with *Connect* (none for Maya).
10. **Button:** Open Knowlu. Under it: Knowlu checks again at 12:00 pm and 6:00 pm. It starts with
    Windows.

---

## 6. `M1` The main page: today's budget

Monday, October 26, 8:15 am (term week 10). Window 1280x800. Two columns: the budget and its line
items (left, about 720px, a 60-75 character title measure), the accounts and the ambient column
(right, about 400px). The left column alone passes the 11 pm test.

### 6.1 Topline (ambient, one line)
Monday, October 26 · week 10 of 16 · Up to date, 6:04 am · **VHL needs you** (notched marker, opens
"VHL signed you out. Sign in again to bring in new VHL work; what Knowlu already has is still here.").
From `topline` and `sources`. Under it, the delta line only when it has items (`delta`): Since 6:00 pm
yesterday: Pre-lecture quiz: Ch. 7 marked done by Knowlu. *Undo* · BIO 201 Lab 4 graded 44/50 · a
new question about Lab 8.

### 6.2 The governing figure: today's balance (`verdict`, `meter`, `free_time`)
- **Sentence first** (the verdict, never a fixed string): About 4.7h is due by tomorrow, and today
  has room for it. Lab 6 is next. After today's work, about 6h is yours, after 6:00 pm.
- **The balance** (five lines, tabular, ruled):

```
Open today, 8:15 am to 11:30 pm, after classes        12.2h
  Due today or tomorrow                         4.7h
  Recommended                                   1.8h
Work planned                                           6.5h
════════════════════════════════════════════════════════
Yours, from 6:00 pm                                    5.7h
```
- **The meter**: one row of 24 blocks of 30 minutes (open time). 10 solid blocks (must-do), 3 hatched
  blocks (recommended), 11 open outlined blocks (yours), with a ruled bracket marking the work room
  (13 blocks). Spill, when must-do exceeds the room, draws **outside** the bracket as outlined,
  diagonally hatched blocks in graphite, never red; the sentence then says it plainly ("About 6h is
  due by tomorrow, and today has about 5h of room.") and offers nothing to cut. Each block is
  focusable and hoverable and names its tasks ("Lab 6: Functions, 1h 12m"). Screen readers get the
  balance table, not the blocks.
- **The one comparison** (Stripe): "about 1h more than a usual Monday" appears only once there are 3
  Mondays of history. Maya has 1, so it is absent. Never a trend line.

### 6.3 Line items (`must_do`, `recommended`, `today.list`)
Group header rows always carry count and hours. Rows within a group sit tight (4px); groups are
separated by 24px (Notion's adjacency rule).

- **Next** (larger, leads): Lab 6: Functions · CS 110 · was due last Friday at 11:59 pm · 1h 12m left
  (40% done in zyBooks) · late label "−30% now; closes Wednesday at 11:59 pm" · *Why first:* Past due
  since Friday. Each day late costs 10%; finishing today keeps it at −30%. Actions: Done · Lock in ·
  Pin · open.
- **Due today or tomorrow · 5 · 4.7h** (Lab 6 included above, so the group lists the next 4):
  zyBooks 7.1-7.4 participation · CS 110 · today at 11:59 pm · 24m · VHL Lección 6: Práctica de
  vocabulario · SPAN 202 · today at 11:59 pm · 36m · Problem set 7 · CHEM 231 · tomorrow at 9:30 am ·
  1h 30m · Peer review: outline exchange · ENG 210 · tomorrow at 12:30 pm · 1h.
- **Recommended · 1 · 1.8h**: Study for Exam 2 (Ch. 5-7) · CHEM 231 · Thursday at 9:30 am · 1h 48m
  today of 4h left · steps as indented toggle children.
- **Done today · 1 · 0.5h** (struck, quiet, uncheckable): Reading: Ch. 9, Cellular respiration.
- **Late window closed · 2 · 3.0h** (only rendered when non-empty): Problem set 6 · CHEM 231 · Late
  work not accepted after 48 hours (your syllabus) · *Archive* · *Still doing it*. VHL Lección 5:
  Vocabulario · SPAN 202 · *Archive* · *Still doing it* ("When will you turn it in?").
- **Later · 20 · about 38h**, collapsed, plus "the rest of Exam 2 study, 2.2h". Opens to the full
  list by due date; anything can be checked off.
- **Closed this week · 8**, collapsed: each with who closed it (you or Knowlu) and *Undo* on Knowlu's.
- Row anatomy: check circle · title · tint swatch + course code · due with time · hours (tabular,
  right). Every field edits in place; Knowlu's original estimate, the why and the source are visible
  in the row's detail but not editable. No start-by, no assigned time.

### 6.4 Accounts (`domains`), the right column's first block
Each domain is an account line: tint, name, status word with a shape (ahead: open bracket up; on
track: level rule; behind: open bracket down; never red), hours, and the way back as a sentence.

- Organic Chemistry · **behind · about 2h** · Today's study session and one Wednesday catch it up
  before Thursday's exam.
- Programming · **behind · about 1h** · Lab 6 is past due. Finishing it today keeps it at −30%.
- Spanish · **ahead · about 1.5h** · You're about a day ahead in Spanish: two of this week's VHL sets
  are already done.
- Cell Biology · on track · Lab 5 report is the big one this week.
- Science Writing · on track · The literature review may move to Monday; there's a question waiting.
- Library job · Tuesday 5:00-9:00 pm, Saturday 10:00 am-2:00 pm · 8h this week.
- Pre-Health Society · Dues due Friday at 5:00 pm. A panel on Wednesday is waiting for your answer.
- Personal · Bus pass by Saturday at noon.

Clicking an account filters the line items to it and opens its statement (grade ring and pending
items from `grades`, exams listed plainly, work left this week in hours).

### 6.5 The rest of the right column
- **Needs you · 4 waiting · oldest 7 days**: one card at a time (`decisions`), oldest first, evidence
  quoted, answers as buttons, *Not now* sends it to the back. First card: Add "SURF application" to
  Personal, due Friday, November 6 at 5:00 pm, about 2h? · Add it · Not for me.
- **Today's schedule** (committed time): BIO 201 lecture 9:00 am · SPAN 202 10:00 am · CS 110 1:00 pm
  · CHEM 231 office hours 3:30 pm (optional, not counted).
- **Next two weeks** (`ahead`): 14 thin bars of hours due, exams marked with a word, the generated
  takeaway under it: Friday is the heaviest day in the next two weeks, about 10.2h due.
- **Good to know** (only when non-empty): CHEM 231 Exam 2 on Thursday is in Hallett Hall 120, not the
  usual room. · BIO 201 Lab 4 report graded: 44/50. · CHEM 231 Problem set 6 is marked missing in
  Blackboard.

### 6.6 Named states
- **All clear:** Nothing is due today or tomorrow. The day is yours. (Balance: due 0h; the meter is
  all open blocks.)
- **More due than room:** the spill state in §6.2. Plain, no advice.
- **Ahead for the week:** You're ahead for the week. Stopping here is fine.
- **Offline:** Offline. Changes stay on this computer and sync when you're back.
- **A source broke:** cause and fix, with when Knowlu tries again ("Blackboard stopped answering.
  Knowlu will try again at 12:00.").

---

## 7. Visual world: a bursar statement crossed with a bookstore receipt

**Rendition check.** Ground is cool white, never cream or paper-warm; dark mode is deep graphite,
never near-black with a neon accent. Red appears only in the wordmark.

### 7.1 Type
- **Schibsted Grotesk** (Google Fonts) for all text: 400, 500, 600. Fixed rem scale, ratio 1.2:
  12 / 14 / 16 (body) / 19 / 23 / 28 (the reveal heading only).
- **Azeret Mono** (Google Fonts) for figures only: hours, counts, times in columns, money. Used for
  measurement, never as a "technical" costume; labels and sentences stay in Schibsted.
  `font-variant-numeric: tabular-nums` on both faces wherever numbers align.
- Line items 15/22; ledger lines 14/20; the next task's title 23/28 weight 600.

### 7.2 Palette as tokens (OKLCH, Stripe's step rule)
One lightness ladder for every hue, 11 steps:
`L0 0.985 · L1 0.95 · L2 0.90 · L3 0.82 · L4 0.73 · L5 0.64 · L6 0.55 · L7 0.46 · L8 0.37 · L9 0.28 · L10 0.20`.
**Contrast rule, written into the token file:** small text sits at least **5 steps** from its
surface; icons, large text and the meter's outlines at least **4 steps**. Dark mode uses the same
ladder reversed, so both modes keep contrast by construction (stripe.md §4.1). Verify every pair at
4.5:1 / 3:1 in the build.

```
/* light */
--ground:      oklch(0.985 0.004 250);   /* cool white */
--paper:       oklch(1 0 0);             /* the statement surface */
--ink:         oklch(0.22 0.014 255);
--ink-2:       oklch(0.44 0.012 255);    /* secondary text, still 7:1 on paper */
--rule:        oklch(0.88 0.006 255);
--rule-total:  oklch(0.30 0.012 255);
--accent:      oklch(0.46 0.13 262);     /* interactive ink blue: buttons, focus, needs-you label */
--brand:       oklch(0.553 0.153 23.5);  /* #BC4444, wordmark only */
--tint-bio:    oklch(0.93 0.045 155);  --tint-bio-ink:  oklch(0.44 0.10 155);
--tint-chem:   oklch(0.93 0.045 305);  --tint-chem-ink: oklch(0.44 0.11 305);
--tint-cs:     oklch(0.93 0.045 230);  --tint-cs-ink:   oklch(0.44 0.10 230);
--tint-span:   oklch(0.93 0.055 80);   --tint-span-ink: oklch(0.46 0.10 80);
--tint-eng:    oklch(0.93 0.045 120);  --tint-eng-ink:  oklch(0.44 0.09 120);
--tint-other:  oklch(0.93 0.012 255);  --tint-other-ink: oklch(0.44 0.02 255); /* job, club, Personal */

/* dark */
--ground: oklch(0.19 0.010 255); --paper: oklch(0.23 0.010 255);
--ink: oklch(0.95 0.006 255);    --ink-2: oklch(0.78 0.010 255);
--rule: oklch(0.36 0.010 255);   --rule-total: oklch(0.82 0.010 255);
--accent: oklch(0.78 0.11 262);  --brand: oklch(0.62 0.15 23.5);
/* tints: fill at L9 (0.28) chroma 0.05, ink at L2 (0.88) chroma 0.08, same hues */
```
Course tints are identity, never state. No hue near red (none between 0 and 60). Status is carried
by shape and word (§1.2, §6.4); broken sources use the notch shape and the accent ink, not red.

### 7.3 Material and density
- The statement is a sheet of `--paper` on `--ground` with no shadow; its top and bottom edges are
  straight; the **perforation** (§1.3) is a row of 3px round cut-outs on a 8px pitch, the one place
  the receipt material shows. No cards, no nested boxes, no coloured side borders.
- Rules do the grouping: 1px `--rule` between groups, 1px `--rule-total` above totals, a double rule
  (two 1px lines, 2px apart) under the one answer figure.
- Density is receipt-tight in the ledger (28px lines), roomier for line items (40px rows, 56px for the
  next task). 4px base unit; 24px between groups; 40px between regions.

### 7.4 Motion (explains what changed; 150-250 ms; exponential ease-out)
- A line settling: its figure fades in with a 2px blur-to-sharp, 180 ms, and the line slides up across
  the perforation, 200 ms. **No count-up tickers**: a figure that counts up is a figure that was never
  true.
- Struck: the strike draws left to right, 240 ms.
- A total changing: the total's rule redraws, 220 ms.
- Done on a line item: the check fills and the line strikes, 200 ms; it stays for the day.
- The reveal's widening (§5) is the single authored moment.
- Reduced motion: all of the above become instant state changes; the "reading" leader is static.

---

## 8. What is taken from each reference, and where it stops

### Notion
1. **The preview that builds as you answer**, made real: Candu records a preview of the interface
   changing beside the signup form (research/notion.md §1.2; candu.ai, "How Notion crafts a
   personalized onboarding experience"). Here it is the ledger pane, real data or plainly "reading";
   the reveal is that pane growing to full width.
2. **Adjacency spacing instead of boxes**: neighbouring list items chunk tight, paragraphs get more
   room (notion.md §3.1; notion.com/blog/updating-the-design-of-notion-pages). Ledger lines and line
   items group by gaps and rules, never containers.
3. **One object, many renders** (notion.md §2.1; notion.com/blog/data-model-behind-notion): the ledger
   line is the same object in the onboarding pane, the order summary, the statement, today's budget
   and an account.
4. **Toggle children** for a task's steps (notion.md §2.2).
- **Stops at:** templates, configurable sections, user reordering (notion.md §7). Hide-until-relevant
  is used only for Late window closed and Good to know, the regions VISION lets appear on demand.

### Stripe
1. **Balances shown precisely, available and pending split** (stripe.md §2.4, [obs]; docs.stripe.com/
   dashboard/basics): here *done* and *still to do*, and *posted* and *reading*. Tabular numerals.
2. **At most one comparison** per figure (stripe.md §2.4), and only when history makes it true.
3. **Checkout's order summary** (stripe.md §3.1; stripe.com/resources/more/payment-page-template-best-
   practices): the proof is the summary above the one $9.99 button, with the exact price and how to
   cancel.
4. **Pending vs needs-you as different states** (stripe.md §1.2; docs.stripe.com/connect/handling-api-
   verification): "reading" never looks like a failure; failures are cause plus fix (§1.7).
5. **Requirements-rendered resume** (stripe.md §1.1, §1.4): `R0` reopens the first gap, never repeats
   a done step.
6. **OKLCH palette with the step-distance contrast rule** (stripe.md §4.1; stripe.com/blog/accessible-
   color-systems, Koopersmith and Miner, 2019).
- **Stops at:** dashboard density, red for negative, the error shake, trend charts on Today.

### Shovel (the third reference)
1. **Time arithmetic as the frame**: hours needed (from estimates) subtracted from hours available
   (from the calendar), giving a cushion that says ahead or behind (research/landscape.md, Shovel
   entry; shovelapp.io: the Cushion "calculates your cushion time so you know if you can meet every
   deadline").
2. **The cushion is per deadline and taken at its tightest**: Purdue's guide defines it as the
   available study time between a task's start and its deadline minus the time the task needs, drawn
   as dots above or below a zero line (purdue.edu/teaching-learning/programs/sst/assets/Shovel - The
   Cushion.pdf, read 2026-10-23). Knowlu already computes the same thing as `runway_days`; this
   direction shows it as one figure in days.
3. **Domains as accounts ahead or behind in hours** (landscape.md: "the ahead / on track / behind
   domains" precedent).
- **Stops at, deliberately:**
  - **Typed estimates.** Shovel's numbers are "only as good as" what students type (landscape.md).
    Knowlu's come from judgment and portals, and every figure says it is an estimate.
  - **Start dates.** Shovel's cushion is fixed by moving a task's start date. Knowlu shows due dates
    only (PRODUCT-DECISIONS).
  - **The fix for a negative cushion.** Purdue's guide suggests deleting something from the schedule
    to free time. That is a sacrifice suggestion, which Knowlu never makes: when more is due than
    there is room, Knowlu states both numbers and leaves the choice with the student.
  - **The cushion graph.** One figure and a word on the statement; the per-deadline dots stay on
    request (a course's statement), not on Today.

---

## 9. Open questions for Quinn (one at a time, in this order)

1. **Hours during onboarding.** Can `judge` run per source as each lands (after the school proof,
   after homework sites), so the hours column settles before the wait? Recommendation: yes; without
   it the ledger's hours all arrive in `O11`, and the payment summary shows counts only (it already
   does, honestly).
2. **The cushion figure.** Add `runway_days` to the read model's verdict payload and to the sample
   file, so the prototype's "about 2 days" is the engine's number. Recommendation: yes; the status
   word is already there.
3. **Hours framing as debt.** The risk this direction carries: a heavy week can read like a bill. The
   guard is in the voice (no red, "rest is permission", no sacrifice advice, "yours" for free time).
   Worth one round of reading the statement as a tired student before a build ships.

---

## 10. Direction contract

- **THESIS:** Every login adds lines to a statement of the student's time; the main page is today's
  budget with its line items. Refuses the to-do list with a calendar beside it.
- **OWN-WORLD:** Cool white paper statement on a cool ground (graphite in dark), Schibsted Grotesk
  text with Azeret Mono figures, ruled totals, one perforation between known and not known, course
  tints as account swatches, red only in the wordmark.
- **STORY:** The student watches their week add up as each place is read, pays with the statement as
  the order summary, confirms the week, sees whether it balances, and each morning opens to a balance
  and the next line item.
- **FIRST VIEWPORT (main page):** Left: verdict sentence, the five-line balance with its 24-block
  meter, then the next task large with its why; right: accounts in hours, one Needs you card.
- **FORM:** Pinned by `../concepts.md` direction 5 (no concept-seed roll; no seed key).
- **FINISH:** unreviewed and undocumented is unfinished; this build ends with the finish review, the
  verdict, DESIGN.md, and every shipping raster carrying its provenance.
