# Direction 5b: Departures (`departures-board`)

Brief for the 2026-10-23 directions run. Written 2026-10-23. **Third reference: Flighty.** Every
direction starts from Notion and Stripe; this one adds Flighty, and no other direction uses it.

Sources this brief follows, in order of authority: `docs/specs/2026-10-23-onboarding-redesign-draft.md`
(ruled 2026-10-23, "the spec"), `VISION.md`, `docs/design/PRODUCT-DECISIONS.md`,
`docs/design/DESIGN-PRINCIPLES.md`, `docs/design/BRAND-VOICE.md`, `docs/surface/anatomy.md`,
`../research/product-truth.md`. The sample data is `../shared/sample-student.json` (Maya Okafor,
Halverson State University; invented). Every person, school, host, room and grade below is invented.
Real hosts appear only for real services (Google, Stripe, zyBooks, VHL).

Citations: `notion.md §n`, `stripe.md §n`, `onboarding-evidence.md §n` (all in `../research/`).
Flighty sources were read on 2026-10-23 and are listed in `archive/BRIEF-first-pass.md` §8.3 as [F1]-[F6].

---

## R2. Revision after the red team (2026-10-23): what changed, and what the prototype builds

Two red-team passes found contradictions with the ruled spec and VISION. This section resolves each
one. **R3 (the second pass, below) overrides R2 where they differ**, and `index.html` builds R2 as
amended by R3. The first-pass brief (§0-§10, with the withdrawn metaphor) now lives in
`archive/BRIEF-first-pass.md`. Screen ids in the prototype are lowercase (`o2p-proof`, `m1-today`);
`screens.json` lists all 66.

### R2.1 The board during onboarding counts; the reveal names

- **Before the reveal, the week strip shows counts, never titles or order.** Each day column carries
  a count figure and one course-tinted chip per item (course code only), from device-side reads. No
  hours, no titles, no "what's next". The reveal is where the week first appears in words, in
  Knowlu's order, with hours and the live card. This gives the reveal back its job (spec §1.2, §1.8)
  and keeps the direction's thesis: the board visibly fills with every login.
