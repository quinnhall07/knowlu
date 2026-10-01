# Knowlu: What's next?

What Knowlu is and must stay, so near-term decisions don't foreclose it. **Check every design against
this file.** It states what holds now; how each point was decided lives in
`docs/specs/2026-09-09-knowlu-cloud-design.md` (signed, with its amendments; the latest, of
2026-09-29, signed 2026-09-29), the product and business plan
(`docs/notes/2026-09-01-product-and-business-plan.md`, "the plan"), and git history.

## The promise

A student opens Knowlu and asks one question: **what's next?** Everything they need to answer it is
in front of them. A class or a meeting in an hour. A test a week out they should start studying for.
Three assignments due tonight. Or nothing due at all — a clear conscience to spend the day freely.

Knowlu gets there from the sources that already know the student's obligations, without manual
entry, and it learns how that student works.

**The test for every feature:** does it make "what's next?" clearer, more complete or more honest?
A feature that does not is cut or deferred, however good it is on its own.

**Who it is for:** undergraduates first, especially those carrying a full load plus clubs, work or
research. Graduate students are served by the same product; nothing is built for them alone yet.

## Design commitments

A design that breaks one of these is wrong even if it is otherwise better.

1. **The engine is deterministic.** Same input, same output, no inference inside it. Judgment happens
   at the boundary, in our judgment service, and enters the engine as data. `rank` never calls a model.
2. **The account is the source of truth; every desktop holds a plain-text mirror the engine ranks
   offline.** The cloud stores, judges, syncs and bills. The service can read what it stores
   (encrypted at rest, not end-to-end), the privacy page says so, and deleting the account deletes it.
   Nothing the student sees depends on the network being up.
3. **The AI never produces anything the student sends to someone else.** No drafted emails, notes or
   messages. Its generated output is internal: proposals, explanations and planning advice.
4. **Time is typed by commitment.** Every confirmed commitment is a note in the vault with a kind
   (class, lab, work, club, meeting, office hours, exam, …) and a level: **hard** (never overlapped),
   **soft** (a proposal may overlap it, and says so) or **optional** (never busy time). Knowlu never
   proposes over a hard commitment. Detail: `docs/specs/2026-09-23-commitment-model-design.md`.
5. **Who started a change decides how it lands.**

   | Who started it | What Knowlu does |
   |---|---|
   | **The student asked** (quick capture, the assistant, an edit) | It acts at once, with undo |
   | **Knowlu noticed** (an email, an announcement, an inference) | It proposes and waits, showing who it came from and a short summary of the context needed to judge it |
   | **An authoritative source changed** (the LMS feed moves a due date, the registrar) | It applies the change and lists it under "what changed" |

      **One exception, at setup:** mail from before a mailbox was connected, read once when it is connected, is applied and listed under "From your email", each item with Undo; what conflicts with the student or with an authoritative source is proposed. Mail after setup is proposed like anything else Knowlu noticed, an LMS's own notification email included; a notice that changes nothing in the plan (a room, office hours, a mailbox gone quiet) is shown, not proposed. *(amended 2026-09-30, email spec A10, signed by Quinn)*

   Anything that another person can see stays a proposal whoever started it. Knowlu never invents a
   due date: a missing one is marked "needs a date" and asked for.

## The main page

One screen answers "what's next?":

- **Today's list** — one ordered list of what to do, so the next thing never takes thought, under a
  **strip of domains** (each course, each club or job, and "other"), each marked ahead, on track or
  behind. Tapping a domain filters the list. A task being avoided rises and says for how long.
- **Today's schedule** — classes, meetings and events on today's timeline. The full week lives in
  its own Schedule view.
- **Free time** — an honest number and its windows ("about 3h free: 4–5:30 and after 8"), computed
  from the work left and the commitments. It shrinks when the student falls behind and grows when
  they get ahead.
- **All clear** — when nothing is due, Knowlu says so plainly. A free day is an answer, not an empty
  page.
- **Grades** — a small ring per course, filled to the current percentage and coloured by letter;
  clicking it shows the breakdown. A term GPA with an uncertainty band that narrows as more of the
  term is graded, and a projected cumulative GPA when the student has entered their prior credits.
  A student can hide grades.
- **Lock in** — for a focused session, Knowlu serves one task after another until the day's work is
  done.
- **Ahead of time** — a test a week out becomes study sessions placed before it; a large assignment
  becomes steps. Getting ahead is visible and rewarded, as more free time.

Knowlu does not let up until the work is done: behind domains stay marked, and up to **two nudges a
day** reach the student when a deadline is at real risk at the current pace or a task keeps being
avoided. Nudges have quiet hours and a one-click snooze. If students dismiss most nudges, the rule
is wrong.

## Where it gets its information

- **The LMS feed** (the student-generated `.ics` for Blackboard or Canvas), not an LMS API. No admin
  approval, scoped to one student. Canvas personal access tokens are rejected on FERPA grounds.
- **Grades from the LMS grade page and from homework platforms** (zyBooks, VHL, Pearson, McGraw-Hill
  Connect and the like), fetched on the device with the student's own sign-in; manual entry fills any
  gap.
