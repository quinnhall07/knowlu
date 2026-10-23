# Onboarding redesign: the spec (ruled by Quinn, 2026-10-23)

**Status: RULED, 2026-10-23.** Quinn ruled every open question on 2026-10-23 (§10). The filename keeps
its `-draft` suffix so links stay valid; the text is the spec. Written the same day by the planner
from Quinn's first prod install (`docs/notes/2026-10-22-first-install-feedback.md`), as a draft with
options, then revised twice. Where a section still shows options, the ruled one is marked; the others
are kept as the record. Each lane still needs its own plan, review and merge (§9); the signed texts
§11 lists are amended by those lanes, not by this file.

**Revision 2 (2026-10-23): the rulings.** Q1, Q2, Q4-Q13 as §10 records. Every ruling took the
recommendation **except Q8**, where Quinn chose **one Google client** (one consent screen), overriding
the recommendation to keep sign-in and data apart. Also recorded: task judging moved to
`qwen/qwen3.5-35b-a3b` at DeepInfra on 2026-10-23 (Quinn), with the effort anchors at prompt version
`task-3`; staging's eval reads `effort_mae` 0.273 (§6, §8).

**Revision 1 (2026-10-23), for Quinn's two rulings of the same day.**
- **The onboarding gets longer, not shorter.** Quinn, verbatim: "If anything, I think the onboarding
  should be longer. By the time the student sees the main page of Knowlu, everything should be set up
  and working. In fact, it's been shown that longer onboarding screens convert better. I want the
  student to understand the benefits of what they're doing as they're being signed into all these
  accounts. The account logins should be during onboarding. I'm also open to switching things up."
  The first draft's "the wizard ends at sign-in and payment; the rest runs in an in-app Setup view" is
  dropped. §1 is rewritten around **one long onboarding**: every login happens in it, each step opens
  with why it helps and ends with proof it worked, and the main page appears only after everything has
  run once, behind a "here's your week" reveal.
- **Q3 is ruled:** turned-in work is marked done automatically, with Undo; on a new vault it is
  archived at first sight (§6).
Sections 2, 3 and 5 stand (their wording now says "onboarding"). §1 is rewritten; §4, §6, §8, §9,
§10, §11 and §12 are revised. Code was still read from the `effort-anchors` worktree, not
`origin/main` (see "What was read").

**Authority, most recent first.** The cloud design (`docs/specs/2026-09-09-knowlu-cloud-design.md`)
and its amendments; `VISION.md`; `CLAUDE.md`'s two overriding rules; the signed specs each section
names. Where this spec disagrees with an older signed text, this spec's ruling is the newer one; the
older text is amended (§11) in the lane that changes the behaviour it describes, before that lane
merges.

**What was read, and from where.** The planner had no shell, so it could not run `git fetch` or
`git show origin/main:<path>`. The main checkout is far behind (its `app/src` has no `mail*.rs` or
`registrar.rs`). Code and plans were read from the newest worktree, `.claude/worktrees/effort-anchors`
(branch `fix/effort-anchors`, which carries `main` plus that fix), and `fix/current-term-courses` from
`.claude/worktrees/current-term`. **Every `file:line` below is that worktree's.** Before a plan is
written from this spec, the planner re-reads each cited line on `origin/main`.

**No student data.** Nothing here names a class, schedule, grade or address. "The founder's school"
stands for the school of the founder's scratch profile; no constant names it.

---

## Summary

Quinn's first real onboarding can ask for **up to four separate school sign-ins** (five once L5's
Simple Syllabus lands), offered typed class times, and left 28 tasks live, many of them already turned
in. The fix is not one bug. It is the order of the setup, how
many browser profiles hold a school session, and what the first run trusts.

| # | Topic | Today | Ruled | Size | Ruling |
|---|---|---|---|---|---|
| 1 | Order of asks | A 10-panel wizard; school email, Blackboard grades and class times come after Finish, each in its own step | **One long onboarding** where every login happens: why → sign in → **one** school sign-in → payment → school email → Gmail and Calendar in one Google consent → homework sites → syllabi → your week → "here's your week" reveal. Each step shows proof; work runs behind the student; the main page waits until everything has run once | L | Direction, Q2, Q7, Q12, Q13 |
| 2 | One school sign-in | Up to five school sign-ins: course link, school mail, class times, grades, Simple Syllabus. Each uses its own browser profile | One kept **school** profile shared by every school read, after spike S-A. Personal mailboxes keep their own profiles | L | Q1, Q9 |
| 3 | Mail strategy | Gmail by forwarding is offered beside Gmail by Google's permission; forwarding wins | Gmail by permission first for the first 100 permission users (friends included); forwarding if declined, failed or past 100. One Google client. M365 older mail: lane L1 after O1, O3 and O4 | S (+ L1, XL) | Q4, Q5, Q8, Q10 |
| 4 | Class times | Typed if no calendar shows them; registrar is a separate button and sign-in | Registrar runs inside the school sign-in; syllabus fills the rest; the student only confirms | S on top of L5 | L5's rulings; Q7 |
| 5 | Late-work policy | L5 reads it onto the course note; `rank` does not use it (Q34, ruled 2026-10-19) | `rank` uses it: a past-due item leaves Today when its late window closes, into "Late window closed", archived only by the student's tap | M | Q6 (reopens Q34) |
| 6 | Turned-in work | zyBooks at 100% files a card; Blackboard's "submitted" is never used | Authoritative "submitted" marks the task done, with Undo; on a source's first run it goes straight to the archive; grades run before ingest | M | Q3, Q11 |
| 7 | Past-term courses | Listed in the course step | Fixed in `fix/current-term-courses` | (in flight) | No |
| 8 | First-run expectations | Old, turned-in and oversized work on day one | Only live, realistically sized work: items 5-7 plus `fix/effort-anchors` and the task judge's move to `qwen/qwen3.5-35b-a3b` (anchors `task-3`, staging `effort_mae` 0.273) | (sum of the above) | No |

**What can ship to friends first** (§9): the two fixes in flight, turned-in work (item 6), the
Gmail-first flip (item 3), and root causes for the mail summary and "See what came from your email".
None of those needs the shared profile. The shared school profile and the long onboarding follow once
spikes S-A, S-B and S-E pass. Wave A's first plan is
`docs/plans/2026-10-23-o1-turned-in-work-plan.md`.

---

## 0. Checked against VISION and the two overriding rules

- **"Onboarding ... asks for sign-ins, nothing else"** (`VISION.md`, The product). Today's wizard also
  asks a profile name, a timezone and two slot times, and makes the student type class times. The
  proposal asks sign-ins, a payment and one confirm screen; it explains, but it asks nothing else.
- **"A new student reaches 'it already knows what's next' in their first session"** (VISION success 5).
  Quinn's ruling makes this literal: the main page is first shown only when it already knows.
- **"Adding a source is one action (a sign-in, a URL or a file)."** One school sign-in that feeds
  five sources meets this better than five sign-ins.
- **Commitment 5, who started a change.** A gradebook that says "submitted" is the LMS, an
  authoritative source: apply and list (item 6). A late rule read from a syllabus was proposed and the
  student confirmed it; using it to *hide* past work is display, and archiving stays a student tap
  (item 5).
- **Commitment 1, `rank` never calls a model.** Item 5 has `rank` read a field the student confirmed.
  No model is involved after the syllabus pass.
- **Portal credentials never leave the device.** The shared school profile is on the device, under the
  app's data folder, like the two kept profiles today. No password is ever kept.
- **Rule 1, no single-user assumptions.** Whether a school's LMS, mail and registrar share one sign-in
  is a curated per-school fact (a field on the campus row), never code. Nothing names a school.
- **Rule 2, frozen references.** Item 5 changes ranking only for a course note that carries
  `late_policy`. No fixture vault has one, so `golden-today-*.md` and `surface-today-*.json` stay
  byte-identical; a test pins that (§5). Item 6 adds an archive reason; no frozen reference changes.

---

## 1. Order of asks

### 1.1 Today

