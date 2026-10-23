# Landscape: "what's next" and planning UIs

Research for the 2026-10-23 directions run (five directions, each from Notion + Stripe + one third
reference). This file is about **structure, not looks**: for each product, the idea that answers "what
should I do next?", and how it handles overload, time and trust in automatic decisions. At the end are
eight candidate third references, each named with what it would contribute to Knowlu.

Ruled facts this is checked against: `docs/specs/2026-10-23-onboarding-redesign-draft.md` (one long
onboarding with every login inside it, a benefit line before each login and proof after it; account,
then school sign-in, proof, payment, Gmail, calendar, homework sites, syllabi confirmed, then "Here's
your week"; skippable sources with the cost named; resume after quit; turned-in work never shown as a
task; "Late window closed"), plus `VISION.md` and `docs/design/PRODUCT-DECISIONS.md` (no start-by dates,
no assigned work times, no sacrifice suggestions, pins instead of hand reordering).

Source note: facts marked (verified) were checked against the linked page on 2026-10-23. Others are
the long-standing, widely documented behavior of the product; check the link before quoting it as a
spec.

## 1. Products

### Things 3 (Cultured Code)
- **Structural idea.** "Today" is a short list the user can actually finish. Anything with a date
  arrives there on its own, and **This Evening** splits the same day into now and later without
  needing clock times. Upcoming and Someday hold the rest, out of sight and still reachable.
- **Overload.** Items you are not doing yet leave Today (Someday, a later date) and come back
  without a backlog that keeps growing. The interface stays calm because the list is small.
- **Time.** Days and day-parts, never slots. That is close to Knowlu's "no assigned work times" rule.
- **Trust.** It makes almost no automatic decisions, so trust comes from being predictable: a dated
  item shows up on its date, and nothing else moves.
- **Knowlu lesson.** Coarse time bands ("today / this evening / this week") answer "what's next"
  without a schedule. Completed items stay visible and quiet for the day.
- URL: https://culturedcode.com/things/ and https://culturedcode.com/things/support/articles/2803573/

### Linear (My Issues, Triage, cycles)
- **Structural idea.** "My Issues" is one list sorted by a stated rule. **Triage** is a separate
  inbox for anything someone else started, and it has to be accepted, merged or declined before it
  joins the workflow (verified). Keyboard first: every action is one key, and Cmd-K reaches the rest.
- **Overload.** Work is bounded in time (cycles). **Auto-close** and **auto-archive** (verified: the
  delay is configurable, archived issues stay searchable and can be restored) remove stale work
  without deleting it.
- **Time.** Cycle boundaries, not calendars. Unfinished work rolls into the next cycle visibly
  instead of piling up silently.
- **Trust.** Every automatic move appears in the issue's history with its actor ("Linear
  auto-closed…"). Triage rules run in a stated order (verified).
- **Knowlu lesson.** VISION's three-way rule for how a change lands (student asked, Knowlu noticed,
  authoritative source changed) maps closely onto My Issues, Triage and an applied-with-history
  change. "Late window closed" works like auto-close: the item moves and stays recoverable.
- URL: https://linear.app/method, https://linear.app/docs/triage, https://linear.app/docs/archive-issues

### Sunsama
- **Structural idea.** The plan is a **ritual**, not a screen. A guided step-by-step morning plan
  covers the steps verified on its help page: reflect on yesterday, choose today, set aside what can
  wait, finalize. An evening **shutdown** closes the day against that plan.
- **Overload.** Estimates are required while planning. It **warns when the day is overcommitted**
  against a daily-hours limit (verified), and the user chooses what moves.
- **Time.** It time-blocks onto the calendar. Knowlu does not do this (no assigned work times).
- **Trust.** Little automation. The user makes every decision, and the app counts the hours.
- **Knowlu lesson.** "Here's your week" can be a ritual moment with a beginning and an end, not a
  dashboard. The overcommit warning says what to keep while making no sacrifice suggestion: Knowlu
  states its estimate and what is due and stops there.