- **Syllabi, uploaded by the student** at onboarding and whenever a term starts: class meetings, exam
  dates, grade weights and the grading scale. Uploading a file is not typing; everything Knowlu reads
  from one is shown for the student to confirm.
- **Calendars**: Google Calendar and Outlook / Microsoft 365, read-only. **Email**: any mailbox that can forward, through the student's own private Knowlu address (school Microsoft 365 and personal Gmail set up automatically, others with guided steps), and Gmail directly by Google's permission. *(amended 2026-09-30, email spec A7, signed by Quinn)*
- **Campus event feeds**, judged against the student's level, role and interests.
- **The registrar**, where a school offers it, through the school's own sign-in window.
- **The student**, through quick capture and the assistant.

Adding a source is one action (a sign-in, a URL or a file), and a source that breaks later fails
visibly rather than going quiet.

## The assistant and quick capture

**The assistant turns what the student says into changes to their plan, and explains the plan.** Two
jobs: capture ("stats hw due Friday", "break down my research paper", "I'm sick today") and explain
("why is Spanish first?", "how much free time do I have Thursday?"). It gives planning advice — how
to approach and split an assignment — and short study tips. It never does the work: no course
content answered, no essays or solutions written.

- A small router sends each message to one of a few **fixed flows**, each with its own prompt, the
  actions it may take and a strict output schema. Rules handle what they can before any model does.
- Each flow is **pinned to the smallest model that passes its evaluation**, as judgment kinds are.
- Every change the assistant proposes appears as a **preview card**; one tap confirms it, and it is
  written through the engine's journaled `write` like any other agent edit.
- Content from emails and announcements is data to the assistant, never instructions; a card is
  the only way anything it reads becomes an action.