The wizard (`app/static/index.html:117-201`) runs ten panels in this order
(`app/static/console.js:3146`): welcome, account, subscribe, **mail** (an announcement only: "Next:
connect your school email ... It starts when Knowlu first opens", `index.html:138-142`), vault name,
**calendars** (pick the school, sign in to the LMS to capture the feed link and the course list, then
a personal iCal paste or Google Calendar, `index.html:148-174`), coursework logins (zyBooks, VHL),
**Gmail** by Google's permission (`index.html:186-191`), slots and timezone, finish.

After Finish, the console does the rest, each as its own step:
- school mail setup opens with the console (`mailFirstOpen`, `console.js:2488-2495`), in its own
  sign-in window;
- the first-day screen asks "When do your classes meet?" (`index.html:96-105`); its registrar button
  "Get my class times" (`console.js:1024-1055`) opens another sign-in window;
- grades sit behind "Connect Blackboard" on the grades strip (`console.js:1203`), a third window after
  onboarding.

Why it is this shape: the wizard runs in a shell with **no `ConsoleState` and no profile folder until
Finish** (`app/src/onboarding.rs:1-5`; email spec D6). So anything that keeps a browser profile, or
needs the vault, had to wait for the console.

### 1.2 The shape: one long onboarding (direction ruled by Quinn, 2026-10-23)

One continuous flow, full screen, from install to the reveal. Every login happens in it. Each step has
the same three parts:
- **Why** (one line, before anything is asked): what this connection does for the student.
- **The ask**: one sign-in, consent or confirm. Never a form beyond that.
- **Proof** (when its work is done): counts the student can check, e.g. "Found 6 classes, 14
  assignments, 3 already turned in". A step whose work is still running shows "reading…" and the
  student may go on; its proof fills in on a progress rail that stays visible to the end.

The main page is shown for the first time only after the reveal (§1.8), when every connected source
has run once.

### 1.3 The steps

Times are **estimates to be measured** (spike S-E, §12). "Background" is what keeps running while the
student moves on. The counts in the proof column are illustrative, not anyone's data.

| # | Step | Why (the line the student reads) | The ask | Proof | Student's time | Background |
|---|---|---|---|---|---|---|
| 1 | Welcome | "Knowlu answers one question every morning: what's next? Next, you'll connect the places that already know your work, about 10 minutes." | none | none | 30 s | none |
| 2 | Your account | "Your plan lives in your account, so it's safe if this computer isn't." | Continue with Google (name and email only) or an emailed code; 18+ and terms | "Signed in as …" | 30-60 s | the vault folder is created with defaults (§1.9) |
| 3 | Your school | "One school sign-in lets Knowlu read your classes, class times, assignments, grades and school email. You won't sign in to these again." | pick the school; **one** sign-in on the kept school profile (§2) | "6 classes this term; class times for 5; 14 assignments, 3 already turned in; grades for 6 courses; syllabi found for 4" | 1-3 min (SSO, MFA) | course list, feed link, registrar, grades capture, syllabus skim, then the school mailbox's forwarding steps |
| 4 | Payment (Q2, ruled) | "Everything you just connected keeps running every day for $9.99 a month, after 7 days free." | Stripe checkout in the browser; testers see "nothing to pay" | "Your trial is on" | 1-2 min | step 3's reads continue |
| 5 | School email | "Professors change dates by email. Knowlu reads what arrives and proposes each change for you to approve." | nothing new where step 3's sign-in covers it; otherwise one sign-in | "Forwarding on; test message arrived in 38 s; first mail read: 2 things to review" | 0-1 min | canary (up to 3 min), first mail pass |
| 6 | Your Google account: Gmail and Calendar (§3, Q4, Q5, Q8) | "Clubs, jobs and internships write to your Gmail, and your calendar says when you're already busy. One Google permission covers both, read-only." | **one** Google consent asking Gmail and Calendar together, on the same Google client as sign-in (no second consent screen); Gmail forwarding only if Gmail is declined or the read fails, or the student is past the first 100 permission users; Skip | "Read 40 recent messages; 3 things to do. 9 events this week" | 20-40 s (permission) or 2-4 min (forwarding) | first Gmail read, first calendar read |
| 7 | Another calendar | "If your calendar isn't Google's, add it so your free time is honest." | shown only when step 6 granted no calendar: paste an iCal link, or Skip | "9 events this week" | 0-60 s | calendar read |
| 8 | Homework sites | "Homework sites hold due dates your school's feed doesn't show." | zyBooks, VHL logins (only those step 3 found mentioned, when known); per site Skip | "zyBooks: 2 books, 8 assignments, 5 done" | 1 min | discovery, first coursework read |
| 9 | Syllabi | "Your syllabus has every exam date, how each course is graded, and the late-work rules." | only for courses step 3 found none: upload a file or connect Simple Syllabus; per course Skip | "Read 5 of 6: 4 exams, weights for 5, late rules for 3" | 0-3 min | syllabus judge pass |
| 10 | Your week | "Check your classes. Everything here can be changed later." | confirm class times, exams, weights and late rules; Edit any row; typing is the last resort | the confirmed week | 1-3 min | the first full slot starts |
| 11 | The reveal (§1.8) | "Here's your week." | Open Knowlu | the week, what's next, what was archived | 30 s | none: everything has run once |

About 8-15 minutes of the student's time, most of it in steps 3, 4 and 10.

**Why this order.** The school step comes first among sources because it feeds the most and starts
the most background work, so by step 10 nearly everything has finished. Payment comes right after
the school step's proof (Q2, ruled). School email follows the school step because it reuses that
sign-in. Gmail and Calendar share one Google consent (Q8). Homework sites come after the school step so
the course list and syllabi can say which sites matter. Syllabi come late because only the courses
still missing one need the student. Your week comes last, since by then registrar and syllabus rows
are in.

**One Google client (Q8, ruled by Quinn, overriding the recommendation).** Sign-in and the data
scopes use the same Google OAuth client. Prod already does: its `google-connect` functions carry the
sign-in client, and Quinn enabled the Gmail and Calendar APIs on that client's project (2026-10-23).
Sign-in asks `openid`, email and name; step 6 asks `gmail.readonly` and `calendar.readonly` together,
widening the same grant (`include_granted_scopes=true` on every ask,
`cloud/supabase/functions/google-connect/handler.ts:6-19`, `:160-168`). So the student meets one app
name and one data consent screen. `google-connect`'s `scopeFor` asks one API scope per request today
(`handler.ts:53-65`); a combined ask is a small change in that function (lane O3). *Reason:* one consent
screen. *Mitigation:* if Google ever restricts the app, the emailed-code sign-in still works, so no
student is locked out. *Revisit:* at Google verification.

### 1.4 Where payment sits (Q2, ruled: option B)

The engine gates itself: `coursework`, `ingest`, `grades`, `judge` and `sync` do not run without an
entitlement (`engine/src/entitle.rs`; `CLAUDE.md`). The school step's **device-side** reads (course
list, feed link capture, registrar, grades capture, syllabus skim) need no entitlement; turning them
into notes and every cloud judgment does.

| Option | What the student sees before paying | For | Against |
|---|---|---|---|
| A. First, as today | nothing but the welcome | No gate change; no cloud cost for students who leave | A card before any value; the longest part of the onboarding happens after the money question, which is the opposite of Quinn's "understand the benefits" |
| **B. After the school step's proof (ruled)** | real counts from their own school: classes, class times, assignments, turned-in work, grades | Value is shown, from the student's own accounts, before the ask; no gate change, since those counts come from device-side reads | The counts are computed on the device from raw captures before any note is written (a small new read-only summary); a student who leaves has spent no cloud money but has signed in |
| C. After the reveal | the whole week | The strongest value-before-price | Every cloud step (ingest, judgment of every task, syllabus, mail) runs unpaid. It needs an "onboarding" entitlement state in the cloud and the engine gate admitting it, which is contract-list work in `entitle.rs`; real cost for every student who walks away; closer to the "no free tier" line (cloud design D2) |

**Ruled: B** (Quinn, 2026-10-23). It shows the student's own data before the price and changes no
gate. Testers (L8) see "nothing to pay" in the same place.

### 1.5 What runs behind the student, and when the reveal opens (Q7, ruled)

Background work starts the moment its ask is answered and never blocks the next step. A progress rail
on the side lists each source with "reading…", its proof, or a named failure.

