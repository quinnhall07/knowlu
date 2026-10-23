# Seven directions for the whole student journey

Written 2026-10-23 for the five-direction UI run. Seven candidates; five go forward. Each one changes
the **structure and flow**, not only the look: how the logins are framed, when the student first sees
their week, what the reveal is, and what the main page fundamentally *is*. Each starts from Notion +
Stripe and takes one distinct third reference (no two share one).

Read with `research/product-truth.md` (what every screen must carry), `research/notion.md`,
`research/stripe.md`, `research/onboarding-evidence.md`, `research/landscape.md`. Sample data is
`shared/sample-student.json` (Maya Okafor, Halverson State, invented).

## What every direction keeps (the ruled floor)

None of the seven moves these; each varies only what is presented, when the week becomes visible,
and what the main page is.

- **Ask order:** welcome -> account (Google or emailed code; the price "$9.99 a month after 7 days
  free, no free tier" is *disclosed* here, charged later) -> **one** school sign-in on the school's own
  page -> its proof -> payment (Stripe checkout; testers see "nothing to pay") -> school email (nothing
  new when the school sign-in covers it) -> **one** Google consent for Gmail + Calendar (forwarding only
  on decline or failure; partial grants reported per scope) -> another calendar only if none granted ->
  homework sites (zyBooks, VHL; only those found mentioned) -> syllabi missing a file -> **Your week**
  (confirm class times, exams, weights, late rules; Edit per row; typing last) -> **"Here's your week"**.
- Why line before each ask, proof after it ("5 classes this term; 26 assignments, 4 already turned
  in"); "reading..." lets the student move on and the proof lands later on a rail that stays visible.
- Skippable steps name their cost on the step and again at the reveal; account, school, payment and
  Your week are not skippable.
- Resume after quit opens the same step with finished proof intact.
- Privacy said at the moment it matters: the school password goes to the school's own page and Knowlu
  keeps the session, never the password; homework-site passwords are stored only on this computer;
  Gmail and Calendar are read-only; the plan is plain files on this computer ("Show me the folder").
- The reveal waits up to 3 minutes, then opens and names what is still running; backfills are never
  waited for; a failed source is named with its fix. Scheduler and start-with-Windows turn on here.
- Turned-in work is never a task: archived at first sight (the reveal says how many, once), later
  auto-marked done with Undo under closed work. Late work past its syllabus window leaves Today for
  "Late window closed" (Archive / Still doing it); inside the window it is labelled ("-30% now; closes
  Wednesday at 11:59 pm").
- Main page: one ordered list is the product; due times shown, never start-by; no assigned work
  times; no sacrifice suggestions; pin, never reorder; red is the brand, never an alarm; light and
  dark first-class; full keyboard.

---

## 1. Setup, one screen at a time (`one-thing-per-screen`)

**Third reference: Apple Setup Assistant.** The safe, strong baseline: the ruled order told as a
linear story, one decision per screen, and a main page that is a single document read top to bottom.

- **Flow thesis.** The student never sees more than one thing to do. Each screen is a full-bleed sheet:
  why, one button, proof. The week is *withheld* until the reveal so the reveal lands as a single event,
  like Setup's "Welcome to iPhone". Progress is counted in the student's currency ("2 of 9 connected:
  account, Knowlu installed") with real endowed progress. Optional steps get Setup's "Set up later in
  Settings" with the cost named. The reveal is Setup's transfer screen: countable per-source lines
  narrating true engine work ("Reading CHEM 231 syllabus... found the late rule"), then the week.
- **Onboarding.** Centred column, one sheet per step, a slim step map pinned at the foot (School ·
  Payment · Email · Google · Homework · Syllabi · Your week) whose items carry "reading..." or their
  proof. Credential moments get one enclosed "secure step" panel used nowhere else.
- **Main page.** **Today as one page**: the verdict sentence ("About 4.7h due by tomorrow; you have
  about 5.7h free after 6:00 pm"), then the next task large with its why, then the rest of the list
  tight beneath, greyed past today's free time; the day's commitments in a narrow ambient column.
  Sections (Needs you, Late window closed, notices) do not exist until they have something in them.
- **Notion.** Home's order (now -> soon -> my work) and hide-until-relevant sections; adjacency spacing
  (rows of a group tight, gaps between groups wide) instead of boxes; a 60-75 character measure.
- **Stripe.** "Finish later" honesty ("Knowlu works with what you've connected; 4 steps left");
  pending states with a time expectation ("usually 1-3 minutes"); Checkout's order summary, where the
  proof itself is the summary above the $9.99 button.
- **Visual world.** Campus wayfinding signage: big, calm step numerals; deep ink-blue ground in dark,
  cool off-white in light; the red block appears only in the wordmark. Hanken Grotesk throughout,
  tabular figures for hours.
- **Risk.** Familiar; the withheld week makes 8-15 minutes feel longer, because nothing visibly
  accumulates between proofs.

## 2. Connections hub, one sheet per source (`connector-hub`)

**Third reference: Plaid Link.** Onboarding is a hub of source tiles that fill in live; each source
opens a Plaid-style sheet. The main page is a list where every row carries its receipt.

- **Flow thesis.** Make *what Knowlu reads* the visible structure. The hub shows every source the
  student will connect, in the ruled order: the next one is lit, later ones are visible with their
  benefit line but marked "after payment" or "after Google". Each tile opens a sheet with Plaid's
  expectation-setting ("You'll need: your school email, password, and your phone for Duo"), then the
  school's own window (its real host shown), then a success state that writes the proof onto the tile.
  Resume is free: the hub renders from what is still outstanding (Stripe's requirements model), so a
  relaunch shows the same board with the first gap lit.
- **Onboarding.** The hub is home; sheets come and go over it. Tiles carry one of five states with a
  word and a shape: needed now, reading, done with proof, needs you, skipped (with cost). Partial
  Google grants show as one tile split by scope ("Gmail connected · Calendar not shared").
- **Main page.** **List + inspector.** The ranked list on the left; selecting a row opens a right-hand
  inspector with its receipt: why it is here, source and last read ("Blackboard, read 6:02 am"),
  estimate as an estimate, history with actor and Undo. The hub survives as the quiet "Connections"
  panel, where VHL's "signed you out this morning; earlier data stays. Sign in again" lives.
- **Notion.** Toggle children for a task's steps (one line until opened); a quiet collapsible rail.
- **Stripe.** Requirements-rendered flow and the five source states; Dashboard Inspector for detail on
  request; every failure as cause plus fix with a code underneath.
- **Visual world.** A key cabinet: slate-grey ground, tiles as matte cards with a notched edge that
  marks a credential step; course tints as soft fills. Schibsted Grotesk; Red Hat Mono for hosts and times.
- **Risk.** A hub invites out-of-order clicking that the ruled order forbids; locked tiles can feel like
  a paywall preview. The list+inspector main page leans operator-dense for an 11 pm student.

## 3. Lanes with a history (`three-lanes`)

**Third reference: Linear (My Issues, Triage, history with actor).** The main page splits by *who
started a change*; onboarding is an import whose every result is a signed history entry.

- **Flow thesis.** VISION's three-way rule (student acts at once; Knowlu noticed -> proposes; an
  authoritative source changed -> applied and listed) becomes the page's structure. Onboarding builds
  trust in that rule from minute one: every proof is written as a dated entry with its actor
  ("Blackboard · 26 assignments read", "Knowlu · 4 turned in, archived · Undo"), so by the reveal the
  student has watched ~20 attributed, reversible actions. The reveal shows each lane filled once.
- **Onboarding.** Two columns: the step on the left, a running log on the right that grows entry by
  entry and shows "reading..." rows resolving in place. Keyboard hints teach during setup (Enter to
  continue, S to skip with its cost).
- **Main page.** **Three lanes**: Today (the ordered list, pin not reorder), Triage (decisions and
  mail-found changes, accept or decline, age shown, "Not now" sends to the back), Changed (applied by
  a source or by Knowlu, actor and Undo; "Late window closed" lives here as Linear's auto-close: moved,
  named, recoverable). Single-key accelerators; every action has a visible control too.
- **Notion.** The block model: one task object rendered as a Today row, a Triage card or a Changed
  entry with the same handle and detail; Notion Calendar's minimal-UI-because-keys idea.
- **Stripe.** Status pills typed by what the reader must do (none / optional / required), always a
  word and a shape; the notification banner that renders only when action is needed, kept as a calm
  positive line when healthy ("Up to date, 8:02 am").
- **Visual world.** A lab notebook: faint graph ruling, numbered entries, every entry initialled by
  its actor; dark-first carbon ground. Hanken Grotesk body, Martian Mono for keys, times and entry ids.
- **Risk.** Three lanes can read as three inboxes, diluting "the list is the product"; Triage must be
  visibly secondary or the student starts the day clearing questions instead of working.

## 4. The checklist becomes your list (`checklist-to-today`)

**Third reference: Things 3.** Onboarding *is* a list of rows; at the reveal those rows step aside
and the student's own Today list takes their place in the same component. Bold.

- **Flow thesis.** Teach the one daily gesture by making setup out of it. Each step is a row ("Sign in
  to Halverson State"); finishing it checks the row and writes its proof as the row's note. Optional
  rows have "Not now, and lose X". "Your week" is a group of prefilled rows, one per class meeting
  (BIO 201 lecture MWF, BIO 201 lab Thursday...), each confirmed with a check or opened to Edit. At the
  reveal the setup rows fold into a single "Set up · 9 of 10, VHL skipped" line, and Today slides in
  beneath with the same row anatomy. The student already knows how to use the page.
- **Onboarding.** One list, a gentle measure, a header that counts "3 steps until your week". No hub,
  no slides; the reveal's wait shows as rows resolving.
- **Main page.** **A finite Today** with a visible fold where today's free time runs out ("about 5.7h
  free after 6:00 pm" marks the line; rows past it are quiet). Above the fold: urgent, then
  recommended. Below: This week and Later, out of sight but reachable. Done rows stay quiet for the
  day and can be unchecked. Domains as a strip of filters with hours ahead/behind.
- **Notion.** The Getting Started page made of the product itself (learn by checking rows); empty
  states that name the next action ("Nothing left today. Next up tomorrow: Lab report, about 40 minutes").
- **Stripe.** Prefill then confirm, every field editable (the Your week rows); the summary step with
  per-section edit, so any confirmed row reopens in place from the reveal.
- **Visual world.** Index cards on a desk: pale blue rules, the brand red as the single margin rule
  down the left (identity, not alarm); round check circles; generous air. Onest throughout.
- **Risk.** Setup rows and task rows sharing a component could blur "a task" with "a setup chore";
  the payment row inside a checklist can feel trivialised.

## 5. The week as a balance (`week-ledger`)

**Third reference: Shovel (time arithmetic, the cushion).** Bold. Every login adds lines to a ledger
of hours needed against hours free; the main page is a budget, not a to-do list.

- **Flow thesis.** Students' real question under "what's next" is "do I have enough time?". Answer it
  from the first proof. A right-hand ledger opens at the school step and fills as sources land: "+26
  assignments, about 31h" in, "Library job, 8h" and "6 events" out, "4 turned in, archived" struck
  through. Payment's order summary is the ledger so far. Your week reconciles it (classes confirmed
  are entered as hours committed). The reveal is the week's statement: each day's hours due against
  hours free, Friday's 10.2h called out, the cushion as one number.
- **Onboarding.** Two panes: step on the left, the ledger on the right building line by line
  (Notion's live preview, made real). Skipped sources show as a pending line with what they would add.
- **Main page.** **A budget for today**: the governing figure first (must-do 4.7h against about 5.7h
  free, the meter in 30-minute blocks that fit or spill, never red), then the list as line items with
  hours, then domains as accounts each ahead / on track / behind in hours with the way back
  ("Organic Chemistry is about 2 hours behind; Lab 6 closes it"). Next task leads the line items.
- **Notion.** The preview that builds as you answer (two panes); spacing rhythm that groups lines.
- **Stripe.** Balances shown precisely with one comparison (available vs pending -> done vs still to
  do; "1h more than a usual Monday"); tabular numerals; OKLCH steps for contrast in both modes.
- **Visual world.** A bursar statement and bookstore receipt: perforated edges, tabular figures,
  ruled totals, course tints as account colours. Schibsted Grotesk text, Azeret Mono figures.
- **Risk.** Hours-first framing can make a heavy week feel like debt; the voice must keep "rest is
  permission" and never tip into pressure. Estimates become very visible, so their honesty is load-bearing.

## 6. Departures (`departures-board`)

**Third reference: Flighty.** Bold. The main page is one next item with live status and a board of
what departs after it; changes arrive like gate changes, with a reason and a source.

- **Flow thesis.** What students fear is not the list but being surprised. Make status and change the
  structure. Onboarding builds the board: each source's proof adds "departures" to a strip of the week
  ("BIO 201 Lab 6 · due Wed 11:59 pm"), turned-in items land as "Arrived" and leave. The reveal is the
  week's board; when a source is still reading, its rows say "Boarding soon" in words, not a spinner.
- **Onboarding.** The step on top, a horizontal week strip beneath that fills from the school proof on;
  the price disclosed at account rides along as a small fixed ticket line until payment.
- **Main page.** **One live card + the board.** The card: "Lab 6 is next · due Wednesday at 11:59 pm ·
  -30% now; closes Wednesday · about 1h 30m · why: closes first and is worth the most". Below, today's
  departures in order with one status word each (On track, Changed, Window closing, Late window
  closed). A change reads like Flighty: "CHEM 231 Exam 2 room moved to Hallett Hall 120 · from your TA's email
  · Thursday". The tray flyout carries the card alone.
- **Notion.** Notion Calendar's menu bar: the tray as a one-glance "what's next" without opening the
  window; its faint-grid recipe for the week strip.
- **Stripe.** Many states collapsed to a few human labels with detail on request; Checkout's success
  checkmark as the one motion for proof and done, never an error shake.
- **Visual world.** A campus shuttle departures sign: deep night ground (and a bright day board for
  light), large tabular countdown figures, status words in a warm amber that is never red. Big
  Shoulders Text for the board, Atkinson Hyperlegible Next for everything read.
- **Risk.** Countdowns can create urgency the brand forbids; the board must count to *due*, never
  to "start by", and never tick visibly by the second.

## 7. A conversation that hands you the list (`concierge`)

**Third reference: Superhuman (the 1:1 onboarding session, taught on the user's own data).** Bold.
Onboarding is a guided conversation with Knowlu; the main page is a morning brief with the list
pinned at its top and the assistant beneath.

- **Flow thesis.** Superhuman made onboarding a live session on the user's real inbox. Knowlu does the
  same with no human: each step is an exchange (Knowlu's why line, one action chip, Knowlu's reply with
  the proof in numbers), and the gestures of daily use are taught on real items as they appear: "These
  4 look turned in. I've archived them. Undo is here if I'm wrong." "BIO 201 lab is Thursday 2:00-4:50 pm per
  the registrar. Right?" By the reveal the student has already undone, confirmed and pinned once.
- **Onboarding.** A single reading column of exchanges; the progress rail stays docked at the side so
  "reading..." results arrive as follow-up replies ("Your VHL course is in: 6 assignments, 2 done").
  Chips only; no free-text answers required.
- **Main page.** **A brief, then the list.** One short paragraph for the day (the verdict sentence,
  what changed overnight, the one decision waiting), then the ordered list pinned as a sticky block,
  then the assistant line for quick capture and "why is Spanish first?". Decisions arrive as Knowlu
  asking, with the quoted email under it. No persona name; the student is talking to Knowlu.
- **Notion.** Notion Mail's proof-from-your-inbox ("3 emails look like assignments, from 2 professors")
  and Zhao's "make the dumbest path great": open, read one line, start the first row.
- **Stripe.** Markdoc's docs-that-use-your-data (every explanation names the student's own course and
  rule: "Your ENG 210 syllabus has no late rule"); Link's recognise -> verify -> fill rhythm for the
  kept school session ("You won't sign in to these again").
- **Visual world.** A returned paper with careful margin notes: cool white paper and blue-black ink
  (not cream), Literata for Knowlu's prose, Mona Sans for chips and the list.
- **Risk.** A chat frame can bury the list and slow the 2-second test; chat-shaped onboarding invites
  the student to type, which VISION forbids beyond sign-ins and confirms.

---

## Which are bold

Safe: 1 (Setup). Middle: 2 (Hub), 3 (Lanes). Bold: 4 (Checklist becomes your list), 5 (Ledger),
6 (Departures), 7 (Concierge). Recommended five for prototyping: 1, 3, 4, 5, 6 (one safe, two
structural, two bold), with 7 the alternate if a conversational main page is wanted on the table.
