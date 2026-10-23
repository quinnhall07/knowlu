# Direction 3: The week as a balance (`week-ledger`)

Brief for the prototype. Written 2026-10-23. Starts from Notion + Stripe; third reference **Shovel**
(semester time arithmetic, the cushion). Bold direction. Sample data: `../shared/sample-student.json`
(Maya Okafor, Halverson State, all invented). Every figure below is computed from that file; where a
figure in `../concepts.md` or the run's direction card disagrees with the data, the data wins and the
difference is named in §0.

Product truth: `../research/product-truth.md`. The ruled spec
(`docs/specs/2026-10-23-onboarding-redesign-draft.md`) is not re-argued here. The direction changes
only what is presented, when the week becomes visible, and what the main page is.

> **Read §11 first.** The prototype (`index.html`, screens in `screens.json`, shots in `shots/`)
> was built after two red-team passes. §11 records every decision the build changed. Where §11 and
> the text below disagree, §11 wins.

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

## 2. Onboarding layout

Window 1280x800. **Two panes from the school step on.**

- **Left pane, the step** (about 560px, a 60-70 character measure): step name, the benefit line, the
  one ask, then the proof. Below the ask, one quiet line of privacy where it matters. At the foot,
  the way forward: the primary button names the next thing ("Continue to payment"), and for a
  skippable step a quiet "Skip" with its cost written beside it, never hidden.
- **Right pane, the ledger** (about 600px): titled "Your week · October 19-25". It is real data or
  plainly pending; it never shows placeholder tasks (notion.md §1.2, "does not carry over").
- **Where you are**: a single line above the step, in words: "Next: payment, then 5 more steps.
  About 7 minutes." The minutes are an honest estimate from the steps left (spec's per-step times).
  No dot stepper, no step numbers.
- Steps 1-2 (welcome, account) are one centred column: there is nothing to put in a ledger yet. At
  the school step the window splits and the ledger slides in from the right with its header only.