**Quick capture:** a global hotkey and a button open a small box anywhere on the desktop. Typed or
dictated (Windows' own dictation first; our own speech-to-text only if that proves too clumsy), the
student's words become a task at once, with undo. What is missing is inferred for now — the course,
the domain, a duration — and Knowlu asks for the rest later, in the app or with a nudge. A due date
is never guessed.

Big or loosely defined work (a research paper, "figure out internships") goes through the assistant:
a short conversation, then steps or a time budget the student confirms.

## Learning

Knowlu learns each student separately: how long their kinds of work really take, when they work
best, what they care about (interests decay and are explored, never driven to zero; a declined
recurring opportunity stays declined across the series) and how far ahead they like to stay in
each domain. What it learns from one student's email trains only that student's model.

## Grades

Grade maths is deterministic: the syllabus's weights and scale, the scores, the credit hours. Grades
may raise the priority of a slipping class's work a little, weighted by each category's share of the
grade and fading as the grade recovers; they never dominate the order. Grades sync with the account
like everything else, are named on the privacy page, and never enter telemetry.

## The calendar

Knowlu's own calendar, in the app, is the complete picture: commitments, events, planned work and
free time. Later it will also write that picture to **a dedicated "Knowlu" calendar inside the
student's Google or Outlook account**, so it reaches their phone. Knowlu never edits the student's
other calendars or anyone else's events; changing those stays a proposal.

## The product

- **A Windows desktop app**, Rust + Tauri, OS-native webview. No web app and no mobile app, and
  nothing builds toward either. A phone may follow once the desktop product is proven; that takes a new signed amendment. The account carries a student between desktops. Base app under 50 MB,
  ideally under 20.
- **One edition: $9.99 a month, an account required**, no free tier, a 7-day trial. An academic-year
  price is kept as an option; billing pauses June to August.
- **Onboarding creates the vault** under `%USERPROFILE%\Knowlu\` and asks for sign-ins, nothing
  else; syllabus upload joins it with the syllabus and GPA phase (Build order). It is judged by one
  thing: a new student reaches "it already knows what's next" in their first session.
- **Minimize the AI surface.** Most capabilities need no model. Classification over generation,
  schema-constrained output on every call, and a rule-promotion loop that retires model calls into
  rules as patterns repeat. The model's job keeps shrinking to the residue rules cannot decide.
- **Judgment is a service.** Extraction and classification run in our cloud, each kind pinned to one
  named model at one named zero-retention host, with no fallback routing, and logged by id. Open-weights models on hosted zero-retention APIs are preferred where they pass the evaluation. Nothing
  model-shaped ships in the app. **Fetch on device, think in the cloud:** the one thing that stays
  on the device is the fetch that needs the student's own portal sign-in.
- **Teach once, run deterministically forever.** A browser and a model bootstrap a source once; the
  output is a small readable extraction script every later sync runs. A broken script is a visible
  failure, and its regeneration is a proposal.
- **Sync replays the journal** to the account's storage. Conflicts surface as proposals, never silent
  merges.
- **Knowlu reads.** Outside the vault it writes only its own dedicated calendar, behind a scope the student grants for that purpose, and, in the student's own mailbox, what brings their mail to Knowlu: forwarding turned on at setup with the student present, older or missed mail forwarded to Knowlu's own address (whose Sent copies it removes), and forwarding turned back on, and then announced, with the saved sign-in the student allowed, never for a mailbox the student turned off in Knowlu's Settings. It never sends anything to anyone else. *(amended 2026-09-30, email spec A8, signed by Quinn)*
- **Telemetry** is interaction events and AI corrections under the terms, keyed to a pseudonymous
  account id, read only as aggregates with a minimum cohort. Raw content only behind a separate,
  revocable opt-in, never content derived from the Gmail API or from forwarded mail, and never grades.

## Standing rules

- Who started a change decides how it lands (commitment 5).
- At most two nudges a day, each for a real risk, with quiet hours. One daily email at most. Ambient
  first.
- Failures are visible; silence is never ambiguous.
- Every automation is editable text: nothing lives only in a UI or only in a chat transcript.
- **Credentials live in the OS keychain**, never in the repo, a log, or anything a model prompt
  touches. **Portal credentials never leave the student's machine**; the cloud may hold the account's
  own secrets (session, LMS feed URL) encrypted, never a portal password.
- **Data minimization is a rule:** collect the least that answers the question. Where a source cannot be narrowed (forwarded mail arrives whole), the cheapest deterministic screen discards what cannot answer it before any model reads it, and Knowlu keeps what it found, never the message. *(amended 2026-09-30, email spec A9, signed by Quinn)*
- **No single-user assumptions.** Anything that would need hand-editing for a second student is a bug.

## Knowlu succeeds when

1. **Daily habit:** students open it every morning and work from it, with no second list in their
   heads.
2. **Nothing missed** that Knowlu had the information to catch.
3. **More free time:** students get ahead and have real time for friends and family, and say so.
4. The order is agreed with, or correctable in under 30 seconds when it isn't.
5. A new student reaches "it already knows what's next" in their first session.
6. A second student needs nothing from anyone: no hand-edited config, no shared credentials.

Failure modes designed against: trust decay, notification fatigue, capture friction, silent changes.

## Build order

Work moves in four stages, each named for what the student gets (the cloud design's Amendment
2026-09-29, ruling 10): **MVP → Pilot → Launch → Beyond**.

**The MVP is everything the founder's own system (quinn-ops) did, plus grades from Blackboard,**
proven on the founder's own scratch profile before anyone else uses it:

- today's page: the ordered list with start-by dates and slack, must-do work against the day's
  capacity, recommended work in the day's gaps, the schedule, coming-up events, and the
  ahead/behind verdict;
- the decisions deck (approve, reject, snooze) and every task field editable, with one progress
  control per task;
- **typed commitments** (the commitment model's phases 1–2): classes, labs, work and meetings,
  each hard, soft or optional, confirmed by the student;
- sources: the LMS feed, zyBooks, VHL, Google Calendar, email (school Microsoft 365 and personal Gmail through email forwarding to Knowlu, and Gmail directly) and campus events, with judgment in the cloud; *(amended 2026-09-30, email spec A11, signed by Quinn)*
- runs and failures visible in the app;
- **grades per course from Blackboard**, as the rings and their breakdowns.

**The pilot** is a few undergraduates other than the founder, on the released app, one computer
each, needing nothing from anyone. It adds the registrar where the school offers it (the
commitment model's phase 3). No one else signs up before the privacy page names the kept
Blackboard sign-in, the saved mail sign-in and the grades the account holds, a lawyer has read it, the re-consent a second
account needs is in the app, and a new vault records its student with the neutral token `student`
rather than the founder's name (the full gate: the cloud design's ruling 10). Until Knowlu has 20 paying users, the kept Blackboard sign-in and the saved mail sign-in are offered at every school without waiting for that school's policy to be read; at the 20th paying user the founder rules on the reads again (the cloud design's Amendment 2026-09-30, A13). *(amended 2026-09-30, email spec A13, signed by Quinn)*

**Launch** is any US undergraduate paying $9.99, on every desktop they use: the relay fetch and
two-desktop sync ship before it.

**Beyond, in phases**, each through spec, plan and review: the main page (domain strip, free time,
all clear, today's schedule); syllabus upload with grade weights, GPA and exam prep; quick
capture; nudges and lock-in; the assistant; the Outlook calendar and more homework platforms; the dedicated
calendar in Google and Outlook.

## Open decisions

| Decision | Status |
|---|---|
| How grades are read from each LMS and platform, and the legal and terms-of-service read on it | Open; Blackboard first |
| How a syllabus is read (a model extracting to a confirmed schema is the likely shape) | Open |
| The assistant's flows, and whether light study tips pass the "never does the work" line in practice | Open; spec before code |
| Lock-in mode and the exact nudge rules | Open |
| Writing the dedicated calendar (`calendar.events` on Google, the Outlook equivalent) | Later; needs a new consent and a privacy-page update |
| Co-founder | Unowned, and the only open decision with no next action |
| Legal read on scraping and minors | Landscape written (`docs/notes/2026-09-09-knowlu-cloud-legal-landscape.md`); a lawyer before the first non-founder paid sign-up |
| The repository's visibility | Public since 2026-09-29 (Quinn: develop in the open now, go private later; no scrub for now; the cloud design's Amendment 2026-09-29, ruling 13). The fixtures and history carry real coursework titles and a few personal paths. Returning it to private is a Launch decision |