- **Per-day counts depend on spike S-F** (device-side parse of the feed's dates). If S-F gives totals
  only, the fallback is built: `o2p-totals` shows the same proof with an undated strip and the line
  "Due dates for each day come in after payment, when Knowlu reads the assignments into your plan."
- **Turned-in work is never drawn.** No line, no stamp, no animation. Only a count under the strip:
  "4 already turned in. They won't be on your list." Before the first slot the copy never says
  "archived" (nothing is archived before step 10); "archived" appears first at W1 and the reveal,
  after the first slot ran ("52 past items and 9 already turned in were archived").
- **The proof explains the gap between 26 and 6.** `o2p-proof`: "Of the 26 assignments, 6 are due this
  week, 16 later this term, and 4 you've already turned in: those won't be on your list. 29 from
  earlier weeks stay off it too."
- **Class times from the registrar are solid from the school proof**, already set (spec §4.2, ruling
  3). Only Knowlu-noticed fixed marks are dotted until Your week: the 3 repeating Google Calendar
  events and the 2 syllabus office hours. One-off calendar events (Haircut, Family video call) are
  solid. Maya's registrar week is 14 meetings from 7 meeting patterns; the proof says "14 class
  meetings a week".
- §1.1's dotted registrar marks, §3.2's "Turned-in work lands, then leaves", §6.4's turned-in motion
  row and §4.1's "Arrived: N, archived" strip cells are withdrawn.

### R2.2 The progress rail replaces the ticket line and "About 6 minutes left"

- A left rail, **"Your sources"**, lists every step to the end, including those ahead (Payment, School
  email, Google, Homework sites, Syllabi, Your week, Here's your week), each with one state word and
  shape (Done, Now, Reading, Still reading, Checking, Needs you, Not connected) and its proof line.
  The current step's sources show as sub-rows (School: Classes, Assignments, Class times, Grades,
  Syllabi). The rail foot carries "About N minutes left" (from the spec's per-step times) and Finish
  later. This is the spec's "progress rail that stays visible to the end" and the labeled step map
  (NN/g; "Step 3 of 9").
- **The ticket line is gone as a fixed bar.** Price disclosure is said once at the account step ("$9.99
  a month after 7 days free. You'll see what Knowlu found at your school before you pay. There's no
  free plan after the trial.") and rides on the rail's Payment row ("$9.99 a month, after 7 days
  free") until payment. Nothing on screen counts down.
- Student-facing copy drops the metaphor: no "departures", "Boarding soon", "Arrived" or "ticket".
  The sign survives in type, figures and structure only. "Arrived this week" is **"Closed this week"**
  (the read model's name), each line saying who closed it: you, Knowlu, or lapsed.

### R2.3 Payment waits for the proof (Q2, option B)

`o2r-reading` holds "Continue to payment" disabled, with the reason beside it ("Opens when classes
and assignments are counted"). It opens when the course list and assignments are counted; a slow
source after that (grades, `o2r-slow`) does not block, and is named on the rail ("Still reading:
early in a term this can take longer"). The order summary at `o3-payment` is therefore never empty.
An empty assignments read is a failure, never an empty term (`o2-x-empty`, "Read Blackboard again",
`SCH-EMPTY-READ`).

### R2.4 Skip costs sit on the step

Every optional step prints "If you skip: ..." under its ask, before the Skip button (spec §1.7). There
is no skip pane. Skipping writes a "Not connected" rail row and one line under the strip ending "You
can connect it in Settings, Sources." The reveal repeats it under **Not connected** with a Connect
button, and `m10-settings` lists every source with Connect, Forget or Sign in. Cost lines avoid
"missed": "won't reach your list", "are missing unless Blackboard's feed carries them".

### R2.5 The school step owns no vendor, page or LMS name in code

- MFA kind, its timeout hint, the help-desk wording and the LMS feed's name come from the **campus
  row** (`mfa: duo`, `mfa_timeout_hint`, `lms: Blackboard`, `feed_steps`). For Halverson State the
  copy reads "Halverson State asks for a phone approval (Duo)"; a school with another MFA or none
  gets its own line or none.
- **Knowlu cannot see the school's page** (spec §2.4: the windows run no scripts). So it never
  mirrors a Duo screen and never detects a lockout page. It knows three things: a session arrived,
  the window closed, or no session came back. `o2w-window` and `o2m-duo` show the same Knowlu band
  while the school's own window (labelled "Knowlu doesn't draw or read this page") changes. The
  endings are `o2-x-closed`, `o2-x-timeout` (Knowlu waits **5 minutes** with no session; the copy
  then cites the campus row's "the phone approval gives up after about a minute") and a student-led
  `o2-x-trouble` ("I can't sign in": locked, no phone, something else -> feed link). `O2-x-locked` and
  `O2-x-duo` are withdrawn.
- **Variants**: `shared_sign_in: true` (Maya): "You won't sign in to these again", School email
  "Nothing new to sign in". `shared_sign_in: false`: the school benefit line drops "and school email
  ... again", and School email shows its own sign-in in an external window with the same three
  endings. **Uncurated school** (`o2n-uncurated`): the feed link, with what it leaves out said
  first. **Kept sign-in turned off** (Settings): one throwaway sign-in for the sitting; the resume
  line becomes "Halverson State asks you to sign in each time Knowlu needs it, because the kept
  sign-in is off."

### R2.6 Mail, Google and the cap

- **School email (`o4-school-email`)** tells the student Knowlu sets a forwarding rule in their school
  mailbox, says some schools refuse it, and says older mail "stays in Outlook for now; Knowlu reads
  what arrives from here on" (M365 backfill is held until L1). The proof drops "older school mail is
  still coming in". A late test message stays **Checking**, never Needs you (`o4r-late`).
- **The reveal's Still coming** no longer says "Older school email ... will be proposed" (the sample's
  `reveal.still_coming` line contradicts spec §3.5 and VISION's setup exception; the prototype shows
  "Nothing. Everything you connected ran once." for Maya). For a Gmail-forwarding student the line
  would read "Older Gmail is still coming in; what it finds from before today is added with Undo."
- **Google (`o5-google`)** explains Google's unverified-app screen in advance (why, and the two
  clicks), and `o5b-consent` shows a schematic of Google's screen, labelled as Google's. The 7-day
  reconnect depends on the project's publishing status (spike S-D) and is not promised either way.
- **Past the 100th Gmail-permission user** (`o5-cap`): Calendar by permission, Gmail by forwarding,
  with the reason in one sentence.
- **Partial grants**: Gmail yes, Calendar no is `o5p-partial` ("Share calendar" asks only for the
  calendar). Calendar yes, Gmail no goes to `o5f-forwarding` with the rail showing Calendar Done and
  Gmail Needs you.
- **Cloud disclosure**: one line at the school step ("Assignment titles and dates go to Knowlu's cloud
  to estimate effort and set the order; your passwords and school session never leave this
  computer"), at Google and School email ("Mail that can't affect your plan is set aside before
  anything reads it; the rest is read in Knowlu's cloud to find dates, and Knowlu keeps what it
  found, never the message"), and at Syllabi. The plan folder is shown at `o1d-signed-in` with "Show
  me the folder".

### R2.7 Secure surfaces are two kinds

**External windows** (school, Google, Stripe) are pages Knowlu doesn't draw: the step shows a framed
box with the host in mono and the label "Knowlu doesn't draw this page". **Knowlu's own form** (zyBooks,
VHL) is labelled "Knowlu's form" and says "Stored only on this computer, in Windows Credential
Manager, and used only to read zyBooks."

### R2.8 Payment copy

"$9.99 a month. The first 7 days are free; the first charge is October 25. Cancel in Settings before
then and you pay nothing." Then "$39.99 for a 6-month term. Pell Grant recipients pay 75% less: $2.49
a month or $9.99 a term. How to confirm it" (figures from `docs/notes/2026-09-29-funding-and-seed-grant.md`).
**Open for Quinn:** how a Pell award is confirmed; the prototype holds a marked placeholder.

### R2.9 Resume, restore and profiles

`x2-resume` (payment), `x3-expired` (school session gone; the rail's Your school reads "Sign in
again"), `x4-restored` (an account that already has a plan, on a new computer: finished steps are
Done with their proof; only per-computer things are asked: the school sign-in and homework-site
passwords) and `x5-picker` (a second profile on the same machine, "Setup not finished: stopped at
Payment, October 18", with Continue setup and Remove). `x1-finish-later` says nothing runs in the
background until setup is finished.

### R2.10 Syllabi and Your week

- `o8-syllabi` offers **Choose CS 110's syllabus file**, **Connect Simple Syllabus** and **Skip CS 110**,
  with the cost on the step. `o8-x` is the unreadable-file ending (`SYL-UNREADABLE`).
- `o9-your-week`: **Classes from the registrar arrive set**, each with Edit only. Every other row
  (exams, weights, late rules, office hours) has Confirm, Edit and **Leave blank**; each group has
  "Confirm all N". Repeating calendar events carry Hard / Soft / Optional and Mine / Not mine. The
  step says: "Anything you leave blank comes back as a question over the next few days." Unconfirmed
  rows go to the deck under L5's caps (spec §4.2).

### R2.11 The main page, regrouped (M1)

Three levels of visibility (principle 5), top to bottom:
1. **Ambient**: the topline (date, week, one health phrase that grows a clause only when something
   needs the student: "VHL needs you"), the week strip (hours due per day with a bar, tasks per day,
   the exam tag, "Heaviest of the next 2 weeks", and a Nov 2-8 cell), and the **domain band** (one
   cell per domain: grade ring and letter, pace in hours, plain ink). Under the band, the read
   model's sentence for each behind domain, which carries the way back ("Today's study session and
   one Wednesday catch it up before Thursday's exam").
2. **Do** (left): the live card, then **Today** (verdict sentence with the 30-minute meter, rows in
   Knowlu's order, "Recommended for today"), then four folds: Done today (can be unchecked), Later,
   Late window closed, Closed this week.
3. **Know** (right): **Your day** (commitments and free time, stated once: "About 5.7h free, after
   6:00 pm"; the verdict line no longer repeats free time), **Needs you** (one card, the deck), **Since
   last night** (changes) and **Good to know** (notices).

Changes from §5:
- **No per-row status column and no new status vocabulary.** A row shows a word only when the read
  model already has the fact: **Late window open** (`state: past due, inside late window`), **Needs
  you** (a pending decision targets the task: Literature review draft, Lab 8 in Later), **Done**.
  "On track" and "Changed" as row words are withdrawn, so surface output and the `surface-today-*.json`
  references are unchanged. A source's staleness lives once, in the topline and the changes list.
- **The room notice is Good to know**, not a change and not on a row: "CHEM 231 Exam 2 on Thursday is
  in Hallett Hall 120, not the usual room. From a CHEM 231 Blackboard announcement email, Saturday
  10:03 am. Nothing in your plan changes."
- **The live card** leads with the title. Its figures are clock times ("Was due Fri 11:59 pm", "Late
  window closes Wed 11:59 pm") at 28 px, with "−30% now" as a small amber pill; the why line carries
  the way back. **The relative phrase ("in about 15 hours") is dropped everywhere** (resolves §9 Q3).
- **The task detail** (`m2-task`) opens under its row: fields the student can change, what Knowlu
  worked out (why, source, first estimate, time spent), steps ("Checking every step doesn't finish the
  task; Knowlu asks you"), history, **Report a problem** (`m2-report`, the eight reasons and a note)
  and **Delete** (`m2-delete`, asks first, optional reason that also files a report).
- **Avoided tasks**: a row that keeps being passed over gains a sub-line "Put off since Friday" and
  rises (VISION). Maya's Monday has none, so no screen shows it.
- **Grades**: rings use `--g-a/b/c/d` (one hue per letter, never red or amber), always with the letter
  printed; Settings has Show / Hide grades, and hidden grades remove ring and letter.
- **Undo on Knowlu's close** (`m1-undo`): from Closed this week or the change line, Undo puts "Pre-lecture
  quiz: Ch. 7" back on the list with "Knowlu won't mark it done again; check it off when it's finished"
  (judge once).
- New states: `m8-offline`, `m9-run-failed` ("The 12:00 check didn't finish: Blackboard stopped
  answering. Knowlu tries again at 6:00; this is from 6:02 am."), `m7-all-clear`, `m11-lock-in`.

### R2.12 Smaller corrections

- Done today rows **can be unchecked** (PRODUCT-DECISIONS), not "uncheckable".
- The meter counts 30-minute blocks of must-do against today's room; spill blocks are outlined, never
  red.
- §6.1 type check: Big Shoulders Text sets tabular figures in the build (strip and row figures align
  in the screenshots); Atkinson Hyperlegible Next's slashed zero shows in body figures. Judge both in
  the prototype (§9 Q5).
- §9 questions still open: Q1 ("Late window open"), Q5 (Big Shoulders in light mode). Q2 is resolved
  ("Closed this week", no "Arrived"), Q3 (relative phrase dropped) and Q4 (ticket line removed). New:
  Pell confirmation (R2.8).

## R3. Second pass after the critique (2026-10-23): what changed in the build

Two critiques scored the first build 57 and 61. Every ruling violation and top problem is answered
here, in their rank order; what they said to keep is kept (the rail as a labeled step map, payment
held for proof, "If you skip" on the step, the school's own window, the order summary, the live card's
clock times, Late window closed, counted-never-drawn turned-in work, the task detail, light mode).

### R3.1 The rail never shows a finding before the login that earns it

Rows ahead now carry the step's **benefit**, not its result: Your school "Classes, assignments, grades
and syllabi", School email "Date changes your teachers send", Google "Club and job deadlines, your
calendar", Homework sites "Due dates from homework sites", Syllabi "Exam dates, grade weights and late
rules". Only after the school read (from `o2p-proof` on) may a row ahead name what that read found
("Same sign-in as your school", "zyBooks and VHL, named in your syllabi", "Found 4 of 5; CS 110 is
missing"). This also removes the shared-sign-in assumption from the screens before the school is
known (R2.5).

Sub-rows show only while something under a step is still running or needs the student. Once every
source is Done the row collapses to its one proof line, so the rail fits at 800 px and stops repeating
the pane. At `w1-wait` the rail says Syllabi **Reading** ("Reading CS 110's syllabus") and Here's your
week "Two things still running", matching the pane.

### R3.2 The board is a departures board now, and it grows at each proof

- One row per day (Mon 19 to Sun 25, then Later). Each row: the day, a count, one tile per item
  (course code and due time; still no titles, hours or order before the reveal, R2.1), and on the
  right the day's fixed time as counts ("3 classes · office hours", "1 event").
- **At a proof step the board opens full and marks what that login added**: the new tiles get an amber
  tint with one short settle (1.4 s, reduced motion: none), and the head says what landed ("+6 due this
  week and 16 later, from Halverson State"; "+1 deadline from Gmail, and your own events from
  Calendar"; "+1 due this week, from zyBooks"; "+2 due this week, from VHL"; "Exam dates from your
  syllabi"). That is the direction's thesis made visible at every proof.
- **On an ask step the board shrinks to one line** of day counts plus "N turned in, kept off your list",
  so the ask, its cost line and its buttons stay above the fold (`o4`, `o5`, `o7`, `o2n`).
- Empty (first screens) it names what fills it: "Your school brings assignments and class times.
  Google brings your own events and personal deadlines. Homework sites bring their due dates." While
  the school reads, every row shows a dashed reading bar.
- The board is hidden at payment (the order summary is the board there) and at Your week.

### R3.3 The school proof is a sentence, not a stat row

`o2p-proof` leads with one proof line ("Found 5 classes, 26 assignments, and 4 you've already turned
in"), three small checked facts (14 class meetings from the registrar, grades for 5, syllabi for 4 of 5)
and the bridge sentence that explains 26 against 6, now pointing at the board ("6 are due this week
(highlighted below)"). The big-number tally is gone.

### R3.4 Numbers bridge from each proof to the reveal

Homework-site proofs now say what they keep off the list: zyBooks "8 assignments this term, 3 already
turned in. 14 from earlier weeks stay off your list"; VHL "6 assignments, 2 already turned in. 9 from
earlier weeks". The reveal then adds them up in one sentence: "Archived the 9 you'd already turned in
(4 in Blackboard, 5 on homework sites) and 52 from earlier weeks (29 in Blackboard, 23 on homework
sites)." The VHL-skipped variant reads 7 and 43.

### R3.5 Your week: classes are one line, the questions come first, Continue is always in view

`o9-your-week` opens with one set line, "5 classes, 14 meetings a week, set from the registrar.
Nothing to type", with "Show the 7 meeting times" (each with Edit). It no longer says "Classes · 7".
Then the rows that need the student, in this order: Late work, Exams, Grading, Repeating events
(Hard / Soft / Optional, Mine / Not mine), Office hours, Your day. No row is pre-answered except ENG
210's "no late rule found", which starts on Leave blank. "Confirm your week" sits in a bar pinned to
the bottom of the pane with "0 of 26 answered. Unanswered rows stay as found and come back as
questions."

### R3.6 The reveal has one hierarchy

Top to bottom: the headline and "9 things due, in Knowlu's order from here on"; the live card for what's
next **with Open Knowlu inside it** ("Your page opens on this, every morning"), beside Tonight (free time once, and the two things due by tomorrow); the week, named in full
with nothing truncated (titles wrap; each day's hours are a small "about 2h", not a display figure;
under each day its class count and the student's own events by name and time); then one band of three
short paragraphs: What Knowlu tidied, From your email (with Undo), Still coming (or "Still coming, and
not connected" with Connect VHL). The folder and checks are one footnote. The right-hand audit column
and the unlabeled 3 px time marks are gone.

### R3.7 M1: the answer is the first thing under the topline

- **Nothing is pinned above the card.** The topline (48 px: date, week, "Checked 6:02 am · in step with
  your account", Settings) and then the live card. The week strip and the domain band are gone from
  the top.
- **One hours statement.** The verdict ("About 4.7h is due by tomorrow, and today has room for it")
  heads a group now called **Due by tomorrow** ("Lab 6 above, then these, in Knowlu's order"). The
  "6 tasks · about 6.5h" header is gone; Recommended for today shows its own 1h 48m. Your day says
  "Free from 6:00 pm" instead of a second hour figure, and its bar has a legend (Filled, Dashed, Open).
- **This week** moves under the list, unpinned, as eight cells: day, hours due that day (minutes under
  an hour: "15m"), a bar, "N due", and an exam tag that wraps ("CHEM 231 exam") instead of truncating.
  Monday's 2.2h plus Tuesday's 2.5h is the verdict's 4.7h, so the numbers agree.
- **Your classes** moves to the right column, last: one row per course (ring and letter when grades are
  shown, pace), the read model's sentence under each course that is behind (it carries the way back),
  and one line for the domains without a grade ("Library job, Pre-Health Society and Personal: on
  track"). Clicking a course still filters the list. Hidden on the all-clear Sunday, where Monday's
  sentences would be stale.
- "VHL needs you" is said once, in Since last night (with Sign in to VHL), not also in the topline.
- Row order numbers are gone (the live card is next; the list header says the order).

### R3.8 Type and craft

- **The read face is now Public Sans** (civic-signage lineage, plain zero), replacing Atkinson
  Hyperlegible Next, whose slashed zero turned every "CS 110" and "10:00" into terminal text. The
  first pass's note that the slashed zero came only from tabular figures was wrong: it is the face's
  default glyph. Big Shoulders Text stays for the board's figures and headings; Atkinson Hyperlegible
  Mono stays for hosts and support codes only.
- **Fonts are embedded** in `index.html` as data URIs (latin subsets, SIL OFL), so the sign keeps its
  type offline or on a slow first run, and the tray matches the board. The prototype makes no
  network calls.
- Payment sets "$9.99" as its own line at 44 px with "a month" beside it, and the Stripe box sits
  above "Continue to checkout", so the trust copy is never under the fold (`x2-resume` included).
- "Prototype:" scenario links are hidden when `?shot=1`.

### R3.9 Open for Quinn (unchanged plus one)

Q1 ("Late window open"), Q5 (Big Shoulders in light mode), Pell confirmation (R2.8), and new: **Public
Sans as the read face** (recommendation: keep it; Atkinson Next only if low-vision legibility testing
says the slashed zero matters more than the look).

---

## Round 2 (2026-10-23): what changed

This round answers the comparison README's "What to fix in 5b". `index.html` builds it, and the changed
screens are re-shot in `shots/` (dark as `<id>.png`; light as `<id>--light.png` for `m1-today`,
`m2-task`, `m7-all-clear`, `o9-your-week` and `m10-settings`).

- **M1 carries the board now.** The headline answers in the board's face ("Lab 6 is next.") with one
  line saying whether the day holds ("About 4.7h is due by tomorrow, and today has room for it. After
  today's work, about 6h is still yours, most of it after 2:15 pm."). Under it sits one departures board:
  Due · Course · Assignment · Left · Status columns, the next departure enlarged as row 1 (its why,
  Lock in, Done, Open in zyBooks, the late-window figure), the rest in Knowlu's order, each with a
  status word that has a shape (Late window, 60% done, Not started, VHL signed out, Needs you), and
  "Recommended for today" as a separator row on the same board. The old card, verdict meter and
  horizontal strip are gone. Row actions float over the status column on hover and never take a column.
- **The week on M1 is the setup board, read against open time.** It has one row per day with count,
  hours and a bar of the work due against that day's open time (from 3, without the "71.5h left" total),
  and the tightest day is named under it ("Tightest: Friday, about 10.2h due in 13.3h of open time").
- **"Free from" is computed.** Open time is the planning day (wake to bed) minus hard and soft
  commitments; optional ones don't count. The label names the longest open stretch ("Free from
  2:15 pm · 12.3h open"; on the all-clear Sunday, "Free now to 7:00 pm · 13.5h open"). The day bar
  draws the same stretch outlined and a now line, so the label and the bar can't disagree. The literal
  fallback is gone.
- **Needs-you cards are date-checked** against the page's own date. A card whose date has passed
  closes itself and is named once ("One question closed on its own once the date passed (the
  Literature review date). Nothing in your plan changed"). M7 no longer offers "Keep Friday" on
  Sunday; it shows Lab 8 (Wednesday, November 4). Waiting counts and the oldest age are computed.
- **The VHL sign-out is in the topline**, with "Sign in again" asking only for VHL. "In step with your
  account" is gone; the health line says "Checked 6:02 am · next check 12:00" (M7: "every source
  answered"). The duplicate VHL item left "Since last night".
- **O9 Your week, rebuilt.** The registrar line stays first and set. Then come "Three questions only you
  can answer" (ENG 210's missing late rule, how firm each repeating event is with Knowlu's guess
  preselected, the wake-to-bed window), then "What your syllabi said": Exams (3 shown, 7 more on
  request), Late rules, Grade weights and Office hours, each with one "These are right" and Edit per
  row. The 26 Confirm / Edit / Leave blank rows are gone.
- **The board arrives with the first proof.** Welcome, account, school sign-in and the school read
  show no board and no empty skeleton; it first appears at `o2p-proof` (or `o2p-totals`), already
  holding the school's counts.
- **Light mode at parity.** The page starts in the Windows setting (`?theme=` and the saved choice
  still win), Settings has a Look row (Dark / Light), condensed figures and labels step up to 700 on
  light, and every new surface (board, lead row, separators, week board, topline alert) uses tokens
  defined in both themes.

Still open: the wait (W1) is not yet rebuilt as a countable list, and the board's per-day counts during
setup still depend on spike S-F.

## Round 3 (2026-10-23): what changed

This round answers the comparison README's "What to fix in 5b before anyone sees it" and its three
grafts. Re-shot in `shots/`: `r1-reveal` (dark and `--light`), `r1-variant`, `w1-wait`, the new
`w1-cap`, `m1-today` (dark and `--light`), `o2p-proof` (dark and `--light`), `o2p-totals`,
`o7-zybooks`, `o7-vhl`, `o8-syllabi`, `o3-payment`.

- **The reveal is the board.** `r1-reveal` keeps the headline, the live card and Tonight, then shows
  the same one-row-per-day board the student watched fill (day, count, then each item with its due
  time, course tile, title and "about 30m"; the day's classes and own events on the right; Later with
  its count; "9 already turned in. Archived, and not on your list" under it). The live item is
  marked **Next** on its row. The 7-column calendar grid is gone. The button is **"Start with Reading:
  Ch. 8"**, not "Open Knowlu". What Knowlu tidied, From your email and Still coming stay as the band
  below the board. The VHL-skipped variant says "Estimated the effort of 30 assignments" (not 34).
- **The wait is "Building your week".** `w1-wait` is a countable list of 9 lines (classes and times,
  assignments, grades, school email, Gmail and Calendar, homework sites, turned-in work archived,
  effort and order, syllabi), each Done or Running, under "7 of 9 done", a 9-segment bar and the elapsed
  time. The 3-minute cap is said up front. New `w1-cap` shows the cap: "8 of 9 done", "3 minutes is
  up, so your week opens now. Still running: CS 110's syllabus", and Open your week, which leads to
  `r1-variant`. The rail ("Two things still running" / "One thing still running"), the list and the
  reveal's numbers (9/52 or 7/43 archived, 34 or 30 estimated) agree.
- **M1, the daily page.**
  - A late item's Due cell reads **"Late"** (amber) with "window closes Wed 11:59 pm". Its Status
    shows "−30% now" and "Was due Fri 11:59 pm". The large "11:59 pm" is gone from late rows (rowHtml
    does this for any row in its late window).
  - The column is **"Work left"**. All times use one format, "1h 12m" or "24m": the verdict is now
    "About 4h 42m of work is due by tomorrow" (the five rows' sum), Your day says "12h 20m open", and
    the week board uses the same format.
  - **Done is the lead row's primary button.** "Lock in" is now a link under the buttons that says what
    it does: "Lock in starts a 25-minute focus timer on this task."
  - **Every row shows a Done button at rest**, in its own column. Pin and Open moved out of the hover
    overlay; they stay in the task detail.
  - **Quieter right column.** The Needs-you card has one filled button (Add it); "Not for me" and
    "Not now" are links, and the quoted email sits behind "Show the email". The day bar labels itself
    ("Now" over the line, "Free 2:15 pm to 11:30 pm" over the outlined stretch), and the legend is one
    line.
- **Grafts.**
  - From 4: once the school is read, the rail rows for Homework sites and Syllabi say **"Found at your
    school."** and why ("CS 110's Blackboard page links zyBooks; SPAN 202's syllabus names VHL"; "4 of 5
    in Blackboard; CS 110's isn't, so Knowlu asks"). On the homework step, the zyBooks and VHL sub-rows
    say the same. The `o7-zybooks` intro no longer says CS 110's syllabus names zyBooks, which
    contradicted the data (CS 110's syllabus isn't in Blackboard).
  - From 1b: `o2p-proof` has ruled rows under the proof sentence, headed **"Left out, and why"**: 4
    already turned in, 29 from earlier weeks, 2 classes from last term (unnamed, per the sample's
    rule), 1 syllabus not found.
  - The proof headline no longer orphans "in." (a non-breaking space plus balanced wrapping).

Still open after round 3:
- **The 11 pm order.** No screen shows the evening case; Lab 6 still ranks first. This is a ranking
  rule for Quinn, not a layout fix.
- `r1-reveal` at 1280x800: the board starts at about 380 px, so Saturday, Sunday and Later sit below
  the fold, and the tidied band comes after them. The reveal scrolls by design, but a student who
  doesn't scroll misses the weekend.
- M1: "This week", "Since last night" and "Your classes" are still below the fold. The right column
  is quieter but not shorter.
- Lab 6 reads "about 2h" on the reveal (October 19 to 25) and "1h 12m" of work left on M1 (October
  26, 40% done). The two screens are a week apart, so this isn't a contradiction, but a critic may
  read it as one.
- Not re-checked this round: the error, empty and skip states other than `w1-cap`, and the M1
  variants (`m4`, `m5`, `m6`, `m7`, `m11`), which share the changed row and right-column code.