**What must have run once before the reveal opens** (Quinn's list):

| Item | Done when | Typical wait (to measure, S-E) |
|---|---|---|
| First sync | the account's first push and pull finished | seconds |
| Coursework | the first `coursework` pass, turned-in items archived (§6) | under a minute |
| LMS feed | the first `ingest` | seconds |
| Grades | the first `grades` step, with the turned-in filter | under a minute |
| Syllabus read | every syllabus found or added has been judged once | about a call per syllabus |
| Class times | step 10 confirmed (by the student) | the student's time |
| Mail | forwarding verified by the canary and the first mail pass delivered (not the whole backfill) | 1-5 min |
| Gmail | the first read (permission) or canary (forwarding) | under a minute |
| Effort and ranking | `judge` and `rank` over the new tasks | to measure |

Two limits keep "everything ran once" honest:
- **Backfills are not waited for.** Microsoft 365's older mail (held until L1) and Gmail's 120-day
  forwarding backfill can take hours or days at the providers' pace. "Mail connected and first mail
  read" means the live path works and one pass ran; the reveal says how much older mail is still
  coming.
- **A cap on waiting** (Q7, ruled). The reveal waits up to **3 minutes** after step 10, then opens
  and names what is still running: "Still reading 1 syllabus; it will appear on your page." Backfills
  are never waited for. A source that failed is named with its fix, never hidden.

### 1.6 Resume after quit

The onboarding's state lives in the profile's app data (`onboarding.json` beside `settings.json`,
never in the vault): the current step, each step's answer, and which background jobs finished. On the
next launch the app opens onboarding at the same step, not the main page, and re-runs any background
job that did not finish (each is already idempotent: captures re-read, slot steps skip what they have
seen). With the kept school profile (§2) the student does not sign in to the school again unless the
school's session expired; with the throwaway fallback they do. A student who left at payment returns
to payment, and the entitlement is re-asked first (the existing `wizGo` re-ask, `console.js:3402-3417`).
The scheduler and "start with Windows" turn on only at the reveal, so an unfinished onboarding never
runs slots in the background.

### 1.7 Skip rules (Q12, ruled)

Ruled: optional sources may be skipped, each with its cost named. Account, payment, Your week and the
school sign-in may not; the school sign-in's only way around is the feed-link fallback.

| Step | May skip? | What skipping costs (said on the step, and again at the reveal) |
|---|---|---|
| Account, payment | No (testers see no payment) | none |
| School sign-in | No, but with a fallback: an uncurated school, or a sign-in that fails, pastes the feed link instead | with the paste: no grades, no registrar class times, no syllabus skim, no turned-in filter for LMS work, and school email needs its own sign-in |
| School email | Yes (e.g. the school forbids forwarding, email spec D6, risk 12) | date changes and new work announced by email are missed |
| Google (Gmail and Calendar) | Yes | club, job and internship tasks from Gmail are missed; free time and busy time ignore the student's own events |
| Another calendar | Yes | as above, for a calendar that isn't Google's |
| Homework sites | Yes, per site | that site's due dates are missing unless the LMS feed carries them |
| Syllabi | Yes, per course | no exam dates beyond the feed, no weights, no late rule for that course |
| Your week | No as a screen; any row may be left blank | blank classes come back as asks over the next days (today's rule) |

Every skip is one named line at the reveal ("2 things not connected") and in Settings, with its
Connect button. Nothing skipped is asked again in a nudge.

### 1.8 The reveal: "Here's your week"

The last screen before the main page, built from the same read model the page uses:
- **The week**: classes, due dates, exams and the student's own events, from the Schedule view.
- **What's next**: the first item of Today's list, and why it is first.
- **Free time**: the honest number for today, where the commitments allow it.
- **What Knowlu tidied**: one line each, no lists: "47 past items and 9 already turned in were
  archived", "Effort estimated for 14 assignments".
- **Still coming**: anything still running or skipped (§1.5, §1.7).
- One button: **Open Knowlu**. The main page then shows exactly what the reveal promised.

### 1.9 How it is built

Today's wizard runs in a separate vault-less shell (`app/src/onboarding.rs:1-5`), which is why the
school mail, registrar and grades waited for the console. The long onboarding needs the vault and the
profile folder from step 2 on, because the kept school profile, the captures and the first slot all
live there.

| Option | What | For | Against |
|---|---|---|---|
| **A. Vault at step 2, onboarding as a full-screen mode of the console (ruled)** | step 2 creates the vault with defaults (name "Knowlu" with a suffix for a second profile, Windows' timezone, slots 12:00 and 18:00, all editable later); the app relaunches into the console with `ConsoleState`, showing onboarding instead of the main page until the reveal | every window, profile and engine step already works from the console; resume is a field in `onboarding.json` | a student who abandons after step 2 leaves a vault folder; Settings and the picker list it as "setup not finished" |
| B. Keep the vault-less shell to the end | stage everything, create the vault at the reveal | no early folder | a kept SSO profile and capture files with no owning profile, and a move of a live WebView2 profile at the end; the first slot cannot run before the reveal, against Quinn's ruling |

**Ruled (Q13): A** (Quinn, 2026-10-23): the vault is created at the account step, and onboarding is a
full-screen mode of the app. The relaunch at step 2 is the one the wizard already does at Finish
(`onboarding::relaunch_with`).

**Google scopes** are as §1.3 rules them: one client; sign-in basic; Gmail and Calendar in one ask at
step 6.

### 1.10 Cost

**L to XL.** console-ui: eleven steps with why, proof and the progress rail, and the reveal.
implementer: the onboarding mode and `onboarding.json` resume; moving `open_lms_window`,
`capture_courses` and `discover_coursework` into the console command list; default plan values in
`create_vault`; the device-side counts for step 3's proof; the reveal's gate. The `main.rs` handler
lists are lane L6's until it merges. `scripts/wizard-check.py` and the wizard's static tests are
rewritten. Copy for each "why" line: Quinn reads them.

### 1.11 Risks

- **Length.** Eight to fifteen minutes is long. Quinn's ruling accepts that; the proof on every step
  and the progress rail are what keep it from feeling like a form. The pilot measures drop-off by step
  (interaction telemetry, class (a)).
- **Waiting at the end.** If background work is slow, the student waits before the reveal. The 3-minute
  cap and the named "still coming" line bound it (Q7).
- **An adopted or restored install** (C3′ restore) must not repeat steps the account already has.
  Onboarding reads what the account and vault hold and skips finished steps, showing their proof.
- **One Google client for sign-in and data (Q8, ruled).** If Google restricts or suspends the app
  over the restricted Gmail scope, sign-in through Google stops with it. Mitigation: the emailed-code
  sign-in is always offered and reaches the same account; Gmail falls back to forwarding. Revisit the
  split at Google verification.
- **C1's rulings.** Many (R-C1-40 to R-C1-55, R-OB-1 to R-OB-4) are about the wizard's panels; each one
  that moves is re-read, not dropped. The plan lists them.

---

## 2. One school sign-in for Blackboard, Outlook and the rest

### 2.1 Today: five windows, four kinds of profile

| Read | Window | Profile | Kept? | Where |
|---|---|---|---|---|
| Feed link and course list (wizard) | `lms-signin` | throwaway temp folder, **incognito** | No: wiped when the window closes | `app/src/lms_link.rs:34-42`, `:164-190` |
| Class times (registrar) | `lms-signin`, reused | same throwaway pattern | No | `app/src/registrar.rs:4-7` (plan ruling R4-a) |
| Grades (Blackboard) | `lms-grades` | `<profile data>\lms-session` | Yes (ruling 12, suspended gate A13) | `app/src/grades.rs:263-275`, `:573-588`, `:617-632` |
| School and personal mail | `mail` | `<profile data>\mail-session-<address_id>`, one per mailbox | Yes (email spec D15) | `app/src/mail_session.rs:4-12`, `:53-56`; `app/src/mail.rs:537-573` |
| Simple Syllabus (L5 Offer A, not built) | its own window | nothing kept | No | syllabus spec §3.1, §15.2 |

A student at a school whose Blackboard, Outlook and registrar all use one campus sign-in signs in to
that same campus account up to four times today, and five with Simple Syllabus, each with its own
MFA prompt.

### 2.2 Why they were separated

Each separation had a reason. None was "the school sign-ins must stay apart":

1. **"Keep nothing" (cloud design §11a, 2026-09-09).** The first rule was that a sign-in window keeps
   no session at all. The feed-link capture is incognito so the campus login "lives in memory rather
   than in a profile on disk" (`lms_link.rs:1-6`). The registrar copied that window as it was (R4-a).
2. **Ruling 12 (2026-09-29) made one exception, for grades only, and fenced it.** One kept profile per
   Knowlu profile; "the calendar-link and course-list captures keep their discard-everything window
   unchanged; the registrar never shares this profile" (cloud design, Amendment 2026-09-29, ruling 12).
   The fence keeps the exception as small as possible while the policy reads were pending. A13 has
   since suspended those reads until 20 paying users.
3. **One profile per mailbox (email spec D15, R24).** The stated reason: "One profile per mailbox means
   'Disconnect Gmail' leaves no Google session behind while the school mailbox stays connected." That
   is a reason to keep **different identities** apart (school versus personal Google). It is not a
   reason to keep the school mailbox apart from the school's own LMS.
4. **Sign-out and delete semantics.** Each kept profile has one owner and one delete: "Forget
   Blackboard sign-in" deletes `lms-session` (`grades.rs:285-304`); a mailbox's Disconnect deletes its
   `mail-session-<id>`; the saved-sign-in switch off deletes every mail profile; Knowlu sign-out and
   account deletion delete everything (`app/src/account.rs:699`, `:736`, `:1662-1683`). One owner per
   folder made every delete easy to state and to test.
5. **Browser settings.** The mail window runs with its own browser arguments (`MAIL_BROWSER_ARGS`,
   `mail.rs:534-535`) and an initialization script (`mail.rs:559`). WebView2 requires every window
   that shares one data folder to use the same environment options, so a shared profile must carry
   one set of arguments for all its windows. *(Unverified in this app; spike S-B, §12.)*

### 2.3 Options

| Option | What | Sign-ins at a school with one campus login | For | Against |
|---|---|---|---|---|
| A. Status quo | As §2.1 | 4 (5 with Simple Syllabus) | Nothing to amend | Quinn's main complaint stands |
| B. One throwaway profile for the setup sitting | The wizard-era reads (feed link, course list, syllabus skim, registrar, Simple Syllabus) share one incognito window, wiped at the end; grades and mail keep their own kept profiles | 3 | No kept-session rule changes | Still three sign-ins; the kept ones are the ones a student meets again |
| **C. One kept school profile (ruled)** | `<profile data>\school-session`: every read that signs in as the student's **school identity** uses it (feed link, course list, syllabus skim, registrar, Simple Syllabus, grades, the school mailbox). Personal mailboxes keep `mail-session-<id>` (D15's reason holds) | **1** | One sign-in, one MFA prompt; later reads (grades refresh, registrar next term, mail repair) are silent until the school's session expires | Amends ruling 12 and D15; one "Forget" now touches several features; the privacy page says more |
| D. Copy the campus sign-in cookies between profiles | Keep separate profiles; after one sign-in, copy the identity provider's cookies into the others | 1 | Keeps each feature's own folder | Code that moves live login cookies between stores is a credential-handling surface; fragile per identity provider; rejected |

**Ruled (Q1): C**, one kept school sign-in, built after spike S-A (§12) shows, on the founder's scratch profile and structure only,
that the founder's school's LMS, Outlook and registrar accept one sign-in in one profile, silently.

### 2.4 What C means, precisely

- **The profile.** `<profile data>\school-session`, under `state::app_data_root()`, never the vault,
  never synced, no password kept, DPAPI-encrypted cookies (the same bounds as ruling 12 and D15). Its
  own path guard in the style of `grades::is_session_dir` and `mail_session::is_session_dir`.
- **Who uses it.** A curated campus row gains one field, `shared_sign_in: true`, recorded when a spike
  shows the school's LMS, mailbox and registrar share one sign-in. Where it is absent, each feature
  keeps today's profile and nothing changes for that school (no gain, no harm). An uncurated school's
  mailbox stays on its own profile.
- **Windows.** One visible sign-in at onboarding's school step (§1.3 step 3). Every later read opens a hidden window on the
  same profile and waits for the school to sign it in silently, as grades' refresh already does
  (`grades.rs:617-632`, `read_hidden`). One window at a time on the profile; the scheduler already runs
  the grades capture and the mail step one after the other.
- **Cookies stay per host.** Each reader still takes only its own host's cookies (`jar_for(w, host)`,
  `grades.rs:600`; the registrar's host-only read, `registrar.rs:6`). A test pins that no reader asks
  the profile for another host's cookies.
- **Scripts stay per window.** The mail window's initialization script (and C3's bearer closure)
  remains on the `mail` window only; the grades and registrar windows run none. No window gains a
  capability grant.
- **The incognito capture stays as the fallback.** A student who turns the kept school sign-in off
  gets option B's behaviour: one throwaway profile for the onboarding sitting, wiped at the end, and a
  sign-in each time grades or mail repair needs one.

### 2.5 Sign-out, Forget and delete under C (Q9, ruled)

| Action | Today | Under C |
|---|---|---|
| Forget Blackboard sign-in | deletes `lms-session` | **"Forget school sign-in"**: deletes `school-session`. Grades refresh, registrar refresh and school-mail repair each then read "needs sign-in" until the student signs in again |
| Disconnect the school mailbox | turns forwarding off, deletes `mail-session-<id>` | turns forwarding off (using the session first), then stops *using* the school profile. The profile stays while grades use it |
| Saved mail sign-in switch off | deletes every mail profile | deletes personal mailbox profiles; the school mailbox stops using `school-session`; the profile is deleted only when no feature uses it |
| Knowlu sign-out, account deletion | deletes every kept profile | unchanged: deletes `school-session` with the rest |

The rule under C, ruled by Quinn (Q9): one "Forget school sign-in". **One profile per identity; each feature is a use of it; the profile goes when its
last use goes, and always on sign-out and deletion.** Settings shows one row: "School sign-in: kept on
this computer, used for grades, class times and school email. Forget."

### 2.6 Cost

**L, plus a ruling.** A spike first (S-A, S-B; main session at the machine, the founder's scratch
profile, structure only). Then: a new `school_session` module (implementer); the grades, registrar,
`lms_link` and mail windows taught to pick their profile from the campus row (implementer); the
sign-out and deletion paths in `app/src/account.rs` (**contract list**: contract-engineer, then
contract-reviewer); the Settings row (console-ui); the privacy page sentence (L4's page, reviewer).

### 2.7 Risks

- **Not every school shares one sign-in.** A school whose Blackboard has its own login, or whose
  conditional access asks MFA per app, gains less. The curated field keeps those schools on today's
  path; the spike records what the founder's school does, and each new curated row records its own.
- **A school session can expire sooner than a mail session would.** Grades already live with this
  ("signed out" is a named state, `grades.rs:593-615`). The email session-lifetime spike measured
  kept sessions lasting at least 4.1 days; one shared profile does not shorten that.
- **One Forget now touches more.** Mitigated by the Settings row naming every use, and by Forget's
  confirm listing what will need a sign-in again.
- **Wider exposure on the device.** Malware running as the student could use the profile. It can
  already use both kept profiles today; one folder instead of two changes little. Stated on the page.
- **WebView2 environment options must match** across the windows sharing the folder (S-B). If they
  cannot (the mail window needs arguments the grades window must not have), C falls back to sharing
  only among the non-mail reads, and the school mailbox keeps one more sign-in.
- **The privacy page.** It names "the kept Blackboard sign-in" and "the saved mail sign-in" today
  (L4's bump). Under C it names one kept school sign-in and what uses it. That is a page change with
  a `PRIVACY_VERSION` move and the re-consent screen L4 built.

---

## 3. Mail strategy

### 3.1 Today

- **Two paths** (cloud design, Amendment 2026-09-30): forwarding to the student's private Knowlu
  address, set up by Knowlu in a sign-in window, for school Microsoft 365 and personal Gmail; and Gmail
  by Google's permission (`gmail.readonly`, read server-side by `gmail-read`).
- **When both exist for one Gmail, forwarding wins** (email spec D22): `gmail-read` reads nothing new
  for a Gmail address with an active forwarding.
- **The Gmail permission is a restricted scope.** Out of Testing, or past 100 users, it needs Google's
  verification and a yearly CASA assessment (cloud design §5.3; quotes of about $540-1,800 a year,
  secondhand, `docs/notes/2026-09-30-email-access-options.md:52`). In Testing mode, refresh tokens die
  every 7 days, and every friend must be added by hand as a test user.
- **The quiet paid release already accepts the 100-user cap** (Amendment 2026-10-16b): "students past
  it use email forwarding."
- **`gmail-read` reads only the last 7 days** (`gmail-read/handler.ts:37`, `newer_than:7d`).
  Forwarding's backfill reads 120 days (email spec D7 as amended).
- **Gmail forwarding's confirmation is already automated.** Google mails a confirmation to the Knowlu
  address; intake recognises it and completes it server-side (email spec §4.3 step 4, E3). Nothing new
  is needed there.
- **Microsoft 365's "Bring in recent mail" is held.** `mail_steps::backfill_held_for_c3` returns true
  for M365 (`app/src/mail_steps.rs:86-95`), so the step is guided and no routine runs. This is why
  Quinn's summary said "Mail before ... was not brought in" and prod's `backfill_since` is null.

### 3.2 Ruled (Q4, Q5, Q8; Quinn, 2026-10-23)

| Who | School mail | Personal Gmail |
|---|---|---|
| **The first 100 Gmail-permission users (friends and paid alike)** | Forwarding, set up inside the one school sign-in (§2) | **Gmail by permission first** (Q4, Q5), in the one Google consent of step 6 (§1.3, Q8). If granted, no forwarding is offered (D22's precedence is unchanged for a student who has both; onboarding simply stops offering the second). **Forwarding only if the permission is declined or the read fails**, or Skip |
| **Past the 100th permission user** | Forwarding | **Forwarding**, until verification and CASA are decided. That decision is taken at Launch or at 100 Gmail-permission users, whichever comes first (Q5) |

This is the 100-user cap Amendment 2026-10-16b already accepts ("students past it use email
forwarding"); the ruling fills the cap with permission users first rather than keeping permission for
testers only.

Why permission first: one Google consent in a browser the student is already signed in to is far
fewer steps than signing in to Gmail inside Knowlu's window (password, phone prompt, Google's "verify
it's you" popup, email spec §4.3). It is Quinn's "fewer logins". Why forwarding past 100: it has no
user cap, no CASA, it reads 120 days back instead of 7, and it is already built for both providers.

**Counting the 100.** Google's cap counts users who granted the app a restricted scope. Knowlu needs
its own count of accounts holding an active Gmail grant (a server-side count over the Google
connections table, `cloud-engineer`), so step 6 can stop offering permission before Google refuses
it. The exact cap semantics are spike S-D's.

### 3.3 Options for personal Gmail (ruled: A as amended)

| Option | For | Against |
|---|---|---|
| **A. Permission first for the first 100 permission users, forwarding past that or on decline or failure (ruled)** | Quinn's ask now; no restricted-scope cost until Launch or 100 | Two paths stay in the code |
| B. Permission for everyone; pay for verification and CASA | One consent for every Gmail user | Yearly assessment cost and a weeks-long review before the quiet release; 7-day history only |
| C. Forwarding for everyone, permission removed | One path | Goes against Quinn's ask; Gmail sign-in inside Knowlu's window for every friend |

**One Google client (Q8, ruled, overriding the recommendation).** Sign-in and data share the one
client prod already uses; no second consent screen exists (§1.3). The mitigation and the revisit are
in §1.3 and §1.11.

### 3.4 Two facts to settle before friends rely on Gmail permission

1. **Prod's Gmail read failed on 2026-10-22.** `gmail-read` answered 500 and the calendar read 502 on
   prod; the function's `GOOGLE_CLIENT_ID` is the sign-in client, which did not yet have the Gmail and
   Calendar APIs enabled (feedback note item 12). Quinn enabled both on that client's project on
   2026-10-23. Lane O0 re-runs one Gmail and one Calendar read on prod and records the result.
2. **Which publishing status the one client's project is in.** In Testing, each permission user
   reconnects every 7 days and must be added by hand as a test user; and since sign-in shares the
   client, Testing would limit Google sign-in itself to listed test users. "In production" and
   unverified shows Google's warning and keeps the 100-user cap but, per Google's pages, does not
   expire tokens weekly. *(To verify against Google's current pages, S-D.)*

### 3.5 Microsoft 365 "bring in older mail": what it would take

It is lane **L1**, already designed and signed: `docs/specs/2026-10-10-m365-backfill-redesign.md`
(read each original in the signed-in Outlook page and upload it from the device; the mailbox is never
sent from) with its plan `docs/plans/2026-10-18-c3-m365-backfill-plan.md` (tasks R0 to R13: a
contract-list `init.js` closure, the upload route and migration, the device pass, retiring the DOM
routine, copy, docs, then the C1 staging check and a live re-proof). Size **XL**. Until it ships the
step stays guided. Under §2's option C it runs on the school profile with no extra sign-in.
**Ruled (Q10): L1 comes after O3 and O4 (the shared sign-in) and O1**, so it is built once, on the
school profile.

### 3.6 Cost and risks

**Cost: S to M** for the Gmail-first flip and the one-consent ask (console-ui: step 6 offers
permission first and hides forwarding when the grant is active; cloud-engineer: `google-connect`
asks Gmail and Calendar in one request, and the count of permission users that turns the offer off at
100; tests for both). **L1 is XL** and is its own lane.

**Risks.** Weekly reconnects, and a sign-in limited to test users, if the client's project stays in
Testing (S-D). Gmail by permission brings in only 7 days of history, so a student who signs up
mid-term gets less Gmail backfill than forwarding would give; the step says so in one line. A
restriction on the shared client would stop Google sign-in too: the emailed code is the fallback (Q8).
The 100th-user boundary must be counted by Knowlu, not discovered from Google's refusal.

---

## 4. Class times from the registrar and the syllabi, not typing

Quinn: "I don't want to enter my class times ... It should be based off of syllabi."

### 4.1 Today

- Class times come from repeating calendar events only. A course with none is "uncovered"
  (`commitments::uncovered_courses`); the first-day screen says "no class times found; Knowlu will ask
  this week" (`console.js:1048-1049`), and from vault day 3 a `commitment-ask` card asks the student to
  type days and times (syllabus spec §1).
- The ask card does allow more than one meeting ("add another time", `console.js:561-577`). The
  first-day screen offers no entry for an uncovered course at all. Quinn's "one meeting time per class"
  is probably the card's single first row; the root cause is not confirmed and the point stands either
  way: typing should not be the path.
- **The registrar (PR #19)** is on `main`: "Get my class times" on the first-day screen and the
  Schedule view (`index.html:99`, `console.js:851-904`, `:1055`) opens a throwaway sign-in window, reads
  the schedule, and confirms matched courses as hard commitments as the student (ruling R24). It is a
  separate button and a separate sign-in.
- **L5, the syllabus read** (spec `docs/specs/2026-10-05-syllabus-class-times-design.md`, signed
  2026-10-19; plan `docs/plans/2026-10-19-l5-syllabus-class-times-plan.md`, r10c, not started). Order
  (ruling 2): the syllabus in the LMS first, then Simple Syllabus or the student's own file, typing
  last. Precedence: registrar above syllabus above calendar (L5's Q6, not this spec's). Syllabus meetings are proposed and
  confirmed by a tap; syllabus cards are held to day 2 (Q24). Its step 1 runs "only inside the
  wizard's LMS sign-in" (§3.1). It is a wave-1 MVP lane but not a condition of the friends' release
  (Q19).

### 4.2 Proposed

Nothing in L5's design of the read changes. What changes is **where its steps run**:

1. **Registrar, automatically.** Inside onboarding's school step (§1.3 step 3), on the school
   profile. The student's ask is the school step itself; its "why" line says Knowlu will read the
   class schedule. R24 still writes matched courses as the student's, so they appear on "Your week"
   (step 10) already set, each with Edit.
2. **Syllabus skim, in the same sitting.** L5's `capture_syllabi` runs after the course list, on the
   school profile instead of the wizard's incognito window. Its command moves to the console list
   with the rest of onboarding (§1.9).
3. **Read during onboarding, waited for at the reveal.** After the skim (step 3) and any upload (step
   9), the app runs L5's `judge --only syllabus`. By step 10 most reads are back, so syllabus meetings,
   exams, weights and late rules are **confirm rows on "Your week"**, not cards. A read still running
   at step 10 is waited for up to the reveal's cap (§1.5, Q7), then arrives as L5 designs it.
4. **Simple Syllabus (L5 Offer A)** uses the school profile where the campus row's
   `shared_sign_in` says it shares the school's sign-in; otherwise its own window, as L5 designs it.
   It is offered in step 9, only for courses step 3 found no syllabus for.
5. **What stays manual:** a tap to confirm each row, and Edit on anything wrong. Typing times stays
   the last resort for a course no registrar, syllabus or calendar covers; its entry sits under that
   course's syllabus offer (L5 §3.1), not as a separate question.

**One L5 rule this touches.** L5 holds syllabus cards to day 2 and files at most 8 a day (Q21, Q24),
to keep the first day calm. Rows confirmed on "Your week" are the student's own answers, not deck
cards, so they spend none of the 15 or the 8. What was not confirmed there still goes to the deck as
L5 rules. This needs L5's spec amended (§11).

### 4.3 Options (ruled: A)

The source order is ruled (L5's ruling 2 and its Q6). Quinn's ruling settles that the syllabus read happens
before the main page; Q7 ruled how long the reveal waits: up to 3 minutes, then it names what is
still running.

| Option | For | Against |
|---|---|---|
| **A. Wait for every read, up to 3 minutes after step 10, then name what is still running (ruled)** | Quinn's "everything has run once" in the normal case; never an endless spinner | A rare slow read lands after the reveal |
| B. Wait with no cap | Literal | A failed or slow cloud call holds the student at the door |
| C. Never wait | Fastest | Against Quinn's ruling |

### 4.4 Cost and risks

**Cost: S on top of L5** (moving `capture_syllabi` and the registrar trigger into the school step;
the confirm rows on "Your week"). It depends on L5's PR-1 and on §2.

**Risks.** The registrar writing as the student without a separate press: the school step's copy
must say it, and R24's rows stay editable (the registrar's apply-and-list rule, ruling 3). A school
with no registrar row and no syllabus in the LMS still needs Offer A or B, which asks the student for a
file: allowed ("uploading a file is not typing", `VISION.md`), and still one action per course.

---

## 5. Late-work policy from syllabi

Quinn: "we should be accounting whether or not late work is worth submitting via syllabi."

### 5.1 Today, as ruled

L5 already **reads** the late rule (syllabus spec §16, Quinn's Q10 ruling): the screen's Policies
part, the model's `policies.late` block, a validated single-line `late_policy` on the course note
(`{accepted, penalty_percent, per, max_days, grace_hours}`), proposed on the course's one amend card
with weights and scale, `AMENDABLE_FIELDS` widened on the contract list (§16.3). It is **shown**:
"Late: −10% a day, up to 3 days (your syllabus)", "Late work not accepted", "Past the late window".
**`rank` does not read it** (Q34, ruled 2026-10-19 as recommended: "a misread rule never reorders
work"). Today a past-due task is Must do and ranks first, forever
(`engine/src/ranking.rs:229-273`, the overdue tests).

Grade weights are read too (§4.7, `grade_weights` on the course note). Nothing uses them to judge
whether an item is "worth it".

### 5.2 Ruled (Q6, Quinn, 2026-10-23): Q34 reopened to its option (b), narrowly

Quinn's new ask is exactly the alternative Q34 set aside. Ruled: past its late window a task leaves
Today into "Late window closed", archived only by the student's tap; "Still doing it" sets a new date.
Three guards:

1. **Only a rule the student confirmed.** `rank` reads `late_policy` only from the course note, which
   holds a value only after the student approved the amend card (or typed it). No model is involved
   at rank time (decision 11 holds).
2. **The window.** For a past-due task of a course with a whole rule: `accepted: false` closes the
   window at the due time plus `grace_hours`; `accepted: true` closes it at the due time plus
   `max_days` (plus grace). With no `max_days`, the window stays open.
3. **What closing does.** Inside the window the task stays Must do, as today, with its label ("−20%
   now; closes Friday"). Past the window it **leaves Today's list and Must do** and waits in a small
   group, "Late window closed", with two buttons: **Archive** and **Still doing it**. No automatic
   archive: archiving is the student's tap, so a misread rule can hide a task from the order but never
   delete or close it, and the student sees it there.

"Still doing it" asks one thing, "When will you turn it in?", and sets that date as the task's `due`,
as the student, through `write`. L5 already rules that a due the student set by hand is an extension
and the late rule "changes nothing" for it (§16.3), so the task returns to the list and is never
hidden again. No new field is needed.

**Not proposed now:** using grade weights to decide "worth it" (a 1%-weight item at −50% is worth
little). That is judgment about value; it is GPA and priority work, Beyond (L5 non-goals; grades spec
G2). The window above is arithmetic on a rule the syllabus states.

### 5.3 Where it lives, and contract impact

| Piece | Where | New? |
|---|---|---|
| The rule | `courses/<slug>.md` `late_policy`, single-line flow mapping through `write` | No: L5 T4e |
| Confirming it | the course's amend card; `AMENDABLE_FIELDS` gains `late_policy` | No: L5 (contract list) |
| The window and the hide | `rank` (ordering and Must do) and `surface` (the "Late window closed" group), both pure over notes | **Yes**: a ranking change |
| Archive | the existing archive action, as the student | No |
| Still doing it | a hand-set `due` through `write`, as the student | No |
| The judgment | L5's one `syllabus` kind in the cloud, read by `judge`, written to `state/syllabi/` | No |

No new note folder, no new frontmatter field, nothing new that syncs. The one contract-level fact: the
read model gains a group, so `surface`'s output changes for a vault that holds a course with
`late_policy` and a task past its window. `surface-today-*.json` are generated from fixtures with no
`late_policy`, so they do not change; a test pins both that and the new group on a scratch fixture.
Whether `rank` already loads course notes is to be checked in the plan (`judge` does).

### 5.4 Options (ruled: A)

| Option | For | Against |
|---|---|---|
| **A. Hide past the window, archive by tap (ruled)** | Quinn's ask; a misread rule cannot close or delete work; nothing new to sync | Reverses Q34 of 2026-10-19; a ranking change with its own tests |
| B. Keep Q34: label only | Ruled; no ranking change | A dead task still tops the list every morning |
| C. Hide and auto-archive past the window | No tap | A misread rule silently closes real work; against "Knowlu noticed → propose" |
| D. Also weigh grade weight and penalty to decide "worth it" | Closer to Quinn's words | Judgment about value; Beyond (GPA, priority) |

### 5.5 Cost and risks

**Cost: M**, after L5's PR-1 (the field must exist). Implementer for `rank` and `surface` (not on the
contract list), console-ui for the group, test-writer first (window arithmetic, the hand-set due
exemption, frozen references unchanged).

**Risks.** A rule the model misread and the student approved without reading. Mitigated: the task is
never closed, only moved to a visible group with its rule beside it. A policy that differs by kind of
work is quote-only in L5 (§16.3), so no window is computed for it. Time zones: the window is counted
in the vault's zone, like `due`.

---

## 6. Turned-in work is not a task

Quinn: "I shouldn't have to micromanage assignments as soon as I create the account." The new vault
had 28 live tasks: 20 from Blackboard and 8 from zyBooks, many already submitted. Every one was due
today or later; the first-run past-due filter worked (59 archived).

### 6.1 Today

- **The Blackboard tasks come from the LMS feed** (`ingest`, `created_by: blackboard`,
  `engine/src/ingest.rs:513-531`). Their `source_uid` is the calendar entry's raw UID
  (`ingest.rs:798`); a gradebook item's reads `_blackboard.platform.gradebook2.GradableItem-<id>`
  (the shape in `ingest.rs`'s tests, e.g. `…GradableItem-_4686399_1`). The feed carries due dates,
  never "submitted". *(Corrected 2026-10-23: an earlier revision wrote `blackboard:<id>`.)*
- **Grades already see submission.** The `grades` step turns each gradebook column and the student's
  row into a grade note with a status: `graded`, `needs-grading` (submitted, not yet marked),
  `in-progress`, `not-submitted`, `exempt` (`engine/src/grades.rs:150-173`, `:186-214`). The column
  id is the note's `source_uid` (`grades.rs:217`). Nothing connects a grade note to its feed task.
- **zyBooks completion files a card.** Tier 1 completion detection (`engine/src/completion.rs`,
  actor `agent:knowlu.completion`) proposes `status: done` as an amend card when zyBooks reports
  exactly 100%. Cards are capped at 15 a day and held on the vault's first day, so on day one the
  student sees 8 live zyBooks tasks that are finished.
- **First sight of a source archives the past, not the finished.** `coursework` writes an item that is
  already past straight into `archive/` as `imported-past` when its source group is new
  (`engine/src/coursework.rs:475-535`); `ingest` does the same (`ingest.rs:536`, `:774`). A finished
  item due next week is created as a live task.
- **Slot order** (`app/src/scheduler.rs:515-548`): sync, coursework, ingest, grades, judge, rank. The
  grades step runs after ingest and before judge and rank.
- **Effort is judged on every live task**, so each finished task created live also spends a task-judge
  call. Task judging moved on 2026-10-23 (Quinn) to `qwen/qwen3.5-35b-a3b` at DeepInfra, with the
  effort anchors at `task-3`; the staging eval reads `effort_mae` 0.273. Archiving turned-in work at
  first sight removes those calls and their estimates from the first day's total.

### 6.2 Proposed

**What counts as turned in.** Only what the source itself says, never a model:

| Source | Done | Still live |
|---|---|---|
| Blackboard gradebook (grade note status) | `needs-grading` (submitted, not marked), `graded` with a score above 0, `exempt` | `in-progress`, `not-submitted`, `graded` with a score of 0 or no score |
| zyBooks, VHL | exactly 100% (tier 1's rule, unchanged) | below 100% |

Only gradebook item columns join (`kind: item`; an Overall column never does). **Ruled 2026-10-23
(Quinn; the no-score half by the controller on Quinn's reasoning; lane O1's plan, Q-A):** `graded`
counts only with a score above 0. A graded zero is not turned in: it stays live, and the late-work
rule decides (§5). A `graded` row with no score also stays live, because every column type that
yields one (`Percent`, `Tabular`, `Text`, or no `displayGrade`, `grades.rs:188-213`) can hold a zero
or a fail. Separately, a task with a "Mark done" card the student rejected or has not answered is
never marked by Knowlu (lane O1's plan, review round 1, F1).

**How it lands.** The LMS and the homework platform are authoritative on their own items (VISION
commitment 5; L5 Q45 already treats `coursework` items as authoritative). So:

1. **At first sight, archived rather than created.** When a source group is new (the rule
   `imported-past` already uses), an item that is already done is written once, straight into
   `archive/`, with `archived_reason: imported-done`, recorded as seen, one line in the run log. It
   never ranks and is never judged (no effort call is spent on it). The reveal (§1.8) says "N already
   turned in were archived", once, and lists none of them.
2. **After that, applied and listed.** A live task whose source now says done gets `status: done`,
   written by `agent:knowlu.completion` through `write`, journal first, and listed with **Undo**
   where the page already lists closed work ("Closed this week", `surface::closed_this_week`, which
   marks an agent's close as `agent`). Undo sets `status` back to `active` as the student, so
   judge-once then keeps it active (the mail lane's D10 Undo pattern).
3. **Judge once, unchanged.** If the journal shows the student set `status` by hand
   (`journal::human_set`, `op: set`), nothing is written and nothing is proposed: tier 1's rule 3
   already skips such a task (`completion.rs:12-19`, `:230-237`). Tier 1's amend card is retired
   for authoritative sources; `propose_done` stays for evidence that is not authoritative (mail's
   completion claims, `engine/src/mail.rs`).
4. **The join is exact.** A feed task is matched to its gradebook column by id only (the feed's
   UID's `<id>` after `_blackboard.platform.gradebook2.GradableItem-` against the grade note's
   `source_uid`, the column id, e.g. `_4686399_1`, `grades.rs:217`), never by title. A feed UID of
   any other shape is never joined.
   That the feed's id *is* the column id is **unverified** (S-C, §12). If S-C fails, Blackboard
   completion waits for a join the spike finds, and zyBooks ships alone.

**Where the Blackboard half runs.** The grades step runs after ingest, so on the vault's first slot
ingest has already created the 20 tasks before grades knows they were submitted. Two ways; **Q11
ruled A** (Quinn, 2026-10-23): the slot becomes **sync → coursework → grades → ingest → judge →
rank**, and the `CLAUDE.md` slot line changes in lane O1's plan.

| Option | What | For | Against |
|---|---|---|---|
| **A. Grades before ingest (ruled)** | Slot becomes sync, coursework, grades, ingest, judge, rank. Ingest's first-sight path reads the grade notes and archives done items as `imported-done` | Literally "archived rather than created"; one record per item | Changes the slot order `CLAUDE.md` states, so it is Quinn's rule change; the plan first checks `grades` reads nothing `ingest` writes |
| B. Keep the order; the grades step finishes the job | Ingest creates; the grades step then moves first-day done items to `archive/` (and marks later ones done) | No rule change | Two journal records per item, and a task that exists for a few seconds mid-slot |

zyBooks and VHL need neither: `coursework` has the completion figure in hand when it creates the item,
so it archives a finished item at first sight in the same branch as `imported-past`
(`coursework.rs:475-535`).

**What "first sight" means per source.** For `coursework` it is a new source group (a book or
section the vault has never held, `coursework.rs:315-335`). For `ingest` it is the vault's first
ingest (`ingest::is_first_run`, `state/today.md` absent, `ingest.rs:473-495`). Until lane O3 moves
the grades sign-in into onboarding, today's wizard connects grades only after Finish, so the first
slot usually runs with no grade notes: Blackboard work turned in before then is created live and
then marked done, with Undo, on the first grades run (path 2). The reorder still matters: from the
first slot that has a grades bundle, grades always land before ingest reads them.

### 6.3 How "done" lands: ruled (Q3, Quinn, 2026-10-23)

**Turned-in work is marked done automatically, with Undo; on a new vault it is archived at first
sight.** That is option A below; it replaces tier 1's cards for authoritative completion. The
completion design note (2026-09-22) and its tests change with it (§11). Judge-once stands: a `status`
the student set by hand is never overwritten and never proposed (tier 1's rule 3).

| Option | Status |
|---|---|
| A. Apply and list, with Undo | **Ruled** |
| B. Keep cards after the first run | Not taken |

In the long onboarding, first sight happens during step 3's and step 8's background work, so the
reveal's "What Knowlu tidied" line (§1.8) is where the student first sees the count.

### 6.4 Cost and risks

**Cost: M.** Spike S-C (main session). test-writer first (first-sight archive for each source, the
done table above, the hand-set exemption, Undo). implementer: `coursework.rs`, `ingest.rs` (which
now runs after grades, so it reads the grade notes and does the Blackboard half, creating and
marking alike; `grades` keeps writing only `grades/`), `completion.rs` (none on the contract list;
`write` and `journal` are used, not changed), and `surface.rs`'s "Closed this week" row gaining its
Undo flag. console-ui: the first-run line and Undo on "Closed this week". `app/src/scheduler.rs`'s slot
order and the `CLAUDE.md` slot line (Q11, ruled). Plan: `docs/plans/2026-10-23-o1-turned-in-work-plan.md`.

**Risks.**
- **A wrong join marks the wrong task done.** Guarded by the exact-id rule and S-C; Undo is the
  backstop and sits on "Closed this week", where the student looks.
- **"Submitted" is not "finished"** where an assignment allows resubmission. The student undoes it
  once; judge-once keeps it.
- **Grades are not available everywhere** (curated Blackboard schools only, `grades.rs` predicate).
  Elsewhere Blackboard work keeps its due date until the student closes it; email receipts (the
  completion design's T9) are the later path.
- **`imported-done` is a new value** of a field existing vaults carry. It is additive: the mail lane's
  unarchive arm matches `imported-past` only (`engine/src/approvals.rs:200-206`), so a done item is
  never brought back by a later date change.

---

## 7. Past-term courses

Quinn: the "what are your classes" step listed past classes. **Being fixed now** on branch
`fix/current-term-courses` (worktree `.claude/worktrees/current-term`): a current-term filter in
`lms_link::courses_from_json` (`app/src/lms_link.rs:483` there). Nothing to design. Under §2 the same
course list feeds the registrar match, the syllabus skim and grades, so the filter's rule must be the
one all three use; the plan checks that it is.

---

## 8. First-run expectations

**The bar:** a new vault holds only live, realistically sized work. The student's first look at Today
should need no clean-up.

| What made day one noisy | Fix | Where |
|---|---|---|
| Past-due imports | already archived as `imported-past` (59 on Quinn's install) | on `main` |
| Turned-in work (28 live, many submitted) | archived at first sight as `imported-done` | §6 |
| Past-term courses | current-term filter | `fix/current-term-courses` |
| Effort far too high (median 8 h, about 155 h in all) | anchors by item type, points and weight in the task judge prompt; task judging moved 2026-10-23 (Quinn) to `qwen/qwen3.5-35b-a3b` at DeepInfra, anchors `task-3`, staging eval `effort_mae` 0.273 | `fix/effort-anchors`; the model move is server-side |
| Effort spent on finished work | turned-in items archived at first sight are never judged | §6 |
| Dead past-due work topping the list | hidden past its late window | §5 (after L5) |
| A first slot before any source is connected | the first full slot runs at step 10, and the main page waits for it (the reveal) | §1.5, §1.8 |
| Too many cards on day one | caps stay: 15 a day, syllabus held to day 2, L3's first-day allowance | L3, L5 |

**One measure for the friends' release:** on a fresh scratch profile at the founder's school, the
main page after the reveal shows no task that is past its window, already submitted, from a past term, or
estimated above a stated ceiling for its kind. The plan turns that into a live-proof checklist.

---

## 9. Lanes, and what ships to friends first

Each lane is one branch with disjoint files where possible. `console.js` stays one queue (HANDOFF §2's
rule); `app/src/main.rs` and `commands.rs` stay lane L6's until it merges. Agents are the roster's.

### Wave A: to friends first (no shared-profile ruling needed)

| Lane | What | Depends on | Agents, and why |
|---|---|---|---|
| **O0. Fixes and root causes** | `fix/current-term-courses` and `fix/effort-anchors` (in flight); root causes for the wrong mail summary (item 3) and the silent "See what came from your email" (item 4); the Outlook "already forwarding to Knowlu" case (item 2); prod's Gmail and Calendar reads (item 12: Quinn enabled both APIs on the one client on 2026-10-23; O0 re-runs one read of each and records it); the updater button (item 13, worktree `button-feedback`) | nothing | `debugger` for items 2-4 (root cause before any fix); `console-ui` for page fixes; `cloud-engineer` for item 12's configuration, with the controller running any prod change on Quinn's go |
| **O1. Turned-in work** (§6) | first-sight `imported-done` for zyBooks, VHL and Blackboard; apply-and-list after; Undo; the slot order sync → coursework → grades → ingest → judge → rank and its `CLAUDE.md` line | S-C (Blackboard half only); Q3 and Q11 ruled | main session for S-C (live, structure only); `test-writer` first; `implementer` for `coursework.rs`, `ingest.rs` (the gradebook join), `completion.rs`, `surface.rs`'s Undo flag, `scheduler.rs` (specified, test-checked, not the contract list); `console-ui` for the first-run line and Undo; `docs-keeper` for the slot line in `CLAUDE.md` and the references; `reviewer` before push. Plan: `docs/plans/2026-10-23-o1-turned-in-work-plan.md` |
| **O2. Gmail permission first** (§3) | in today's wizard and Settings: offer Gmail permission first while fewer than 100 accounts hold a grant; offer forwarding on decline, on a failed read, or past 100; hide Gmail forwarding while the grant is active; the one-line 7-day history note | S-D; Q4, Q5 ruled | `console-ui` (one panel, one Settings row, one static test); `cloud-engineer` (the count of permission users, the combined Gmail and Calendar ask on the one client); `reviewer` |

### Wave B: the long onboarding (rulings in; after spikes S-A, S-B, S-E)

| Lane | What | Depends on | Agents, and why |
|---|---|---|---|
| **O3. The long onboarding** (§1) | the vault at step 2 and the onboarding mode of the console; eleven steps with why, ask and proof; the progress rail; `onboarding.json` resume; skip rules (Q12); payment after the school step's proof (Q2); step 3's device-side counts; the reveal and its 3-minute gate (Q7); onboarding commands moved to the console list; the vault at the account step (Q13) | L6 merged (it owns `main.rs`); S-E for the copy and the measured waits | `planner` (the C1 rulings to carry over, the step-by-step plan); `console-ui` (the steps, the rail, the reveal); `implementer` (onboarding mode, resume, counts, gate, command moves, defaults); `reviewer`; Quinn reads every "why" line |
| **O4. One school sign-in** (§2) | `school-session` profile; grades, registrar, LMS capture and the school mailbox on it; Forget and sign-out semantics; the Settings row; the privacy sentence | Q1, Q9 ruled; O3; S-A, S-B | `contract-engineer` then `contract-reviewer` for `app/src/account.rs` (sign-out and deletion, contract list); `implementer` for the window modules; `console-ui` for Settings; `reviewer` for the page text; Quinn reads the page |

O3 and O4 ship together: O3 alone lengthens the onboarding but saves no sign-in. Q2's ruling (B)
changes no entitlement gate, so neither lane touches `engine/src/entitle.rs`.

### Wave C: after L5's PR-1

| Lane | What | Depends on | Agents, and why |
|---|---|---|---|
| **O5. Syllabus and registrar inside onboarding** (§4) | `capture_syllabi` and the registrar read in the school step; `judge --only syllabus` during onboarding; syllabus rows confirmed on "Your week"; Offer A on the school profile where shared | L5 PR-1; O3, O4 | `implementer` (small, specified); `console-ui` (the "Your week" rows) |
| **O6. The late window** (§5) | `rank` and `surface` read `late_policy`; the "Late window closed" group; Archive and "Still doing it" | L5 PR-1 (the field); Q6 ruled | `test-writer` first (window arithmetic, frozen references unchanged); `implementer`; `console-ui`; `reviewer` |
| **L1. M365 older mail** (§3.5) | as its own plan | Q10 ruled: after O1, O3 and O4; runs on the school profile | as its plan says |

**Sequence in one line:** O0, O1 and O2 now; spikes S-A to S-F in parallel; O3 with O4; L5 as
planned, then O5 and O6; L1 after O1, O3 and O4. Until O3 ships, friends get today's wizard with O1
and O2 in it.

---

## 10. Rulings (Quinn, 2026-10-23)

Every question this spec asked is ruled. The direction of Q2 (one long onboarding where every login
happens, each step explaining its benefit, the main page only after everything has run once) and
**Q3** (turned-in work is marked done automatically, with Undo; archived at first sight on a new
vault) were ruled first; the twelve below the same day.

| Q | Ruling | As recommended? | Where |
|---|---|---|---|
| Q11 | Grades run before ingest: the slot is sync → coursework → grades → ingest → judge → rank. The `CLAUDE.md` slot line changes in lane O1's plan | Yes | §6.2 |
| Q4 | Gmail by permission first for friends; forwarding only if the permission is declined or the read fails | Yes | §3.2 |
| Q1 | One kept school sign-in (option C), built after spike S-A passes | Yes | §2.3 |
| Q9 | One "Forget school sign-in"; the profile goes with its last use, and always on sign-out or account deletion | Yes | §2.5 |
| Q2 | Payment right after the school step's proof (option B); no change to the entitlement gate | Yes | §1.4 |
| Q7 | The reveal waits up to 3 minutes after "Your week", then names what is still running; backfills are never waited for | Yes | §1.5, §4.3 |
| Q12 | Optional sources may be skipped, each with its cost named. Account, payment, "Your week" and the school sign-in may not; the school sign-in falls back to the feed link | Yes | §1.7 |
| Q13 | The vault is created at the account step; onboarding is a full-screen mode of the app | Yes | §1.9 |
| Q8 | **One Google client** for sign-in and data, for now. Prod already uses the sign-in client for Gmail and Calendar, and Quinn enabled both APIs on it. Reason: one consent screen. Mitigation: the emailed-code sign-in is the fallback if Google ever restricts the app. Revisit at Google verification | **No: overrides** the recommendation (separate clients) | §1.3, §1.11, §3.3 |
| Q5 | Gmail by permission first for the first 100 Gmail-permission users; forwarding beyond that. Verification and CASA decided at Launch or at 100 users | **Amended:** the recommendation was forwarding by default for paid students | §3.2 |
| Q6 | Q34 reopened: past its late window a task leaves Today into "Late window closed", archived only by the student's tap; "Still doing it" sets a new date | Yes | §5.2 |
| Q10 | L1 (M365 older mail) comes after O3 and O4 (the shared sign-in) and O1 | Yes, with O1 added | §3.5, §9 |

What remains open is not a question for Quinn but a fact: the spikes in §12. A spike that fails sends
its section back to Quinn with the finding, never a quiet redesign.

---

## 11. Signed texts these rulings amend

Nothing below is edited by this spec. Each amendment is written in the lane that needs it, by its
text, never by line number.

| Document | What changes | For |
|---|---|---|
| Cloud design, Amendment 2026-09-29 ruling 12 | "the calendar-link and course-list captures keep their discard-everything window unchanged; the registrar never shares this profile" becomes one kept school profile, per identity | Q1, Q9 |
| Cloud design, Amendment 2026-09-30 | "a saved mail sign-in, one WebView2 profile per connected mailbox" becomes per identity; deletion rules as §2.5 | Q1, Q9 |
| Cloud design §4.2 (the wizard) and §11a's "keep nothing" rows | the long onboarding's steps, payment's place, the main page after the reveal; the kept school session's wider use | Q1, Q2, Q7, Q13 |
| `VISION.md` pilot paragraph | "the kept Blackboard sign-in and the saved mail sign-in" becomes the kept school sign-in and saved mail sign-ins | Q1 |
| Email forwarding spec | D6 (setup runs inside onboarding, not at the console's first open), D15 and §4.13 (profiles and deletes), §4.2 step 6, §7's privacy sentences; D22 unchanged, its UI default noted | Q1, Q4, Q9, Q13 |
| Grades spec §4, §11 | `lms-session` becomes a use of `school-session`; Forget; privacy sentences | Q1, Q9 |
| Grades spec §10, §12 | the slot line (grades before ingest); "pinning ICS tasks to courses by gradebook column" leaves §12's out-of-scope list for the narrower join of §6.2 (status only, in `ingest`, never `course` or `due`) | Q3, Q11 (lane O1) |
| Commitment model phase 3 spec D1, its plan's R4-a, the close-out plan, `HANDOFF.md` §3 | "the registrar never shares the kept Blackboard profile"; the registrar runs inside onboarding's school step | Q1, Q13 |
| Syllabus spec §3.1, §15.2, §16.3, §16.8, Q21, Q24, Q34; L5 plan's command placement | step 1 and Offer A on the school profile; syllabus rows confirmed on "Your week" outside the deck's caps; `rank` reads `late_policy` | Q1, Q6, Q7, Q13 |
| Completion detection note (2026-09-22) §2, §7a; stream J T8 | cards become apply-and-list; `imported-done` at first sight | Q3 (ruled) |
| C1, C1b and C1c specs; email spec D6 | the wizard's panels become the long onboarding; the vault at the account step; payment's place; the first run before the reveal; school mail inside onboarding | Q2, Q7, Q12, Q13 |
| Gmail connect spec | the Gmail step's place and copy; permission first for the first 100 permission users; Gmail and Calendar asked in one consent | Q4, Q5, Q8 |
| Cloud design §5.3 and any text naming a separate Google data client or project | one Google client for sign-in and data until verification; the emailed-code fallback | Q8 |
| Cloud design, Amendment 2026-10-16b | "students past it use email forwarding" stands; the cap is now filled with permission-first users | Q5 |
| `CLAUDE.md`, `docs/reference/app.md` (scheduler) | the slot order line: sync → coursework → grades → ingest → judge → rank (lane O1) | Q11 |
| `site/privacy.html`, `PRIVACY_VERSION` | the kept school sign-in and its uses; Gmail permission versus forwarding | Q1, Q4 |
| `docs/reference/app.md`, `HANDOFF.md` §2 | the new lanes, the long onboarding, the profiles | all (docs-keeper) |

---

## 12. Spikes and facts to settle before a plan

| Id | Question | How | Who |
|---|---|---|---|
| S-A | At the founder's school, does one visible sign-in on one profile let the LMS (`users/me` 200), Outlook (a signed-in mailbox), the registrar (its rows call) and Simple Syllabus come up **silently** in later hidden windows? How many MFA prompts? | Founder's scratch profile, a dev build, Quinn at the machine for MFA; record structure and counts only | main session (live; the proof harness cannot drive a school sign-in) |
| S-B | Can the grades, registrar and LMS windows run on a data folder that also hosts the mail window, given `MAIL_BROWSER_ARGS` and WebView2's one-environment rule? Must the LMS capture stop being incognito? | the same dev build; two windows in turn and together | main session |
| S-C | Is the `<id>` in a feed task's `…GradableItem-<id>` UID the gradebook column id of the same item? | compare a scratch vault's feed tasks with its grade notes, ids and counts only | main session |
| S-D | The one Google client's project (Q8): its publishing status, token lifetime and test-user list on prod, and whether Testing limits Google sign-in to test users; how Google counts the 100 restricted-scope users (Q5); one prod Gmail and Calendar read now that Quinn enabled the APIs | Google Cloud console, read-only; Quinn changes anything | controller, with Quinn |
| P-1 | Does `grades` read anything `ingest` writes? (Q11, ruled; lane O1's first task confirms) | code read | lane O1 |
| P-2 | Does `rank` load course notes today? (§5) | code read | planner |
| S-E | How long does each step and each background job really take (§1.3, §1.5)? Whether Q7's ruled 3-minute cap is usually met, and the "about 10 minutes" line, depend on it | a dev build against staging, a scratch profile at the founder's school, timed end to end, counts and seconds only | main session |
| S-F | Can step 3's proof counts (classes, assignments, already turned in, grades) be computed on the device from the raw captures, before payment and before any note is written? | code read of the capture bundle and course list, then the S-E run | planner, then main session |
| S-G | Can the school step's hidden background windows keep running while the student is in the browser paying, and while the visible onboarding window changes steps (WebView2 hidden-page throttling, the C3 note's P3 lesson)? | the S-E run | main session |
| P-3 | Can the console start in a full-screen onboarding mode with the tray, scheduler and autostart off until the reveal, and relaunch into it at step 2 (`onboarding::relaunch_with`)? | code read | planner |
| R-1 | Quinn's premise that longer onboarding converts better: what the evidence says for paid consumer apps, and which parts (benefit lines, progress, proof) carry the effect | desk research, sources cited | `researcher` |

Every row is read-only or runs on a scratch profile against staging, cleaned up after. Nothing that
depends on a spike is built until it passes.

---

## Appendix: feedback items this spec does not design

These look like bugs or configuration; each gets a root cause first (lane O0), never a design around
it.

- **Item 2.** Mail setup asked Quinn to set Outlook forwarding by hand although it already forwarded to the
  Knowlu address it named. The step should recognise its own address as already set (email spec §4.2
  step 3 covers only "forwarding elsewhere"). Likely tied to the held backfill making the step guided.
- **Item 3.** The summary listed "Bring in recent mail" and "Finish" as not done after "I've done
  this". The guided answer seems not to be recorded as done; `backfill_held_for_c3` is the context.
- **Item 4.** "See what came from your email" does nothing; Quinn suspects a covered popup.
- **Item 12.** Prod's `gmail-read` 500 and Calendar 502: the one client lacked the Gmail and Calendar
  APIs; Quinn enabled them on 2026-10-23. O0 re-runs one read of each (S-D).
- **Item 13.** "Check for updates" gives no feedback; "Update now" needs two presses. Branch
  `button-feedback` exists.