- URL: https://www.sunsama.com/features/daily-planning-and-shutdown, https://help.sunsama.com/docs/daily-planning

### Structured
- **Structural idea.** The day is **one vertical timeline**. Fixed events and tasks share it, and
  the gaps between them are drawn as real space. "Now" is a marker moving down the line, so the next
  thing is literally the next thing below it. Undated tasks wait in an inbox beside the line.
- **Overload.** It is visible as a timeline running past bedtime. There is no automatic triage.
- **Time.** Clock-first, explicit durations.
- **Trust.** Not applicable (manual).
- **Knowlu lesson.** Drawing free time as space between hard commitments is a direct visual for
  VISION's "about 3h free: 4 to 5:30 and after 8". Use the timeline for commitments only, and keep
  tasks out of the slots.
- URL: https://structured.app/

### Motion
- **Structural idea.** An AI fills the calendar with tasks automatically, by priority and deadline,
  and re-plans whenever anything changes.
- **Overload.** It flags tasks that will miss their deadline.
- **Time.** Every task gets a slot.
- **Trust.** This is the cautionary tale. Reviews repeatedly name **a plan that keeps reshuffling
  itself** and rigidity in auto-scheduling as the main complaint, even when they praise the engine
  (verified across several reviews; several reviewers are competitors).
- **Knowlu lesson.** An anti-reference for the main page, and it confirms the ruled "no assigned work
  times". Auto-placement destroys trust when the user cannot predict it or see why it happened.
  Knowlu's deterministic order and visible reasons ("why it's here") are the answer to that.
- URL: https://www.usemotion.com/, critique: https://www.morgen.so/blog-posts/motion-review

### Akiflow
- **Structural idea.** A **universal inbox** gathers tasks from many tools (Gmail, Slack, Jira,
  Todoist…) into one list (verified), which the user then drags onto the calendar. A command bar
  drives everything.
- **Overload.** Snooze and plan by keyboard. The inbox is processed to zero.
- **Time.** Manual time-blocking.
- **Trust.** Every item shows which tool it came from.
- **Knowlu lesson.** Source badges on every item, plus a single place where everything Knowlu found
  shows up first. For Knowlu that place is mostly invisible, because the engine merges sources itself.
  The onboarding proof step ("found 5 classes, 23 assignments") is the inbox moment done once.
- URL: https://akiflow.com/, review: https://www.morgen.so/blog-posts/akiflow-review

### Todoist
- **Structural idea.** "Today" plus overdue, sorted by priority (P1 to P4). Natural-language quick
  add ("essay draft fri 5pm p1"). The **Upcoming** view shows a week as columns of days.
- **Overload.** Overdue items pile up at the top in red, with **Reschedule** as one bulk action. Karma
  and streaks offer daily and weekly goals.
- **Time.** Dates, optional times, no slots.
- **Trust.** Manual. It added "smart schedule" suggestions in past releases, which were optional and
  shown for the user to accept.
- **Knowlu lesson.** A red overdue pile is the anti-pattern that "Late window closed" exists to
  avoid. Quick add maps to "the student asked: act at once, with undo".
- URL: https://www.todoist.com/help/articles/plan-your-day-with-the-today-view-UVUXaiSs

### Superhuman
- **Structural idea.** Onboarding was a **mandatory 1:1 session** for years, run on the user's own
  inbox. The product was then rebuilt so the in-app flow reproduces that session (verified: First
  Round, Vohra). The core loop (inbox zero by keyboard) is taught by doing it on real mail, never with
  a tour.
- **Overload.** Split Inbox separates "important" from everything else. Reminders bring a thread
  back. Auto-summaries.
- **Time.** Speed is the promise ("every action under 100 ms").
- **Trust.** It earned trust by being fast and by teaching the user its decisions in person.
- **Knowlu lesson.** A long onboarding is acceptable when every screen does real work on the
  student's own data. Each login's proof ("found 9 already turned in") is Superhuman's "look, that's
  your real inbox" moment. Teach the one daily gesture (check off, pin) during the reveal.
