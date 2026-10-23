# Product truth for the five directions

Written 2026-10-23 for the five-direction UI run. Every direction must fit this file. It carries no
interface ideas: no layout, palette, type or component. It says what the screens are, what each one
must carry, what no design may break, how Knowlu sounds, and what went wrong on the first real
install.

Sources, in order of authority: `docs/specs/2026-10-23-onboarding-redesign-draft.md` (ruled
2026-10-23, "the spec"), `VISION.md`, `docs/design/PRODUCT-DECISIONS.md`, `docs/design/DESIGN-PRINCIPLES.md`,
`docs/design/BRAND-VOICE.md`, `docs/design/PRODUCT-BRIEF.md`, `docs/surface/anatomy.md` (the read
model), `docs/notes/2026-10-22-first-install-feedback.md`, `docs/design/log.md`. The current UI
(`app/static/`) was read for product truth only; its look is an anti-reference.

Shared sample data for every prototype: `../shared/sample-student.json` (one invented student).

---

## 1. The journey in one line

Install → **one long onboarding** (every login inside it; a benefit line before each ask, proof after
it) → **"Here's your week"** reveal → the **main page** the student opens every morning to answer
"what's next?". The main page is first shown only when it already knows what's next (VISION success 5).

---

## 2. The screens the journey needs

### A. Onboarding (one continuous, full-screen flow; about 8-15 minutes of the student's time)

Every step has three parts, always in this order:

- **Why**: one line, before anything is asked, saying what this connection does for the student.
- **The ask**: one sign-in, one consent, one payment or one confirm. Never a form beyond that.
- **Proof**: counts the student can check ("5 classes this term; 26 assignments, 4 already turned in").
  A step whose work is still running says "reading…" and the student may move on; its proof fills
  in later on a **progress rail** that stays visible to the end.