- Below 1100px wide the ledger collapses to a docked strip at the bottom (total lines only: "Work
  coming in 22 · estimating", "Open this week ···"), expandable.

---

## 3. Flow map

Each screen has an id. "→" is a transition, with its trigger. Background reads start the moment an
ask is answered and never block the next step (spec §1.5).

### 3.0 Launch and resume

| Id | Screen | Transition |
|---|---|---|
| `R0` | Launch router (no UI; reads `onboarding.json`) | no file → `O1`. File with a current step → that step, ledger rebuilt from the vault and finished jobs (`R1`). Onboarding finished → `M1` |
| `R1` | Resume banner on the saved step | shown once at the top of the left pane; any action dismisses it. Unfinished background jobs re-run silently; their lines show "reading" again |
| `R1-s` | Resume when the school session expired | the step opens normally; the ledger's school lines stay posted; if a later read needs the school, the school line goes to "needs you" with "Sign in to Halverson State again" (opens `O3b`) |
| `R1-p` | Resume at payment | entitlement re-checked first; already paid → advance to `O5` with the ledger's last line "Your trial is on" |

### 3.1 Steps

| Id | Screen | In | Out (trigger) |
|---|---|---|---|
| `O1` | Welcome | first launch | "Start" → `O2` |
| `O2` | Your account | `O1` | Google returns, or the emailed code is accepted → `O2p` (proof inline) → "Continue to your school" → `O3`. Code wrong → `O2-e` inline |
| `O3` | Your school: pick | `O2` | school picked (prefilled from the email domain where it matches) → `O3a`. "My school isn't listed" → `O3f` |
| `O3a` | What you'll need | `O3` | "Sign in at Halverson State" → `O3b` |
| `O3b` | School window open (Halverson State's own page, its host shown in the window title bar: `login.halverson.example`) | `O3a`, `R1-s` | session established → `O3c`. Window closed with no session → `O3-x` |
| `O3c` | Signed in; reading | `O3b` | immediate. Ledger lines appear one by one as "reading". Course list posts first (seconds) → proof line fills → "Continue to payment" enabled at once (student may go on while lines settle) → `O4` |
| `O3-x` | Sign-in didn't finish (window closed, Duo not approved, timed out) | `O3b` | "Sign in again" → `O3b`. Second failure in a row also shows "Use your Blackboard calendar link instead" → `O3f`. "Finish later" → app may quit; resume returns to `O3a` |
| `O3f` | Calendar-link fallback | `O3`, `O3-x` | link pasted and read → `O3c` with fewer lines (the costs post as pending lines) → `O4` |
| `O4` | Payment | `O3c` | "Start 7 days free" opens Stripe Checkout in the browser → `O4w` |
| `O4w` | Finishing in your browser | `O4` | Checkout succeeds → `O4p` → "Continue" → `O5`. Browser tab closed or checkout cancelled → `O4-x`. Tester → `O4-t` instead of `O4` |
| `O4-x` | Checkout didn't finish | `O4w` | "Open checkout again" → `O4w` |
| `O4-t` | Tester: nothing to pay | `O3c` | "Continue" → `O5` |
| `O5` | School email | `O4p` | covered by the school sign-in: forwarding set up and canary sent automatically → proof posts → "Continue" → `O6`. Not covered: one sign-in (same pattern as `O3a-O3b`). "Skip" → pending line → `O6` |
| `O6` | Google: Gmail and Calendar | `O5` | "Continue to Google" → Google warning screen (unverified app) → consent → `O6p`. Partial grant → `O6-g`. Declined or read failed → `O6f`. "Skip" → pending lines → `O8` |
| `O6-g` | Partial grant ("Gmail connected · Calendar not shared") | `O6` | "Share calendar" → consent again for that scope → `O6p`. "Continue without it" → `O7` |
| `O6f` | Gmail by forwarding (fallback only) | `O6` | forwarding verified → `O6p` (Gmail line). "Skip" → pending |
| `O7` | Another calendar (only when no calendar was granted) | `O6-g`, `O6f`, Google skipped | iCal link read → proof → `O8`. "Skip" → pending line → `O8` |
| `O8` | Homework sites (only sites the syllabi or Blackboard mention: zyBooks, VHL) | `O6p`/`O7` | per site: credentials panel → sign-in checked → that site's proof. Wrong credentials → `O8-x` on that site's row. "Skip VHL" → pending line. Both settled → "Continue" → `O9` |
| `O8-x` | That site didn't accept the sign-in | `O8` | "Try again" or "Skip VHL for now" |
| `O9` | Syllabi (only for courses with none found: CS 110) | `O8` | file uploaded → "reading" → proof. "Skip CS 110" → pending line. → `O10` |
| `O10` | Your week: the reconcile | `O9` | every row confirmed, edited, or left blank → "This is my week" → `O11` |
| `O11` | The wait (0 s to 3 min) | `O10` | every first-run job done → `O12`. 3 minutes pass → `O12` with "still running" lines |
| `O12` | Here's your week: the statement | `O11` | "Open Knowlu" → `M1`. Scheduler and start-with-Windows turn on here |

### 3.2 Paths named on the map

- **Duo MFA.** Happens inside `O3b`, on the school's page. Knowlu prepares the student at `O3a`
  ("Have your phone nearby"), waits without a timer while the window is open, and treats a closed or
  timed-out window as `O3-x`, never as an error that blames the student (onboarding-evidence §4.1:
  4.35% of UCI attempts failed at the Duo step). The kept school profile means Duo is asked **once**.
- **Failed school login.** Knowlu never sees the password, so it cannot say "wrong password". It sees
  only "the window closed and no session exists", and says exactly that.
- **Skip paths.** Each optional step's Skip writes a pending line under the perforation saying what it
  would have added. The reveal repeats each skip once, with Connect. Nothing skipped is nudged later
  (spec §1.7).
- **Slow first run.** `O11` waits only for first-run jobs, never backfills, and caps at 3 minutes.
- **Resume after quit.** `R0` → the same step, finished proof intact, ledger rebuilt. A student who
  quit at payment returns to payment (`R1-p`).

---

## 4. Exact copy, step by step

Benefit lines are the spec's wording (Quinn reads final copy). The proof line is what the left pane
says once the step lands; the ledger lines are what the right pane posts. Every figure is Maya's.
Mechanics follow BRAND-VOICE: sentence case, "you", contractions, "5:00 pm", no exclamation marks,
no "just", "easy", "simply", "oops", "don't worry".

### `O1` Welcome
- **Heading:** Know what's next.
- **Benefit:** Knowlu answers one question every morning: what's next? Next, you'll connect the places
  that already know your work, about 10 minutes.
- **Second line:** As each place is read, you'll see your week add up: the work coming in, and the
  time you already have spoken for.
- **Button:** Start

### `O2` Your account
- **Benefit:** Your plan lives in your account, so it's safe if this computer isn't.
- **Ask:** Continue with Google · Email me a code. Checkbox: I'm 18 or older and agree to the terms.
- **Price disclosure (always visible here):** Knowlu is $9.99 a month after 7 days free. There's no
  free tier. You'll see what Knowlu finds before you pay.
- **Code wrong (`O2-e`):** That code didn't match. Check the newest email from Knowlu, or send a new
  code.
- **Proof:** Signed in as maya.okafor.demo@gmail.example. Your plan is kept as plain files on this
  computer. *Show me the folder*
- **Button:** Continue to your school

### `O3` / `O3a` Your school
- **Benefit:** One school sign-in lets Knowlu read your classes, class times, assignments, grades and
  school email. You won't sign in to these again.
- **Ask (`O3`):** Which school? [Halverson State University, prefilled] · *My school isn't listed*
- **What you'll need (`O3a`):** Your Halverson State email and password, and your phone for Duo.
- **Privacy line:** You sign in on Halverson State's own page. Knowlu keeps the session on this
  computer, never your password.
- **Button:** Sign in at Halverson State
- **While the window is open (`O3b`, left pane):** Waiting for Halverson State. If it asks, approve
  the Duo request on your phone.
- **Ledger opens:** header "Your week · October 19-25", sub-header "Halverson State · Fall 2026, week
  8 of 16" once the course list posts.
- **`O3-x`:** The sign-in window closed before Halverson State let you in. Nothing was saved. Sign in
  again when your phone is nearby.
  Buttons: Sign in again · *Finish later*. After a second miss, also: *Use your Blackboard calendar
  link instead*. If Halverson State locked the account: Halverson State's help desk can unlock it.
  Your place here is saved.
- **`O3f`:** Paste your Blackboard calendar link. Knowlu reads due dates from it.
  Cost, said before the paste: With only the link, Knowlu can't see grades, class times from the
  registrar, syllabi, or what you've already turned in, and school email needs its own sign-in.
- **Proof (left pane):** 5 classes this term; class times for 5; 26 assignments, 4 already turned in;
  grades for 5 courses; syllabi found for 4.
- **Ledger lines posted** (in the order they land, sample times 19:07-19:09):

| Section | Line | Count | Hours |
|---|---|---|---|
| Work coming in | Blackboard · assignments this term, due through December 4 | 22 to do | estimating, later 41.5h |
| Work coming in | Blackboard · already turned in, archived (struck) | 4 | 4.0h |
| Work coming in | Blackboard · past items, archived (struck) | 29 | not counted |
| Time spoken for | Classes · 7 meetings a week, from the registrar | 7 | 16.2h a week |
| Accounts opened | Cell Biology, Organic Chemistry, Programming, Spanish, Science Writing | 5 | (one tint swatch each) |
| Below the perforation | Grades · reading → posts "5 courses" | | |
| Below the perforation | Syllabi · 4 found, reading | | |
| Below the perforation | Last term · 2 classes left out (BIO 110, MATH 141 never named) | 2 | |

  The class line is **found**, not committed, until Your week (`O10`).
- **Button:** Continue to payment

### `O4` Payment
- **Benefit:** Everything you just connected keeps running every day for $9.99 a month, after 7 days
  free.
- **Order summary = the ledger so far.** The right pane gains a ruled block at the bottom:

```
Read so far                        5 classes · 22 to do · 4 turned in
Knowlu, monthly                    $9.99 a month after October 25
Due today                          $0.00
```
- **Options line:** Or $39.99 for a 6-month term. Pell Grant recipients pay 75% less.
- **Privacy line:** You pay for Knowlu, so your data is never sold.
- **Cancel line:** Cancel any time in Settings before October 25 and you won't be charged.
- **Button:** Start 7 days free (opens checkout in your browser)
- **`O4w`:** Finish checkout in your browser. This screen moves on by itself when it's done.
- **`O4-x`:** Checkout closed before it finished. Nothing was charged. *Open checkout again*
- **`O4-t`:** Nothing to pay: you're a tester.
- **Proof:** Your trial is on. First charge October 25.
- **Ledger:** the payment block collapses into one posted line, "Knowlu · trial on · first charge
  October 25", above the perforation.

### `O5` School email
- **Benefit:** Professors change dates by email. Knowlu reads what arrives and proposes each change
  for you to approve.
- **Ask:** Nothing new: your school sign-in covers it.
- **Proof:** Forwarding on. Test message arrived in 38 s. New mail is read as it arrives; older school
  mail is still coming in.
- **Skip cost:** Skip, and date changes and new work announced by email are missed.
- **Ledger:** "School email · read as it arrives" posted; "Older school mail · still coming in" stays
  under the perforation (it is a backfill and is never waited for).

### `O6` Google: Gmail and Calendar
- **Benefit:** Clubs, jobs and internships write to your Gmail, and your calendar says when you're
  already busy. One Google permission covers both, read-only.
- **Before the button:** Google will show a warning first, because Google is still reviewing Knowlu.
  Choose Advanced, then continue to Knowlu. Gmail permission brings in the last 7 days of mail.
- **Button:** Continue to Google
- **Proof:** Read 40 recent messages; 1 thing to do (Pre-Health dues, added). 6 events this week, 3
  repeating ones to check on Your week.
- **Ledger lines:**

| Section | Line | Count | Hours |
|---|---|---|---|
| Work coming in | Gmail · Pre-Health Society dues, due Friday, October 30 at 5:00 pm | 1 | 0.25h |
| Time spoken for | Library job · Tuesday 5:00-9:00 pm, Saturday 10:00 am-2:00 pm | 2 shifts | 8.0h |
| Time spoken for | Pre-Health Society · Wednesday 6:30-7:30 pm | 1 | 1.0h |
| Time spoken for | This week only · haircut Thursday, family video call Sunday | 2 | 1.75h |
| Not counted | Intramural soccer · optional | 1 | 0h |

  The three repeating events stay **found** until `O10`. Optional events are listed as "not counted"
  so the student sees why their hours are missing.
- **`O6-g` partial:** Gmail connected · Calendar not shared. Without your calendar, your free time
  can't see your shifts or plans. *Share calendar* · *Continue without it*
- **`O6f`:** Gmail's read didn't work. Knowlu can bring Gmail in by forwarding instead, about 3
  minutes.
- **Skip cost:** Skip, and club, job and internship tasks from Gmail are missed, and your free time
  ignores your own events.
- **Pending line if skipped:** "Gmail and Calendar · would add your shifts, clubs and plans to the
  time spoken for."

### `O7` Another calendar (not shown for Maya)
- **Benefit:** If your calendar isn't Google's, add it so your free time is honest.
- **Ask:** Paste the calendar's iCal link.
- **Skip cost:** Skip, and your free time can't see your own events.

### `O8` Homework sites
- **Benefit:** Homework sites hold due dates your school's feed doesn't show.
- **Ask:** Sign in to zyBooks and VHL. Both are mentioned in your syllabi.
- **Privacy line:** These sign-ins are stored only on this computer, in Windows Credential Manager.
  (Never claim the school-SSO property here.)
- **Proof:** zyBooks: 1 book, 8 assignments, 3 already done. VHL: 1 course, 6 assignments, 2 already
  done.
- **Ledger lines:** zyBooks · 5 to do · estimating, later 8.0h; zyBooks · 3 already done (struck) ·
  2.5h; VHL · 4 to do · estimating, later 4.0h; VHL · 2 already done (struck) · 1.5h.
- **`O8-x`:** VHL didn't accept the sign-in. Try again, or skip VHL for now; its due dates will be
  missing until it's connected.
- **Skip cost (per site):** Skip, and VHL's due dates are missing unless Blackboard's feed carries
  them.
- **Pending line if skipped:** "VHL · would add your SPAN 202 practice sets and what you've finished."

### `O9` Syllabi
- **Benefit:** Your syllabus has every exam date, how each course is graded, and the late-work rules.
- **Ask:** CS 110's syllabus wasn't in Blackboard. Add the file, or skip this course.
- **Proof:** Read 5 of 5: 10 exams, weights for 5, late rules for 4 (ENG 210's syllabus has none).
- **Ledger lines:** "From your syllabi · study for 3 exams in the next three weeks · 3 · estimating,
  later 10.0h" (Knowlu's study tasks; source named "Knowlu, from your syllabi").
- **Skip cost:** Skip, and CS 110 has no exam dates beyond the feed, no weights and no late rule.

### `O10` Your week: the reconcile
- **Benefit:** Check your classes. Everything here can be changed later.
- **Ask:** one row per meeting, exam, weight set, late rule and repeating event (33 rows for Maya,
  grouped Classes, Exams, Grading, Late work, Your week, Office hours, Your day). Each row: Confirm ·
  Edit. Typing is the last resort. Confirm all in a group with one control after reading it.
- **The ledger's job here:** as each class row is confirmed, its hours move from **found** to
  **committed** (the bracket shape) and the "Time spoken for" total re-rules. When the last row is
  settled the ledger shows the reconciled week:

```
Awake this week (7:30 am to 11:30 pm)                       112.0h
  Classes, 7 meetings, confirmed                    −16.2h
  Library job, confirmed                             −8.0h
  Pre-Health Society, confirmed                      −1.0h
  This week only: haircut, family video call         −1.75h
Time spoken for                                             −26.9h
════════════════════════════════════════════════════════════════
Open this week                                               85.1h
```
- **Button:** This is my week

### `O11` The wait
- **Left pane heading:** Putting your week together.
- **Body:** Lines on the right settle as each read finishes, usually within a minute. Knowlu waits up
  to 3 minutes, then shows your week with anything still running named.
- **Ledger:** the remaining lines under the perforation settle one by one, each with its real job
  name ("Syllabi · reading CS 110", "Effort · estimating 34 assignments"); the hours column fills;
  totals appear once nothing above them reads "estimating". This is true engine progress, never a
  ticker (onboarding-evidence §1.4, §3.9).
- **At 3 minutes:** Still reading 1 syllabus; it will appear on your page. → `O12`.

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

**Rendition check (amended in the second pass, §12.6).** Ground is warm statement stock, a sheet is
paper with a torn foot, and Knowlu's red (the wordmark's) carries the one primary action per screen,
focus and the needs-you shape. Dark mode is deep warm graphite.

### 7.1 Type (amended, §12.6)
- **Source Serif 4** (Google Fonts, 500/600) for headings, the next task's title and the statement's
  answer line: the letterhead of the statement, echoing the wordmark's serif.
- **Schibsted Grotesk** for everything else, figures included. **No monospace and no tabular
  figures:** Schibsted's tabular set gives the decimal point a full cell ("$9 . 99"), so figures are
  proportional and right-aligned. Azeret Mono is dropped.

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

---

## 11. Build decisions (2026-10-23): what the prototype changed, and why

Two red-team passes found contradictions with the ruled spec, VISION and PRODUCT-DECISIONS. Each was
resolved in the build as follows. Screen ids are the prototype's (`screens.json`).

### 11.1 Hours, payment and judgment
1. **Turned-in work carries counts only, never hours** (spec §6.2: archived at first sight, never
   judged). The section is "Already turned in, never a task". The 8.0h figure is gone everywhere.
2. **No judgment before payment** (spec §1.4, option B; option C rejected). Before `O4` the ledger
   shows counts and the time classes take (arithmetic from registrar times, not judgment); the hours
   column is blank and the sheet foot says "Hours are estimated once your trial starts; nothing is
   judged before then."
3. **No "archived" before the gated run.** Before payment the struck line reads "already turned in ·
   archived when your trial starts"; it changes to "archived" at `o4p-trial-on`, when the first
   entitled run starts.
4. **Q1 restated:** may `judge` run per source *after `O4`*, so hours settle during steps 5-9? The
   prototype shows that behaviour (Blackboard settles at the Google proof, the homework sites at
   Syllabi, the study sessions in the wait). Without it, every hour arrives in `O11`.
5. **Rounding:** hours are whole from 10h up, half hours below, quarter hours as minutes under 1h,
   and every hours column is headed "hours, about". No one-decimal estimates.
6. **Money is not in the ledger.** The order summary at `o4-payment` is its own block above the
   button: the first thing due (a fact read from the feed, not a judgment), what was read, the price,
   $0.00 today. The "trial on" line never posts into the hours ledger.

### 11.2 Flow and paths
7. `O6` Skip goes to `O7` (`o7-other-calendar`), never past it.
8. **Past the 100th Gmail-permission user** (spec §3.2): `o6f-forwarding`, Gmail by forwarding in
   Knowlu's window (Knowlu's own count decides), with the reason in plain words. Calendar then goes by
   its private iCal link at `O7`, since an unverified app's calendar scope shares the cap. *Open
   question for Quinn:* confirm that Calendar past 100 goes by link.
9. **Declined or failed Gmail read:** `o6f-declined`. Forwarding is not offered up front (spec: once
   permission is granted, onboarding stops offering the second path).
10. **The Google warning is designed, not described:** `o6-google` shows a sketch of the screen,
    says why it appears ("Google hasn't verified Knowlu yet", not "is reviewing"), when that is
    decided, and what to check before continuing (the address and the app name).
11. **School email asks before it acts** (`o5-school-email`): "Knowlu will add one forwarding rule to
    your school mailbox ... You can turn it off in Settings." One button, "Turn on forwarding". It is
    no longer a screen with no ask.
12. **A labelled step map in words** replaces "no stepper": every step named, done ones checked,
    skipped ones struck, plus an honest range ("About 5-14 min left") from the spec's per-step times.
    `o1-welcome` lists what's ahead, including the Google warning and the homework-site passwords.
13. **Credential moments get one enclosed treatment** (onboarding-evidence §3.4): a ruled panel with
    a header naming where you sign in. It is used only at `O3a`, `O5`, `O6f`, `O8` and `R2`; §7.3's
    "no boxes" rule holds everywhere else.
14. **The school window answers "why not my browser?"** (`o3a-need`, `o3b-window`, `o3b-duo`): a real
    address bar in the window, Knowlu's check that the host is the school's, and a link for the
    school's IT office.
15. **Homework sites explain why they differ** (`o8-sites`): they don't use the school sign-in, so
    each needs its own password, kept in Windows Credential Manager and sent only to that site.
16. **Syllabi** (`o9-syllabi`): adds "connect Simple Syllabus" and the cloud disclosure: "The file is
    read in Knowlu's cloud to find exam dates, weights and late rules."
17. **Confirming moves onto the ledger.** Class times carry "found" and a "Check them" control at
    `o3c-proof`; Your week (`o10-your-week`) shows only what is left, with "Your day" (the
    planning-day default, 7:30 am to 11:30 pm) as its first row, not hidden among 33.
18. **Resume cases added:** `r1-resume` (payment), `r1-school-expired` (a needs-you line on the
    ledger), `r2-restored` (an account that already has a plan: done steps are marked, and only the
    device-bound sign-ins are asked again, because they never leave a computer). Still to specify:
    resume mid-`O10` (answered rows kept) and mid-`O6` OAuth (the grant is re-checked first), and the
    "setup not finished" listing for a vault abandoned after step 2.
19. **Re-engagement (open question for Quinn):** one "your setup is saved at Payment" reminder the
    day after an abandoned onboarding. It isn't a skipped-source nudge, but nothing sends it today.

### 11.3 The reveal (`o12-reveal`)
20. **The cushion is removed.** It was slack under another name (PRODUCT-DECISIONS: no start-by
    dates) and its figure was invented. §9 Q2 is withdrawn. The topline's existing status word
    ("steady") stays; the runway figure is not drawn.
21. **"Open this week 85.1h" is removed** as an answer figure. The week is per day: due and open
    hours under each column, with a bar of one against the other.
22. **The week shows what spec §1.8 lists:** classes, the student's own events, each due item with
    its time and estimate, optional events hatched and named. The next exam is named under it.
23. **The reveal is cut** to: what's next and why, tonight and tomorrow, how much of tonight is free,
    the week, what was tidied, from your email (with Undo), still coming, and one button. Every line
    of the ledger is on request (`o12-statement`).
24. **Failures appear at the reveal** (`o12-still-running`): a needs-you line with its fix (VHL), a
    skipped source with Connect, and "still reading 1 syllabus" at the 3-minute cap.

### 11.4 The main page (`m1-today`)
25. **Order:** topline, delta line, the **domain strip** (VISION: the list sits under it and it
    filters), then **Next** first, then one balance sentence with a one-row meter, then the list.
    The five-line balance table is on request ("How it adds up"). The next task is the first thing in
    the main column, which is what the 11 pm test asks.
26. **Grades are on the page:** a ring per course, hideable, opening the course (`m1-domain`).
27. **"Accounts" and bursar language are dropped.** Domains are a strip with a status shape and
    word; the course view carries the pace sentence. §9 Q3 is settled by not using billing words.
28. **Must do is called "Must do"**, never "due today or tomorrow"; the verdict reads "Today's
    must-do is about 4.5h, and today has room for about 6.5h of work."
29. **Free time is a number plus open stretches**, never "yours, from 6:00 pm": "About 5.5h free.
    Today's work fits in the open stretches with that left over. Knowlu doesn't set when you work."
    The stretches are drawn on today's schedule.
30. **A task being put off rises** (`m1-avoided`, invented variant): a "Being put off" group with
    "put off 4 days".
31. **Quick capture** is in the topline ("Add or ask", Ctrl K). The topline names the runner
    ("Updated 6:04 am by MAYA-LAPTOP"; anatomy §3.1).
32. **Done today can be unchecked** (PRODUCT-DECISIONS). Late window closed carries counts, never
    hours. The one-comparison line ("more than a usual Monday") is not drawn: there is no history.
33. **States built:** `m1-all-clear`, `m1-spill` (spill drawn outside the bracket, no advice),
    `m1-source-broken`, `m1-offline`, `m1-vhl`, `m1-undo` (Undo on a Knowlu close, with "won't mark
    it done again"), `m1-late` ("When will you turn it in?"), `m1-task-open` (every field edits in
    place; Knowlu's estimate, why and source are visible, not editable), `m1-today-dark`.

### 11.5 Fixed facts
34. October 19-25 is **week 9 of 16** (week 8 ends on onboarding Sunday; the main page is week 10).
35. Campus-row fields drive every school-specific string: `name`, `mfa` ("Duo"; a school without MFA
    drops the phone line), `lms` ("Blackboard"; Canvas reads "Canvas calendar link"),
    `shared_sign_in` and its `covers` list (a school whose mail isn't covered shows "School email
    needs its own sign-in" at `O5` and drops "school email" from the `O3` benefit), and the sign-in
    `host`. The prototype reads them from one `ROW` object.

## 12. Second pass (2026-10-23): what the critiques changed

Two critiques scored the first build 54 and 57. Every top problem was addressed in place; what they
said to keep was kept. Screen ids are in `screens.json`; shots in `shots/`.

1. **The statement now balances, over one period.** It is "Your week, Mon Oct 19 to Sun Oct 25":
   *Your time* (awake 112h, less classes 16h, less repeating shifts and club 9h, less this week's
   plans 2h = **open time 85h**) against *Work due this week* (Blackboard 6 · 9.5h, zyBooks 1 · 2h,
   VHL 2 · 2h = **13.5h**), and one double-ruled answer: **open time left after the work, 71.5h**,
   with the tightest day named under it ("Thursday, 4h due in 10h open"). Every figure is computed
   in the page from `sample-student.json` (`weekOf()`); displayed parts add up to displayed totals.
   Work after this week is a count line ("Due through December 4 · 26"), never mixed into the week.
   This partly reverses §11.3 item 21: the open-time total is back, but only as one side of a
   balance whose answer and tightest day are shown with it. **Question for Quinn:** is "open time
   left after the work" too close to slack (PRODUCT-DECISIONS)? It is a fact about the week, never a
   date to start by; if it reads as slack, the answer line can drop the figure and keep the tightest
   day.
2. **The reveal is the statement, finished** (`o12-reveal`): one row per day (classes and plans,
   each due item with its time, open, due, and a bar of due drawn inside open), the week total, the
   double-ruled answer, and next week in one line (24h due in 85h open, Friday heaviest, CHEM 231
   Exam 2). The 7-column calendar grid is gone. The side column is Next and why, tonight and
   tomorrow, what waits for a yes, what was tidied, what is still coming.
3. **Mail-found work is proposed, not added** (ruling fix): "Waiting for your yes: Pay Pre-Health
   Society dues ($15) · Add it / Not now". The Gmail proof says it "waits for your yes".
4. **Proof counts follow the ruled form**: "22 assignments to do, and 4 already turned in"; the
   turned-in line strikes the *work's titles*, never the source's name.
5. **School email is no longer its own ask** (`o5-school-email`): "Nothing new to sign in to"; a
   disclosure line says that continuing turns on one forwarding rule, with how to turn it off, and
   the same line appears earlier at `o3a-need`. The button is "Continue". **Question for Quinn:** is
   a disclosed Continue enough, or should forwarding wait for an explicit control?
6. **Visual world committed** (§7 amended): warm stock, Source Serif headings, dotted leaders,
   double rules, a perforation and a torn sheet foot; Knowlu red for primary actions; a highlighter
   stroke and "just read" tag mark the lines each step posted, so every proof visibly changes the
   statement. Money and hours render without tabular spacing.
7. **Main page** (`m1-today`): one top bar (date, week, Add or ask, runner, status word, needs-you
   flag), then **Next** first; under it today's balance as arithmetic ("Open today 12h − Must do
   4.5h − Recommended 2h = Free about 5.5h", which adds up), then "Since 6:00 pm yesterday" with Undo,
   then the domain strip directly above the list it filters (status words only where a course is
   behind or ahead; no bare glyphs). The block meter is gone. The rail is today's schedule with open
   stretches, **This week** as a small statement (due inside open per day, double-ruled "left after
   the work"), grades as a list with a small ring per course, and Needs you.
8. **States fixed**: `m1-spill` keeps Next (Lab 7) and shows "Over by 1h" without advice;
   `m1-domain` and `m1-task-open` drawers cover the rail only, so the list's hours stay visible, and
   the course drawer no longer repeats the task list.
9. **Check it** (`o10-your-week`, step renamed so it no longer reads like "Here's your week"): exams
   grouped by course with one "Confirm all 10"; the other groups are one line each; the primary
   action sits above the fold.
10. **Step map**: eight short labels plus the destination "Your week"; one estimate ("About 7 min to
    your week") instead of a range.
11. **Scaffolding removed from screens**: no "(prototype)" buttons. The Duo push is approved on a
    drawn phone beside the school window; checkout is a drawn browser window; the wait advances by
    itself (or "Don't wait"); alternate paths live only in the screens index, which `?shot` hides.
    Screenshots freeze motion and force the light theme except `-dark` screens.


## 13. Round 2 (2026-10-23): what changed after the five-direction comparison

- **The weekly leftover total is gone.** "Open time left after the work" (71.5h at the reveal, 60.5h on
  the main page) summed every awake hour as free. Its place is the honest reading of the same ledger:
  the **tightest day**, work due inside that day's open time ("Tightest day: Thursday · 4h of 10h" at
  the reveal; "Tightest day: Fri · 10h of 13.5h" under This week). The reveal's total row keeps the
  work due (13.5h) and drops the 85h open sum; next week's line drops its open sum too.
- **The reveal is no longer spoiled.** During onboarding the ledger shows only its two inputs: "What
  takes your time" (awake window with no hour sum, then the hours classes and plans take) and "Work
  due this week". The open-time total is removed, and the answer line is held: "Each day, work
  against open time · at the end". The day-by-day balance first appears at `o12-reveal`. The welcome
  sheet, the school intro line and Check it (`o10`, "Everything is read ... then Knowlu weighs each
  day") were reworded to match.
- **Daily page: the must-do list follows Next directly** (y about 290 on `m1-today`, was 570). Today's
  balance moved to the right column as a small ledger above the open stretches (Open today, Must do,
  Recommended, "Open after the work about 5.5h"), replacing the duplicate "About 5.5h free" line.
  "Since 6:00 pm yesterday" and the course chips moved below Recommended. On `m1-spill` the over-by
  line stays above the list (it is that state's message) and "Since 12:00 pm" moved below it.
  "Free" in the tally became "Open after the work".
- Every screen was re-shot into `shots/`.