- URL: https://review.firstround.com/superhuman-onboarding-playbook/

### Apple Setup Assistant (iOS / macOS)
- **Structural idea.** A long, linear setup with **one decision per screen**. Each screen opens with
  a plain line on why it matters (Face ID, Location, Screen Time). Every optional step offers
  **"Set Up Later in Settings"**, which names where to find it again. It resumes after a restart.
  Data transfer runs in the background with a stated remaining time.
- **Overload.** None to manage: it is a sequence.
- **Time.** An honest progress bar and estimate during transfer. "This may take a few minutes."
- **Trust.** Privacy screens (the "Data & Privacy" icon) explain what leaves the device.
- **Knowlu lesson.** This is almost exactly the ruled onboarding shape: a benefit line before each
  login, skippable with the cost named, resume after quit. The "waits up to 3 min, then names what
  is still running" reveal is Setup's transfer screen. Its privacy pattern carries "portal
  credentials never leave the device".
- URL: https://support.apple.com/guide/iphone/turn-on-and-set-up-iphone-iph3277f6/ios, https://www.apple.com/legal/privacy/data/

### Arc (The Browser Company)
- **Structural idea.** The browser takes things away so the user doesn't have to. Unpinned tabs
  **auto-archive after 12 hours** (configurable) and stay recoverable in Archive. Pinned items stay.
  Onboarding imports from the old browser and **shows what it brought over**.
- **Overload.** Handled by decay with a safety net.
- **Time.** An implicit "today" (the archive window).
- **Trust.** It is recoverable and the user can see it happen, so automatic tidying feels safe.
- **Knowlu lesson.** Pins that stay plus tidying that happens on its own matches Knowlu's pins and
  "Late window closed" / turned-in work archived with Undo. The import-proof onboarding matches the
  ruled proof after each login.
- URL: https://resources.arc.net/hc/en-us/articles/19231060187159-Auto-Archive-Tabs (Arc is in
  maintenance; Dia succeeds it, and the pattern stands)

### Duolingo
- **Structural idea.** Onboarding asks **why** and **how much** (goal, minutes a day) and runs a
  placement test, so the first screen after setup is already personalized. The main screen is a
  **single path**: one glowing next node. There is never a menu of choices.