| # | Step | Why line (spec wording, Quinn reads final copy) | The ask | Proof (as in `sample-student.json`) | Skippable? |
|---|---|---|---|---|---|
| 1 | Welcome | "Knowlu answers one question every morning: what's next? Next, you'll connect the places that already know your work, about 10 minutes." | none | none | n/a |
| 2 | Your account | "Your plan lives in your account, so it's safe if this computer isn't." | Continue with Google (name and email only) **or** an emailed code; 18+ and terms | "Signed in as …" | No |
| 3 | Your school | "One school sign-in lets Knowlu read your classes, class times, assignments, grades and school email. You won't sign in to these again." | pick the school; **one** sign-in (SSO/MFA happens in the school's own window) | "5 classes this term; class times for 5; 26 assignments, 4 already turned in; grades for 5 courses; syllabi found for 4" | No; fallback is pasting the LMS feed link (costs: no grades, no registrar times, no syllabus skim, no turned-in filter, school email needs its own sign-in) |
| 4 | Payment | "Everything you just connected keeps running every day for $9.99 a month, after 7 days free." | checkout in the browser (Stripe); testers see "nothing to pay" | "Your trial is on" | No |
| 5 | School email | "Professors change dates by email. Knowlu reads what arrives and proposes each change for you to approve." | nothing new where step 3's sign-in covers it; otherwise one sign-in | "Forwarding on. Test message arrived in 38 s. Older school mail is still coming in." | Yes, cost named |
| 6 | Google: Gmail and Calendar | "Clubs, jobs and internships write to your Gmail, and your calendar says when you're already busy. One Google permission covers both, read-only." | **one** Google consent (Gmail + Calendar together, same Google app as sign-in). Forwarding offered only if declined or the read fails | "Read 40 recent messages; 1 thing to do. 6 events this week" | Yes, cost named |
| 7 | Another calendar | "If your calendar isn't Google's, add it so your free time is honest." | only when step 6 granted no calendar: paste an iCal link | "N events this week" (not shown for Maya: Google Calendar granted) | Yes |
| 8 | Homework sites | "Homework sites hold due dates your school's feed doesn't show." | zyBooks, VHL logins, only the sites step 3 found mentioned | "zyBooks: 1 book, 8 assignments, 3 already done"; "VHL: 1 course, 6 assignments, 2 already done" | Yes, per site |
| 9 | Syllabi | "Your syllabus has every exam date, how each course is graded, and the late-work rules." | only for courses with no syllabus found: upload a file (or Simple Syllabus) | "Read 5 of 5: 10 exams, weights for 5, late rules for 4" | Yes, per course |
| 10 | Your week | "Check your classes. Everything here can be changed later." | **confirm** class times, exams, weights and late rules found by the registrar and syllabi; Edit any row; typing is the last resort | the confirmed week | No as a screen; any row may be left blank |
| 11 | The reveal | "Here's your week." | Open Knowlu | see B | n/a |

Order facts a design must keep: account → school → **proof** → payment → school email → Google →
(other calendar) → homework sites → syllabi → your week → reveal. Payment sits right after the school
step's proof so value is shown before the price (Q2). The school step comes first among sources because
it starts the most background work.

**Every onboarding screen must also carry:**
- Where the student is in the flow and roughly how long is left (honest estimate).
- The progress rail: each source with "reading…", its proof, or a named failure with its fix.
- For each skippable step, **what skipping costs**, said on the step (and again at the reveal).
- Privacy in plain words at the moment it matters: portal passwords never leave this computer; Knowlu
  keeps the session, never the password; Gmail and Calendar are read-only.
- Resume: quitting and relaunching returns to the same step, with finished proof intact. A student
  who left at payment returns to payment.

**Onboarding must not ask:** a profile name, a timezone, slot times, class times typed by hand, or
anything that is not a sign-in, consent, payment or confirm (VISION: "asks for sign-ins, nothing
else"). Defaults are set silently and editable later.

### B. "Here's your week" (the reveal)

Built from the same read model as the main page, so the page then shows exactly what the reveal
promised. It carries:
- **The week**: classes, due dates, exams and the student's own events (Mon-Sun).
- **What's next**: the first item of Today's list, and **why** it is first.
- **Free time**: the honest number for today (and its windows where commitments allow).
- **What Knowlu tidied**, one line each, no lists: "52 past items and 9 already turned in were
  archived", "Effort estimated for 34 assignments".
- **Still coming**: anything still running or skipped, named ("Still reading 1 syllabus; it will
  appear on your page"; "Older school email is still coming in"; "2 things not connected").
- One button: **Open Knowlu**.
- Timing: waits up to **3 minutes** after "Your week" for first runs, then opens and names what is
  still running. Backfills are never waited for. A failed source is named with its fix.

### C. The main page ("what's next"), opened every day

What the student comes to do, most to least frequent (PRODUCT-BRIEF): what do I do now? → am I okay?
(on track, how much left, how much free time) → does anything need me? → what's coming? → how's this
class going? → is Knowlu working?

**The list is the product.** Everything else supports it. VISION's main page regions:

- **Today's list**: one ordered list. Urgent (due today or tomorrow, calendar days) first, then
  recommended work up to today's available time; everything else reachable so the student can check
  off anything. Under a **strip of domains** (each course, each club or job, Personal), each marked
  ahead / on track / behind in hours; tapping a domain filters the list. A task being avoided rises and
  says for how long.
- **Today's schedule**: classes, meetings and events on today's timeline (the full week in its own
  Schedule view).
- **Free time**: an honest number and its windows ("about 3h free: 4-5:30 and after 8"), from the
  work left and the commitments.
- **All clear**: when nothing is due, say so plainly. A free day is an answer, not an empty page.
- **Grades**: a small ring per course, filled to the percentage, coloured by letter; click for the
  breakdown. Term GPA with an uncertainty band later; hideable.
- **Lock in**: a focus session serving one task after another (pomodoro; Done, Skip, Pause, Exit).
- **Decisions**: questions Knowlu can't answer alone, with evidence, never ignorable (age tracked).
- **Late window closed**: past-due work whose syllabus late window has closed leaves Today into this
  small group, with **Archive** and **Still doing it** ("When will you turn it in?").
- **Closed this week / history**: rolling 7 days, who closed each (you, Knowlu, lapsed), with **Undo**
  on what Knowlu auto-marked done.
- **System status**: ambient; speaks up only when something changes or breaks.

**Information each task row can carry** (what to show where is the design question): title; course
or domain; due date **with a time**; estimate (as an estimate, possibly a range); time spent;
progress; importance; source (Blackboard, zyBooks, VHL, Gmail, Outlook, the student, Knowlu); steps;
notes; **why it is placed here**; urgent / recommended / later; flags (a proposed change from a
source, a conflict with a commitment, a mismatch like "you: 100% · zyBooks: 60%"); late label inside
the window ("−30% now; closes Wednesday"); pinned. Every field is editable in place; what Knowlu worked
out (original estimate, why, source, time spent) is visible but not editable.

**Read-model facts the page draws from** (anatomy.md; names are the engine's):
- `verdict`: one generated sentence for the day plus its two figures (must-do hours, capacity hours).
  Never a fixed string; never disagrees with the visual.
- `meter`: does today fit? 30-minute blocks that fit vs. blocks that spill, hoverable to name the tasks.
  (Red must not be the spill signal any more; see §3.)
- `must_do` / `recommended`: the ranked rows, every count with hours ("3 tasks · 4.5h", never a bare
  count, so a course can't hide 35 hours behind 3 tasks).
- `the_day`: today's commitments and open blocks; all-day lane; recurring commitments shown as
  commitments.
- `ahead`: 14 days of hours due per day, exams, a generated takeaway ("Thursday is the heaviest day in
  the next two weeks").
- `coming_up`: next five accepted events.
- `decisions`: the deck (one card at a time; the page never grows with the queue); header with
  pending count and oldest age. Kinds: event-accept ("Required" / "Worth a look"), event-check,
  commitment-check ("Is this part of your week?"), commitment-ask, amend (a date change), mail-found
  task.
- `good_to_know`: notices that are neither task nor decision (room change, grade posted, late/missing).
- `closed_this_week`: me / agent / lapsed marks.
- `grades`: per course `pct`, `letter`, `basis`, `earned/possible`, `pending` (items waiting to be
  graded), `fetched_at`, entries by category.
- `delta`: what changed since the last run ("2 marked done by Knowlu · CHEM 231 date moved").
- `topline` sync/backup/last check: in plain words ("in step with your account", "offline — changes
  stay on this computer", "Blackboard stopped answering. Knowlu will try again at 6:00").
- Every region always renders; empty states are designed and named ("Nothing is at risk today.",
  "Queue clear").

**Other surfaces the main page links to** (on request, not foreground): Schedule / week view,
course overview (grade, pace in hours with how to catch up, exam dates listed plainly, work left this
week), club/job overview, calendar view, task drawer (all fields, history, report a problem, delete
with optional reason), Settings (connected sources with each one's Connect/Forget, "School sign-in:
kept on this computer, used for grades, class times and school email. Forget."), quick capture,
the assistant.

---

## 3. Rulings and constraints a design must not break

**From the ruled onboarding spec (2026-10-23)**
1. One long onboarding; every login inside it; why before each ask, proof after it.
2. One school sign-in covers LMS, course list, syllabi, registrar, grades and school Outlook where
   the school shares a login. Never show four or five separate school sign-ins.
3. Payment after the school step's proof. $9.99/month, 7 days free, no free tier (also: $39.99 a
   6-month term; Pell Grant recipients 75% less). Testers see "nothing to pay".
4. Gmail by Google permission first, **one consent** for Gmail + Calendar. Forwarding only on
   decline, failure, or past the first 100 permission users. Gmail permission brings 7 days of history;
   say so in one line.
5. Class times come from the registrar and syllabi and are **confirmed**, never typed (typing is the
   last resort for a course nothing covers). Multiple meetings per class are normal (lecture + lab +
   recitation).
6. Past-term courses never appear.
7. Turned-in work is never shown as a task: archived at first sight on a new vault; afterwards
   auto-marked done by Knowlu **with Undo**, listed under closed work. A student's own hand-set status
   is never overridden.
8. Late work past its syllabus late window leaves Today into **"Late window closed"**; archiving is
   the student's tap; "Still doing it" sets a new due date. Inside the window the task stays and is
   labelled ("−20% now; closes Friday").
9. Optional sources skippable with their cost named; account, payment, school sign-in and Your week
   are not.
10. The reveal waits up to 3 minutes, then names what is still running.
11. Resume after quit, at the same step.
12. The scheduler and "start with Windows" turn on only at the reveal.

**From VISION and the signed design**
- **Fetch on device, think in the cloud.** Portal credentials (school, zyBooks, VHL) never leave the
  student's machine; Knowlu keeps a session, never a password. The vault is **plain text on the
  student's machine** (`%USERPROFILE%\Knowlu\`); the account holds the synced copy.
- Nothing the student sees depends on the network: the page works offline.
- **Who started a change decides how it lands.** The student → acts at once, with undo. Knowlu noticed
  (an email) → proposes and waits, showing who it came from. An authoritative source (LMS, registrar)
  changed → applies and lists under "what changed". Exception: mail from before setup is applied and
  listed under "From your email", each with Undo. A notice that changes nothing (a room change) is
  shown, not proposed. Knowlu never invents a due date: "needs a date".
- The AI never drafts anything the student sends to someone else.
- Commitments have a kind (class, lab, work, club, meeting, office hours, exam) and a level: **hard**
  (never overlapped), **soft** (may be overlapped, says so), **optional** (never busy time).
- Calendars are read-only in Knowlu.
- Desktop only, Windows, Tauri webview; small windows to large monitors.

**From PRODUCT-DECISIONS (Quinn)**
- Urgent = due today or tomorrow (calendar days).
- **No start-by dates shown.** Show due, always with a time. (The engine still ranks by start-by; the UI
  stops showing it, log D3.)
- No assigned work times; no sacrifice suggestions (never "skip the club", never "stay up").
- No manual reordering; **pin** instead (pin lasts until done).
- Steps: checking all steps never completes the task; Knowlu asks.
- Deleting asks first with an optional reason; "report a problem" with a fixed reason list.
- Completed tasks stay visible for the day and can be unchecked.
- Decisions must not be ignored; their age is visible; "Not now" sends one to the back; answers take
  effect at once, with undo; evidence (quoted email, untrusted) always available.
- 2-3 nudges a day (VISION amendment pending), quiet hours.
- Focus sessions: pomodoro, two clocks (pomodoro and per-task time), Done / Skip / Pause / Exit.
- A Personal domain always exists; clubs and jobs become domains.
- Earlier design-session choices (log D4, D5): keyboard shortcuts as optional accelerators (no command
  palette); "next, then the rest" (one large next task, a quiet list below) with tasks beyond today's
  free time greyed. These were taken under the old structure and are open to being beaten, not ignored.

**From DESIGN-PRINCIPLES (the test every screen must pass)**
- Clarity and restraint; one idea per surface; three levels of visibility (foreground, ambient, on
  request); position carries meaning; every element carries meaning.
- Show amounts, say reasons: quantities get a visual form; reasons and requests get a sentence.
- **Red (#BC4444) is the brand, never an alarm.** It never marks behind or busy. "Broken" gets a
  non-red signal. No alarm styling for being behind.
- Colour never the only signal; full keyboard; visible focus; screen readers; reduced motion; light
  **and** dark both first-class (Quinn works in dark).
- Readability is not negotiable: no faint text.
- Prefer undo over confirmation.
- The 11 pm test: next action in 2 seconds? okay-or-not without reading a paragraph? nothing useless
  on screen? nothing that makes them feel worse than the facts warrant?

**Run hard rules**: invented data only; no network from prototypes except Google Fonts; brand mark is
`docs/design/assets/Knowlu_Wordmark_Red.png` (serif "Know" near-black, "lu" on a red block).

---

## 4. Brand voice in brief

- Knowlu is **the study partner who is already locked in**: motivates by example, not pressure. Test:
  nobody feels anxiety reading it.
- Lead with what to do next. "Spanish practice is next. About 1 hour."
- Numbers over adjectives. "Organic Chemistry is about 2 hours behind." Never "way behind".
- Every hard truth comes with the way back. Frank, never threatening.
- No guilt, no excuses: never "overdue!", "don't forget!", "don't worry", "no pressure".
- Rest is permission: "You're ahead for the week. Stopping here is fine."
- Estimates are estimates ("about 40 minutes"); never recommend sacrifices; when more is due than
  time, say so plainly and leave the choice.
- Due times, not start times ("Due tomorrow at 9:30 am").
- Quiet confidence: short sentences, few exclamation marks, no emoji, a nod not a parade ("Done.
  That's the last one today.").
- Failures plainly with what happens next: "Blackboard stopped answering. Knowlu will try again at
  6:00."
- Privacy in plain words: "Knowlu keeps the session, never your password."
- Mechanics: "Knowlu" (never KnowLu); "you"; contractions; sentence case everywhere; "5:00 pm" with
  lowercase am/pm; "today", "tomorrow", weekday within a week, "November 6" beyond; "about 40
  minutes" in sentences, "40m" / "1h 15m" where tight; numerals for counts and grades; course code
  where clear (CHEM 231), name in sentences about how a class is going.
- Avoid: hustle, grind, crush, productivity, no pressure, don't worry, oops, simply, just, easy,
  overdue (as alarm), snooze (use "Not now").

---

## 5. What Quinn hit on the first real install (2026-10-22)

Each is a failure a new design must make impossible or visibly handled.

| # | What happened | What the design must do |
|---|---|---|
| 1 | Asked for Gmail forwarding before (instead of) Gmail permission; Google consent only through the unverified-app "Advanced" link | Google consent first, one screen for Gmail + Calendar; forwarding only as the fallback. Prepare the student for Google's warning screen while the app is unverified |
| 2 | Told to set up Outlook forwarding by hand though it already forwarded to the Knowlu address | Recognise "already set"; never ask for work that's done |
| 3 | Mail summary listed "Bring in recent mail" and "Finish" as not done after "I've done this"; "Mail before 24 June was not brought in" | Proof must be true; never contradict what the student just did; say plainly what is still coming (older mail) |
| 4 | "See what came from your email" did nothing (likely a covered popup) | Every button gives visible feedback; overlays never hidden behind other windows |
| 5 | Only one meeting time per class, and typed | Class times from registrar and syllabi, confirmed; multiple meetings per class |
| 6 | Past-term classes listed | Current term only |
| 7 | Blackboard connected **after** onboarding, as a separate step | Every login inside onboarding; one school sign-in for everything it can cover |
| 8, 9 | Old and already-turned-in assignments arrived as live tasks (28 live; 59 past archived) | Turned-in work archived at first sight; the reveal says how many, once, without listing them |
| 10 | Effort estimates absurd (median 8 h, max 24 h, ~155 h total) | Realistic estimates (reading 0.5h, homework 2h, essay 3h, exam study 4h); estimates shown as estimates, correctable in place |
| 11 | Late work not weighed against the syllabus late policy | "Late window closed" group; late label inside the window |
| 12 | Inbox never read; Gmail/Calendar reads failed silently | A source that fails is named with its fix, on the rail and at the reveal; never silence |
| 13 | "Check for updates" and "Update now" gave no feedback; needed two presses | Every action acknowledges at once (pressed, working, done, failed) |

The theme across all thirteen: **the student was made to micromanage and could not trust what the
screen said.** The new flow must earn trust with true proof, few asks, and a first main page that
needs no clean-up.

---

## 6. The shared sample student, in brief

`../shared/sample-student.json` holds one invented student, **Maya Okafor**, a sophomore biology
major at the invented **Halverson State University** (Blackboard, school Outlook, one shared school
sign-in). Every number in it is computed from its raw data, so the counts agree everywhere.

- **Two moments.** Onboarding and the reveal: Sunday October 18, 7:05-7:19 pm (week 8); the reveal
  shows Monday October 19 to Sunday October 25. The daily main page: Monday October 26, 8:15 am
  (week 10). A week of use between them is what makes "Closed this week" with Undo, "Late window
  closed" and an in-window late label real.
- **Courses:** BIO 201 (lecture MWF + Thursday lab), CHEM 231 (TTh + Wednesday recitation; Exam 2
  Thursday October 29, room moved by email), CS 110 (zyBooks; syllabus uploaded at step 9), SPAN 202
  (VHL), ENG 210 (no late rule in its syllabus). Two Spring 2026 courses are `show: false` and appear
  nowhere else.
- **Commitments:** library job Tuesday 5-9 pm and Saturday 10-2 (work, hard); Pre-Health Society
  Wednesday 6:30-7:30 pm (club, soft); intramural soccer Sunday (optional); office hours (optional);
  awake 7:30 am to 11:30 pm.
- **45 assignments listed** (plus 52 earlier items archived at setup only as a count): 9 turned in
  before setup (archived), 6 marked done by Knowlu since (Undo), 2 checked off by Maya, 2 in "Late
  window closed", and 26 live now, one of them inside its late window (Lab 6, "−30% now; closes
  Wednesday at 11:59 pm").
- **Today:** about 4.7h due by tomorrow, 6.5h of work planned, about 5.7h free after 6:00 pm; Lab 6
  is next. Friday is the heaviest day of the next two weeks (about 10.2h).
- **Mail:** 6 emails (3 Gmail, 3 school Outlook): an extension granted (proposed date change), a TA's
  date change (proposed), a room change (shown, not proposed), a club panel (event question), a
  research fellowship (task question, waiting 7 days), club dues (applied at setup, with Undo).
- **Live failure to design for:** VHL signed Maya out this morning; its earlier data stays.