- **Overload.** None. One node at a time. Streak repair forgives a missed day.
- **Time.** A daily goal in minutes. Reminders.
- **Trust.** Cheerful and celebratory. It leans heavily on streak pressure.
- **Knowlu lesson.** "One next node" is the purest answer to "what's next", and Lock in already
  follows it. Reject the guilt and streak mechanics (they contradict the calm, honest brand and "no
  sacrifice suggestions").
- URL: https://blog.duolingo.com/new-duolingo-home-screen-design/

### Shovel (student study planner)
- **Structural idea.** **Time arithmetic for the semester.** Hours needed (from estimates) are
  subtracted from hours available (from the calendar). A "Time Cushion" says ahead or behind
  (verified: review and Purdue overview). Coursework is imported from the LMS (Brightspace).
- **Overload.** The cushion goes negative, and it is the student's job to react.
- **Time.** Per-task estimates. Available time from the linked calendar.
- **Trust.** Students type estimates themselves, and the result is only as good as those numbers.
- **Knowlu lesson.** This is the direct student-market precedent for VISION's free-time number and
  the ahead / on track / behind domains. Knowlu's step forward is that the estimates and imports
  come from judgment and portals, not typing. The weekly reveal can show the cushion for the week.
- URL: https://www.shovelapp.io/, https://www.purdue.edu/teaching-learning/programs/sst/assets/Shovel---Overview.docx

### MyStudyLife
- **Structural idea.** The student timetable is the spine: classes as rotation-aware weekly
  recurrences, and tasks and exams attached to a class.
- **Overload.** Lists of tasks by due date. Minimal.
- **Time.** The class timetable is **typed by hand**, which is the setup cost Knowlu removes (class
  times come from syllabi and the registrar).
- **Trust.** Manual.
- **Knowlu lesson.** Attaching every item to a course is right, and Knowlu's domain strip already
  does it. Manual timetable entry is the anti-reference for the onboarding.
- URL: https://mystudylife.com/

## 2. Cross-cutting findings

1. **Products that answer "what's next" well keep the visible list finite** (Things Today, Duolingo
   path, Linear My Issues). Overload is solved by **moving things away recoverably** (Arc archive,
   Linear auto-close, Things Someday), never by a growing red pile (Todoist overdue).
2. **Automatic decisions are trusted when they are predictable, attributed and reversible.** Linear
   (history plus actor), Arc (archive), Superhuman (taught in person) earn it. Motion (reshuffling it
   doesn't explain) loses it. Knowlu already has the principle in VISION's three-way rule. The design
   has to make the actor and the Undo visible on the page.
3. **Time without slots works**: day-parts (Things), cushion arithmetic (Shovel), free gaps drawn as
   space (Structured). That is the room Knowlu has under "no assigned work times".
4. **Long onboarding is acceptable when every step does real work and shows its result** (Apple
   Setup, Superhuman, Arc import). The ruled benefit-then-proof pattern sits at the intersection of
   those three.
5. **A ritual beats a dashboard for the weekly view** (Sunsama planning and shutdown). "Here's your
   week" is one, and later weekly or daily check-ins could reuse its frame.

## 3. Eight candidate third references

Each would sit beside Notion (calm document surface, blocks, quiet chrome) and Stripe (trustworthy
setup flows, precise numbers, explained money). One per direction; five of the eight get picked.

| # | Candidate | Structural idea it contributes to Knowlu |
|---|---|---|
| 1 | **Apple Setup Assistant** | Onboarding as one decision per screen. Each login opens with a "why" line, has a "Set up later" exit that names the cost, resumes after quit, and ends on an honest progress or transfer screen that becomes the 3-minute reveal wait. |
| 2 | **Linear** | Three lanes that match VISION's change rule: My Issues (the ordered list), Triage (what Knowlu noticed, accepted or declined), history with the actor (what an authoritative source changed). Keyboard first and calm. Auto-close becomes "Late window closed". |
| 3 | **Things 3** | A finite Today split into day-parts (now / this evening) instead of time slots. Upcoming and Someday stay out of sight but reachable. Done items fade but stay for the day. The calmest possible daily page. |
| 4 | **Sunsama** | "Here's your week" and an optional daily start as a guided ritual with a beginning and an end. Overcommitment stated as numbers (estimate vs. free time) with no sacrifice suggestion. |
| 5 | **Superhuman** | The onboarding is the tutorial: each step works on the student's real, just-fetched data and teaches the one daily gesture by doing it. Speed (instant actions, keyboard) is part of the promise. |
| 6 | **Arc** | Pinned items stay and everything else tidies itself away recoverably. Turned-in work is archived with Undo and late items leave Today. Import-proof onboarding ("here's what we brought over"). |
| 7 | **Shovel** | The week as arithmetic: hours needed against hours free gives a cushion (ahead / on track / behind) per course and for the week. Student-native precedent for the free-time number. Knowlu fills it from portals, not typing. |
| 8 | **Flighty** (not in the brief's list; proposed) | A "what's next" for travel. It ingests from email automatically, shows one next item with live status and an honest reason when something changes ("delayed: inbound aircraft late"), and timeline cards count down. It models how to show automatic, source-attributed updates ("what changed") with calm confidence. URL: https://flighty.com/ |

Considered and not proposed as a third reference: **Motion** (an anti-reference for slot scheduling
it doesn't explain), **Todoist** (its overdue pile is an anti-pattern; quick add is already covered
by Linear's Cmd-K), **Duolingo** (its "one next node" lives inside Lock in, but the streak and guilt
mechanics conflict with the brand), **Structured** (its free-gaps timeline is a component idea for
any direction's schedule strip, not a whole direction), **Akiflow** (source badges are a component
idea), **MyStudyLife** (a manual-entry anti-reference).
