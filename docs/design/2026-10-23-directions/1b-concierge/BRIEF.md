# Direction 1b: A conversation that hands you the list (`concierge`)

Written 2026-10-23 for the five-direction UI run. Inputs: Notion, Stripe, and **Superhuman's 1:1
onboarding session, run on the user's own inbox** as the third reference. This brief is the build
contract for the prototype in this folder. Sample data is `../shared/sample-student.json` (Maya
Okafor, Halverson State University; every person, school, room and address in it is invented). Every
number below comes from that file. Ruled facts come from
`docs/specs/2026-10-23-onboarding-redesign-draft.md` ("the spec") and `../research/product-truth.md`;
where this brief and the spec disagree, the spec wins.

**Thesis.** Superhuman made onboarding a live session on the user's real inbox. Knowlu runs the
same session with no human in it. Each step is one exchange: Knowlu's why line, one action, then
Knowlu's reply with the proof in numbers. Undo and Confirm are offered on real items the
moment those items appear, never on a tutorial (Pin waits for the main page; see §0). The reveal is
Knowlu's first morning brief, and the main page keeps the list in the body of the page from its top,
with Knowlu's letter and notes in the margin beside it.

**What it refuses.** A chat app. Knowlu's lines are set as prose on a page, not bubbles. Onboarding
has no text box: every answer is a chip, a system sign-in window, a file dialog or a checkbox. The
main page never puts the conversation above the list; the list owns the left of the page and Knowlu
writes in the margin.

**How it differs from the other four.** Not a checklist that becomes the list (1), not one sheet per
step with the week withheld (2), not a ledger of hours (3), not a hub of source tiles (4), not lanes
split by actor (5). Here the structure is **an exchange**: why, act, proof, and the first use of a
gesture, in the order the student meets their own data.

## Round 2 (2026-10-23, after the five-direction comparison)

The prototype wins over anything below that disagrees.

- **Less to read.** Every step on the main path is one line from Knowlu plus what it asks. The
  welcome is one sentence; the account step's four-row "where things live" table folds behind a
  "Where your things live" link; school, email, Google, syllabus and week-check copy is cut to
  one line each (roughly 15 paragraphs to the week became about 6).
- **Steps take the shape of what they ask** (`.stage.k-<kind>`): school is a picker list (`S`); a
  sign-in is a three-row "You'll need · You type it on · Knowlu keeps" table (`S-need`, the site
  sheets); proof is a large sentence over ruled rows (`S-proof`); school email is a single
  switch row (`E`); Google is a two-row scope list (`G`); the syllabus is a drop zone (`Y-cs110`);
  one-fact checks (`K1`, `K3`) are a large question with two big answers; tables (`K2`, `K4`) widen
  to 46rem. The stage sits higher (9vh, was 13vh).
- **Daily page (`T`).** The list starts at the top of the main column: "Today · Monday, October 26 ·
  6h 30m of work", then Lab 6 with Start. The day strip moved below the list as "Your day".
  The margin opens with Lab 6's why, level with its row.
- **Leader line fixed.** The note is aligned to its row (`alignTo`) with the margin-top transition
  turned off before measuring, so the line runs straight across the gutter. The reveal's Thursday
  note is placed the same way, level with Thursday.
- **The margin is a letter, not a dashboard.** The six count rows are gone: one sentence on the
  load, then one paragraph whose phrases ("3 things changed", "4 questions are waiting", "2 came
  off your list", "Exam 2", "closed 8", "2 notices") open the same footnotes as before.
- **Reveal margin cut** from five notes to two, plus the Thursday note. "Still reading" shows only
  on `R-cap`.

## 0. What the build changed (2026-10-23, after two red-team rounds)

The prototype (`index.html`, ids in `screens.json`) is now the contract where it and the sections
below disagree. Each change answers a red-team finding; the spec still wins over both.

**Structure**

| Was | Now | Why |
|---|---|---|
| Today: brief paragraph over the list, nine stacked regions in a 28rem margin | The **list owns the main column from its top** (day strip, class line, then row 1 at about y 270). The brief moved into the margin as **Knowlu's letter**: at most four sentences, and each region (What changed, Asks, Late window closed, Coming up, Closed this week, notices) is a linked phrase that opens as a footnote under the letter, one at a time. The focused row's note sits below it, joined to the row by the leader line | Red team: the first row sat below the fold, and the margin was a dashboard sidebar. Now the 2-second test lands on row 1, and the margin is one paragraph with footnotes, not nine widgets |
| Rail: the full ruled order of about 15 sources from step 1, "About 7 minutes of yours left" | The rail is **what Knowlu has so far** (earned lines only; the four newest keep their proof), then **Now**, then one "Then …" line that names the remaining steps only once the school proof has made them known. The heading counts the wait: "Your week in about 10 minutes / About 7 of yours, then up to 3 while I finish reading" | No frightening list of unknowns, no time that jumps, and the reveal is named as the goal from step 1 |
| Reveal: heading, brief, 7-row table, list, button below the fold, per-row editors | A lead sentence (what's next, the heaviest days, free time tonight), **"Tonight and tomorrow" rows first**, the week table as support, and one **Open Knowlu** in a sticky foot so it never falls below the fold. No per-row editors (spec §1.8: one button) | R overflowed 800 px and buried the aha |
| Proof auto-advanced after 1.2 s (`A-ok`, `P-ok`) | No timers. Proof stays in the thread; the next question appears under it | Proof and "Show me the folder" vanished on a timer |

**Gestures**

- `S-undo` is **removed**. Before payment nothing is written (engine gate, spec §1.4, S-F), so the school
  proof only counts: "26 assignments · 4 already turned in; they won't become tasks." First-sight
  archives are never listed and never get Undo (spec §6.2 item 1); the reveal's **Tidied** note says the
  count once.
- Undo is taught on real apply-and-list items only: `G-proof` (pre-setup Gmail, VISION's setup
  exception), the reveal's From your email, and Today's What changed and Closed this week (`T-changed`,
  `T-undone`).
- `W-pin` is **removed**: it taught overriding the ranking at the aha moment, before effort and rank
  were final. Pin lives on Today only (the row panel and `p`); a finished pinned task stays pinned until
  its due date passes (PRODUCT-DECISIONS).
- Confirm stays at `K1`, and gets a second true use in the wait: `W-cs110` asks the student to confirm
  CS 110's syllabus rows when they land (the data has it finishing 40 s into the wait).
- `K3` no longer "confirms" 19 facts with one press. The rows are "read as written", and Knowlu asks
  only about the one that isn't written anywhere (ENG 210's late rule).

**Facts corrected**

- School email (`E`) is a real consent, not "Nothing new to do". It says Knowlu turns on forwarding in
  the school mailbox to a private Knowlu address, read in Knowlu's cloud. Its follow-up and the reveal
  say mail already in the inbox before today **isn't brought in yet** (M365 backfill is held until L1).
  Nothing promises that older mail will arrive "as a question".
- Google: `G-warn` is its own exchange, shown only while Google's review is unfinished, with a drawing
  of the warning, why it appears, and **Use forwarding instead** as an equal choice. `G-cap` is the
  past-100-permission-users branch (Calendar by permission, Gmail by forwarding). `G-fwd` is
  Knowlu-run forwarding set up in a Gmail sign-in window (`G-fwd-win`), not three manual steps.
- Homework: `H-vhl` cites SPAN 202's syllabus, which Knowlu has read. `H-zybooks` is the "not known"
  variant: CS 110's syllabus isn't in yet, so Knowlu asks "Does it use zyBooks?". `H-sso` covers
  students who reach a site through Blackboard and have no password. The homework sheet is drawn
  differently from the school window (dashed edge, key glyph, "Knowlu keeps this password"); the
  school window keeps no password.
- Syllabi: `Y-cs110` offers **Choose the file**, **Connect Simple Syllabus** and **There isn't one yet**
  (spec step 9).
- No single-user assumptions: the MFA vendor and the LMS come from the campus row (`SCHOOL.mfa`,
  `SCHOOL.lms` in the prototype). `S-split` shows a curated school without `shared_sign_in`
  (Ridgeway College, invented: Canvas, no phone approval, school email and registrar asked later).
  `S-feed` names Blackboard or Canvas.
- The S-need privacy line now says only what is true: "No password is kept, by Knowlu or anyone."
- The reply after `A` adds a four-line **where things live** block (this computer / your account /
  Knowlu's cloud / nowhere), so "fetch on device, think in the cloud" is said once, plainly. `A` puts
  the Pell price ($2.49) beside $9.99.
- Today's numbers agree: the letter has four sentences; free time is "about 5.7h free, most after
  6:00 pm" (only 5.5h fit after 6:00 pm before 11:30 pm); the status line reads "Checked 6:00 pm
  yesterday · next at 12:00 pm" (no 6:04 am run). Reveal: "about 2.7h free, most of it after 9:00 pm".
- Counts reconcile: the wait says "40 assignments: 26 from Blackboard, 8 from zyBooks and 6 from VHL;
  9 already turned in"; the reveal says "92 items: 52 past and 9 turned in, archived; 34 to plan: 31
  open assignments and 3 exam-study sessions".
- Payment's summary ends with what is left after checkout ("about 4 minutes of yours, then your
  week"). `X` (Finish later) offers one reminder email with a link back (a proposal for Quinn;
  VISION's one-email-a-day limit holds).

**States added** (each named, never silence): `O-offline`, `A-restored` (an adopted account skips
finished steps), the relaunch at step 2 (one quiet line after `A`), `S-nojoin` (spike S-C fails: no
turned-in count), `P-nocount` (spike S-F fails: payment without counts), `S-slow` (grades slow),
`E-slow`, `E-own`, `E-skip`, `G-skip`, `G-closed`, `W-slow` (the 3:00 cap) leading to `R-cap`,
`T-offline`, `T-lapsed` (entitlement past the 72-hour grace: the list stays and stays ranked, reading
stops, **Update payment**), and `T-free` (all clear; empty regions as one quiet line).

**Type.** Mona Sans's tabular zero is slashed, which reads as a costume, so the build uses lining
proportional figures. Literata and Mona Sans are self-hosted in `assets/fonts/` (SIL OFL, downloaded
from Google Fonts), so the prototype makes no network calls.

**Still open for Quinn.** "Open Knowlu" is the spec's ruled label, but the student is already in
Knowlu; the build keeps it, with a line beside it ("Your list for today, with these notes beside
it"). The reminder email in `X` is new. Whether Knowlu says "I" (the prototype has the toggle).

### 0.1 Second pass (2026-10-23, after two critiques scored 57 and 61)

Where this list and anything above disagree, this list wins.

| Was | Now | Why |
|---|---|---|
| A scrolling thread: every past exchange stayed on the page, and the open one landed mid-page under clipped history | **The stage.** One open exchange at a fixed anchor (about 13% down), centred in the main area. Above it sit one quiet line ("6 answered so far · Show the whole conversation") and the exchange just answered, folded to its answer and reply. The full history opens on request. While a sign-in or checkout window is open, the column moves left and narrows so the window never covers it or the rail | It read as a chat log scrolled by a machine. Every shot opened on half-cut lines |
| The school proof rendered inside payment, then repeated as the order summary and in the rail | **`S-proof` is its own exchange.** One sentence ("Found 5 classes, 26 assignments and 4 already turned in at Halverson State."), the detail rows under it, and **Looks right**. Then `P` shows only the price, the first-charge date, cancel terms and what is left after checkout | Spec: proof after every login. The aha was spent next to the price |
| `E` was its own consent: two paragraphs and a **Turn on forwarding** button | `E` follows payment. The answered line above it reads "Your trial is on", then the spec's why line, then "Your Halverson State sign-in already covers school email, so there's nothing to sign in to. One thing changes in your mailbox…", **Continue**, and **Leave school email out** with its cost. The proof rows of `S-proof` also say "School email: covered by this sign-in" | Spec step 5: nothing new where step 3's sign-in covers it. The mailbox write is still disclosed |
| Rail: every source kept its sub-line, and sub-lines wrapped to orphans | Only the **two newest** keep a one-line proof (no wrapping). Older finished sources fold into one line ("Account, Halverson State, Payment"). The heading is "Your week in about 7 minutes / 4 with you, then up to 3 while I read" | The rail outranked the question being asked |
| Kickers above paragraphs ("School email, later", "Picking up where you left off", "Coursework", "Past work", "Tidied", "From your email", "Still coming", "What happens now", the row title above Today's note) | Removed. Follow-ups are a glyph and a sentence; reveal notes are paragraphs whose first clause names the subject; Today's note is joined to its row by the leader line alone | Craft floor bans eyebrows |
| Today: day strip, 8 course pills with grade letters and pace, a 13-square half-hour meter, then a 4-sentence linked letter | The day strip is neutral, and grades and pace pills are gone from the daily page. The list header reads "**6h 30m** of work · about 5h 40m free after it". The margin opens with one sentence that adds up ("4h 42m is due by tomorrow. With 1h 48m toward Exam 2, today's list is 6h 30m, and about 5h 40m stays free.") and then a **count list** (3 changed, 4 questions waiting, 2 off your list, Thu Exam 2, 8 closed this week, 2 notices), each row opening its footnote in place. The assistant line stays pinned under the margin and never scrolls away | The page failed the 3-second test, its four hour totals disagreed, and decimals ("4.7h") sat beside "1h 12m" |
| VHL sign-out appeared three times; the Lab 4 grade appeared twice | The sign-out appears only in the top bar. The grade is in What changed, and notices hold the room change and the missing mark | One fact in one place |
| Reveal: a rainbow of course pills, Saturday and Sunday under the sticky button, "about 2.7h" | One neutral text line per day for classes and commitments (optional ones in italic), due work in its own column, and an hours column. The two 4h days are tinted as the heaviest. All seven days fit above **Open Knowlu** at 1280x800. "about 2h 40m" | The table carried too much colour to say which days are heavy |
| `K4`: 3 rows × 4 preselected options, plus office hours, your day and definitions | Each row shows its level and says where it came from ("Firm · as your calendar marks it"), with **Change** to open the four options. Office hours and your day are one sentence with one Change link | 14+ controls in one exchange |
| `G-proof`: **Looks right** and **Undo** as equal buttons | **Continue**, with Undo as a quiet link at the end of the "I added it" sentence | Undo is a recovery action, not a required decision |
| `G-warn`, homework sheet | `G-warn` now leads with the benefit and specifics (read-only; can't send, delete or change; never sold; take access back on your Google Account's Security page). The drawing of Google's screen uses the page's own type. The homework sheet puts the why-it's-safe line **before** the fields (this computer only, Windows Credential Manager, removable in Settings in one click) | Both explained the mechanism, not the reason to trust it |
| `W-slow` only extended the wait | `W-slow` is the 3:00 cap: "Three minutes are up, so your week opens now. One thing is still running", then **Still running: CS 110's syllabus**, then **Show my week**, above the progress grid. The status line reads "Read 8 of 9 · 3:00 of 3:00" | Spec: wait up to 3 minutes, **then name what is still running** |
| `A` and `P` listed "$2.49 with a Pell Grant" and "$39.99 for a 6-month term" | Removed. Neither price is in the ruled spec | Invented prices shown as fact |
| `Q` opened mid-summary and said "the sign-in you just made" | `Q` opens on a clean exchange: "Welcome back. Your school and payment are done; about 4 minutes of yours are left, then your week." | Resume should not open on clipped history |
| Small things | The checkbox label's stray space is fixed. The dark-mode wordmark has no white tile: its ink inverts and its red keeps its hue. The CHEM 231 source line is now "exams are 60% of the grade" | Craft floor |

**Flagged for Quinn, not changed.** The spec's step 4 is ruled ("$9.99 a month, after 7 days free",
"Your trial is on"), so the build keeps the trial. It does sit next to the cloud design's "no free
tier". `A` now says it plainly: "The first 7 days are a trial; after them there's no free plan." If
the trial is not meant to exist, `A`, `P`, `P-out`, `E` and the rail's "first charge Oct 25" change
together.

**Exception to "onboarding has no text box".** The homework-site sheet (`H-vhl-sheet`,
`H-zybooks-sheet`) has an email field and a password field, because those sites have no school
sign-in Knowlu can use. It is drawn differently on purpose: a dashed edge, a key glyph, and "Knowlu
keeps this password".

---

## Direction contract

- **THESIS.** Onboarding is a session run on the student's own data: one exchange per ask, proof
  in numbers, each daily gesture taught once on a real item. Today is a brief and a list: one
  paragraph, then the list pinned in place, with Knowlu's notes in the margin.
- **OWN-WORLD.** A returned paper with careful margin notes. Cool white paper (never cream) and
  blue-black ink. Knowlu writes in Literata, set as prose. The student's things (chips, rows,
  times) are set in Mona Sans with tabular figures. Notes connect to what they are about with a
  thin pen-blue leader line. Done is a tick; needs you is a proofreader's insertion caret in amber;
  red lives only in the wordmark.
- **STORY.** The student watches Knowlu read their school and say what it found, in their own course
  codes. They believe it because it names their own syllabus rules. They finish having pressed
  chips, signed in and confirmed, never having typed an answer.
- **FIRST VIEWPORT (onboarding).** 1280x800. A 44 px top bar holds the wordmark. A 38rem reading
  column starts at x 96: the thread, with the open question resting at about 40% of the window
  height. A 17rem rail is docked at the right edge behind a hairline.
- **FIRST VIEWPORT (Today).** *(changed, §0)* A 45rem main column: the day line, the class line and
  the list, with row 1 in the upper half. The margin holds Knowlu's letter (at most four sentences,
  each region a footnote link), the focused row's note on a leader line, and the assistant line at
  its foot.
- **FORM.** Direction 7 in `../concepts.md` (`concierge`), brought in as 1b. Bold. Code-led; no
  comps.
- **FINISH.** unreviewed and undocumented is unfinished; this build ends with the finish review,
  the verdict, DESIGN.md, and every shipping raster carrying its provenance.

---

## 1. Flow map

### 1.1 The frame shared by every onboarding screen

- **Top bar (44 px).** The wordmark on the left. Nothing else; the rail carries progress and the
  exit, so the top stays quiet.
- **The thread (38rem column).** Exchanges stack downward. The open question is scrolled to rest at
  about 40% of the window height, so the student's eye returns to the same place every time.
  Earlier exchanges stay above it, in full contrast (secondary ink, still at least 4.5:1); scrolling
  up re-reads them. Nothing in the thread can be typed into.
- **The rail (17rem, docked right).** It shows the sources in the ruled order, each with one of
  six states as a word plus a shape (§1.6), the student's time left at the top ("About 7 minutes
  of yours left"), and **Finish later** at the foot. Clicking a done source scrolls the thread to the
  reply that proved it. The rail never lets the student jump ahead: the order is ruled.
- **The sign-in sheet.** The one enclosed shape in onboarding. It holds a credential moment only:
  the school's window (opened by Knowlu, the host shown), a homework site's two fields, the emailed
  code. It slides up from the bottom of the thread column, 32rem wide, on a raised paper ground
  with a 1 px pen-blue edge, a small authored lock glyph and one plain privacy sentence. No other
  element in the product looks like it, so it keeps its meaning (Baymard, research/onboarding-evidence.md
  §3.4).

### 1.2 The anatomy of one exchange

Every ask is the same four beats. Its id names the exchange; a letter suffix names a state of it.

1. **Why** (Knowlu, Literata). One line before anything is asked. It names the benefit in the
   student's own terms as soon as Knowlu knows them (from `S` onward: "Halverson State", course codes).
2. **What it takes** (Mona Sans, small, only on credential asks). Plaid's expectation line: "You'll
   need your Halverson State email and password, and your phone for Duo."
3. **The action.** One primary chip (filled blue-black ink), at most one secondary chip (outlined),
   and, on a skippable ask, a **Not now** text chip with its cost written directly under it. Chips
   answer to `Enter` (primary) and `1` to `3` in order, shown on focus only.
4. **The reply** (Knowlu, Literata), then, when there are counts, a short **proof block**: one count
   per line in Mona Sans with tabular figures and a tick, each line landing as its read finishes.

When the student answers, the chips they did not press fold away and the pressed one becomes a short
**answer line** set flush to the column's right edge in Mona Sans ("Continue with Google", with a
tick). That is the student's side of the exchange: not a bubble, a pencilled answer in the margin of
Knowlu's question.

A read that finishes after the student has moved on arrives as a **follow-up**: a short Knowlu reply
inserted directly above the open question, headed by the source ("Halverson State, later"), never
below it, so the open question never moves out from under the student's eye. The rail updates at
the same moment.

### 1.3 Exchanges in order

Ids are stable; the prototype's state jumper and copy table use them.

| Id | Exchange | Shows | Leaves when | Goes to |
|---|---|---|---|---|
| `O0` | Opening | Knowlu's opening line; **Get started** | chip | `A` |
| `A` | Your account | why line; the price disclosed; **Continue with Google**; secondary **Email me a code**; 18+ and terms as one checkbox above the chips | Google returns / code chosen | `A-ok` / `A-code` |
| `A-code` | Emailed code | sign-in sheet: "I sent a 6-digit code to …"; six code boxes (typing a code is a sign-in) | code accepted | `A-ok` |
| `A-ok` | Account proof | "Signed in as maya.okafor.demo@gmail.example." The plain-files line with **Show me the folder** | 1.2 s, or folder link closed | `S` |
| `S` | Your school | why line; "Which school?" with one chip per curated school and **Another school** | school chip | `S-need` |
| `S-other` | School not curated | "I can't sign in to that school yet. I can still read your Blackboard calendar link." | chip | `S-feed` |
| `S-need` | Before the window | expectation line; sign-in sheet note; **Sign in at Halverson State** | chip | `S-win` |
| `S-win` | School window open | holding line: "Halverson State's sign-in is open in its own window, at login.halverson.example." Duo line beneath. No Finish later while it is open | session arrives / window closed / Duo page for 45 s | `S-proof` / `S-closed` / `S-duo` |
| `S-duo` | Waiting for Duo | the Duo follow-up line (copy in §2.3) | session / closed | `S-proof` / `S-closed` |
| `S-closed` | Window closed early | cause and fix; **Open Halverson State sign-in**; from the second close on, also **Trouble signing in?** | chip | `S-win` / `S-help` |
| `S-help` | Trouble signing in | three chips: **Try again** · **Use the calendar link instead** (cost under it) · **Finish later** (says setup waits for the school) | chip | `S-win` / `S-feed` / `X` |
| `S-feed` | Calendar link fallback | three short steps to copy Blackboard's calendar link; **Paste link** (reads the clipboard; pasting is not typing); cost line | valid link | `S-proof` (fewer lines) |
| `S-proof` | Your school: proof | the reply and the proof block (§2.1), landing line by line | all counted: classes, times, assignments | `S-undo` |
| ~~`S-undo`~~ | *Removed (§0).* First gesture: Undo | "These 4 look turned in. I've archived them so they won't show as tasks. Undo is here if I'm wrong." **Looks right** · **Show me the 4** | chip | `P` (or the 4 open in place, each with **Undo**) |
| `P` | Payment | why line; the order summary built from `S-proof`; **Continue to checkout** | chip | `P-out` |
| `P-tester` | Payment, tester | the same summary; "Nothing to pay: you're a tester." **Continue** | chip | `E` |
| `P-out` | Checkout open | "Checkout is open in your browser on Stripe's page. I'll move on by myself when Stripe confirms, usually under a minute." **Open checkout again** | Stripe confirms / 10 min | `P-ok` / `P-stale` |
| `P-stale` | Not confirmed | "Stripe hasn't confirmed yet. If you finished, it can take a minute. If you closed the page, open checkout again." | confirm / chip | `P-ok` / `P-out` |
| `P-ok` | Payment proof | "Your trial is on. First charge October 25." | 1.2 s | `E` |
| `E` | School email | covered: "Nothing new to do here: your Halverson State sign-in covers your school email." **Continue**. Not covered: a sign-in sheet and **Not now** with its cost | chip | `G` |
| `G` | Gmail and Calendar | why line; what Google will show; read-only line; **Continue to Google**; **Not now** with its cost | chip | `G-win` / `H-zybooks` |
| `G-win` | Google window open | "Google's window is open. Google will warn first that it hasn't finished reviewing Knowlu: choose Advanced, then Go to Knowlu." | consent / declined / closed | `G-proof` / `G-fwd` / `G` (with the closed line) |
| `G-proof` | Google proof, and a second Undo | Gmail and Calendar reported separately (§2.1); the dues item added from mail before setup, with **Undo** | chip | `H-zybooks` |
| `G-part` | Partial grant | "Gmail is connected. Calendar wasn't shared, so your free time won't count your own events." **Share calendar** · **Add another calendar** · **Not now** | chip | `G-win` / `C` / `H-zybooks` |
| `G-fwd` | Gmail by forwarding | only on decline or a failed read; 3 steps; **I've set it up** waits for the test message; **Not now** | test arrives / chip | `G-proof` / next |
| `C` | Another calendar | only when no calendar was granted; **Paste calendar link**; **Not now** with its cost | link / chip | `H-zybooks` |
| `H-zybooks`, `H-vhl` | Homework sites, one exchange per site the syllabi mention | why line naming the course; **Sign in to zyBooks** opens the sign-in sheet (email, password); **Not now** with its cost | signed in / refused / chip | proof, then next site or `Y-cs110` / `H-*-fail` |
| `H-*-fail` | Site refused | cause and fix, sheet reopened with the email kept and the password cleared | retry / chip | `H-*` / next |
| `Y-cs110` | A missing syllabus, one per course | why line naming the course; **Choose the file** (Windows' own file dialog; a file is not typing); **Not now** with its cost | file / chip | `Y-cs110-read` / `K1` |
| `Y-cs110-read` | Reading it | "Reading CS 110's syllabus. Usually under a minute; you can keep going." The proof arrives later as a follow-up | immediately | `K1` |
| `K1` | Second gesture: Confirm, one row | "BIO 201 lab is Thursday 2:00-4:50 pm in Kessler Hall 310, per the registrar. Right?" **Right** · **Change it** | chip | `K2` |
| `K2` | The other meetings | "The registrar has 6 more meetings for your 5 classes." A 6-row timetable block; **All right** · **Change one** | chip | `K3` |
| `K3` | From your syllabi | exams (10), grade weights (5) and late rules (4) as three tight blocks; ENG 210's blank rule as its own line with **Leave it blank** · **Add one** | chip | `K4` |
| `K4` | Your own week | one line per repeating calendar event, each with level chips **Firm** · **Flexible** · **Optional** · **Not mine**; office hours as optional; "Your day: 7:30 am to 11:30 pm" with **Change** | each row answered, then **This is my week** | `W` |
| `W` | While I finish | the wait: true follow-ups narrating real work; the third gesture offered | everything ran once, or 3:00 | `R` |
| ~~`W-pin`~~ | *Removed (§0).* Third gesture: Pin | "Lab 4 report is the biggest thing this week: about 3h, due Thursday at 2:00 pm. It isn't first, because smaller things are due sooner. To keep it on top until it's done, pin it." **Pin it** · **Keep the order** | chip | stays in `W` |
| `R` | Here's your week (the first brief) | §3 | **Open Knowlu** | `T` |
| `X` | Finish later | Knowlu's leaving exchange (§1.5) | chip | quit / back |
| `Q` | Resume | the thread restored, the open question re-asked with one line above it (§1.4) | as that exchange | as that exchange |

`K1` to `K4` are one stop on the rail ("Your week · 2 of 4"). Each homework site and each missing
syllabus counts inside its stop ("Homework · 1 of 2").

**Why the gestures sit where they do.** Undo comes at `S-undo` because turned-in work is the first
thing Knowlu decides on the student's behalf, and archiving it at first sight is ruled (spec §6.3);
the student sees the decision, its count and the way back at the moment it is made. Undo is shown a
second time at `G-proof` on mail from before setup (VISION's setup exception). Confirm comes at
`K1` on a single row before the batch, so the student learns what "Right?" means on one lab before
being asked about six meetings at once. Pin comes during the wait at `W-pin`, when the list exists
and there is time to spare. Each gesture is offered once; using it is the student's choice. A run
in which nothing needed undoing is a correct run, and Knowlu never invents an item to practise on.

### 1.4 Resume after quit (`Q`)

- State lives in `onboarding.json` in the profile's app data, never in the vault (spec §1.6): the
  open exchange id, each answer, and which background reads finished. The thread is re-rendered
  from that file, so a relaunch shows the same exchanges with every finished proof intact.
- One line from Knowlu is added above the re-asked question, and only on the first view after a
  relaunch. It picks up the open loop by name (Ovsiankina, research/onboarding-evidence.md §1.5):
  > Welcome back. Your school and payment are done. Google is next; about 5 minutes left.
- Reads that had not finished restart and show "reading…" on the rail again.
- **Quit at payment:** the entitlement is re-checked first (spec §1.6); the thread reopens at `P`.
- **School session expired while away:** Knowlu recognises the kept session is gone and re-opens
  the sign-in itself, never an error page (Link's recognise, verify, fill; research/stripe.md §3.2):
  > Halverson State signed you out while you were away. Sign in once more; nothing you connected is
  > lost. **Sign in at Halverson State**
- **Quit with a window open** (school, Google, checkout): the thread reopens on the ask before it
  (`S-closed`, `G`, `P-out`), never on a dead waiting line.

### 1.5 Finish later (`X`)

**Finish later** sits at the foot of the rail on every exchange except while a school, Google or
checkout window is open. Pressing it adds one exchange to the thread rather than opening a dialog:

> Leaving now is fine. I'll keep what you've connected and open right here next time. I won't check
> your work until setup is done: 4 steps left, about 5 minutes.
>
> **Quit for now** · **Keep going**

### 1.6 The rail: sources and their states

The rail lists, in the ruled order: Account · Halverson State (with five sub-lines: classes, class
times, assignments, grades, syllabi) · Payment · School email · Gmail · Calendar · zyBooks · VHL ·
Syllabi · Your week. A source the student will not meet (Another calendar, for Maya) is not listed.
Each line carries one state, always as a word and a shape, never by colour alone (Stripe's
requirement states, research/stripe.md §1.2):

| State | Shape (authored SVG) | Word | Under the line |
|---|---|---|---|
| now | a short pen-blue underline beneath the label | "Now" | none |
| not yet | a hairline circle | none | none |
| reading | three dots that fill one by one (static with reduced motion) | "reading…" | its expected time: "usually under a minute" |
| done | a tick | the proof, short | "5 classes · 26 assignments" |
| needs you | the insertion caret, amber | the fix | "VHL: sign in again" |
| skipped | a dash | "Not now" | the cost, short: "zyBooks due dates missing" |

The time left at the top is the sum of the spec's median student time for the exchanges still ahead
(spec §1.3), rounded to the minute, never below "About 1 minute". It counts the student's time only.

### 1.7 Skip paths and their named costs

A skip is the **Not now** text chip (BRAND-VOICE's word), set 16 px right of the last chip. Its cost is
written directly beneath the chips in Mona Sans at 0.875rem, before the student presses anything, so
the cost is read, not discovered. After a skip, Knowlu replies with one line ("Skipped. Connect
Google any time in Settings.") and the rail shows the dash. Every skip is named again at the reveal
and listed in Settings with **Connect**. A skipped source is never asked about again in a nudge (spec
§1.7).

| Exchange | Cost line, shown under the chips |
|---|---|
| `E` (when not covered) | "Without it, date changes and new work your professors announce by email are missed." |
| `G` | "Without it, club, job and internship tasks from Gmail are missed, and your free time ignores your own events." |
| `C` | "Without it, your free time ignores events on that calendar." |
| `H-zybooks` | "Without it, CS 110's zyBooks due dates are missing unless Blackboard's calendar carries them." |
| `H-vhl` | "Without it, SPAN 202's VHL due dates are missing unless Blackboard's calendar carries them." |
| `Y-cs110` | "Without it, CS 110 has no exam dates beyond Blackboard's calendar, no grade weights and no late rule." |
| `K3`, `K4` rows | "Leave it blank" per row: "Blank classes come back as one question each over the next few days." |

Account, school, payment and Your week have no **Not now**. The school's only way round is `S-feed`,
whose cost line reads: "With the link only, I can't see grades, class times from the registrar, your
syllabi or what you've turned in, and school email needs its own sign-in."

### 1.8 Failure and waiting, in the thread

Each is a Knowlu line with the cause and the fix and, where support could need it, a small code
under it in Mona Sans (research/stripe.md §1.7). None of it is red, nothing shakes, and nothing blames
the student (research/onboarding-evidence.md §4.1).

| Where | Knowlu's line |
|---|---|
| `S-need` (always) | "You'll need your Halverson State email and password, and your phone for Duo." |
| `S-win` (always, under the holding line) | "Halverson State will ask your phone to approve. Keep it nearby." |
| `S-duo` (window on Duo's page for 45 s) | "Waiting for your phone. If the request timed out, ask for a new push in Halverson State's window." |
| `S-closed` | "The Halverson State window closed before sign-in finished. Open it again when you're ready." |
| `S-help` | "Trouble signing in? You can try again, use your Blackboard calendar link instead, or finish later. Setup continues once your school is connected." |
| `H-vhl-fail` | "VHL didn't accept that email and password. Try again, or set up VHL later; its due dates will be missing until it's connected." |
| `G-fwd` lead-in | "Gmail's read didn't go through. I can bring Gmail in by forwarding instead, about 3 minutes." |
| `G-win` closed | "Google's window closed before you chose. Continue to Google when you're ready." |
| Follow-up, read past twice its expected time | "Halverson State's grades are taking longer than usual. You can keep going; I'll say here when they're in." |
| `E` test message slow | "Waiting for the test message. Usually 1-3 minutes; you can keep going." |

A failed login is never seen by Knowlu as a wrong password: the school's own page handles that.
Knowlu sees only a window that closed or a session that never arrived, and says exactly that.

### 1.9 A slow first run

- **During the exchanges.** Every "reading…" carries its expected time (research/stripe.md §1.8):
  "usually under a minute" for the course list, assignments and grades; "about a minute each" for
  syllabi; "usually 1-3 minutes" for the school email test; "usually under a minute" for Gmail and
  Calendar. Proof that lands late arrives as a follow-up above the open question.
- **The wait (`W`).** It starts when `K4` is answered and is capped at 3 minutes (spec §1.5). It is
  not a loading screen: it is the thread continuing. A countable line heads it, "8 of 10 read", and
  updates in place (NN/g progress indicators, research/onboarding-evidence.md §3.9). Below it, each
  finishing read writes one true follow-up (labor illusion, Buell & Norton 2011, §1.4), for Maya:
  > Reading CS 110's syllabus: found the late rule, −10% a day, up to 5 days.
  >
  > Checked 40 assignments against what you've turned in: 9 already turned in, archived.
  >
  > Estimated effort for 34 assignments. Readings are about 30 minutes; lab reports about 3 hours.

  Then `W-pin`. At 3:00 the brief opens and names what is still running. Maya's opened after 40 s,
  when CS 110's syllabus finished.
- **A source that fails** is named with its fix in the thread at once, on the rail, and again in the
  brief. It never stays a silent "reading…".

---

## 2. Onboarding copy, exchange by exchange

These follow BRAND-VOICE: sentence case, "you", contractions, numerals (spelled out at the start of a
sentence), lowercase am and pm, no exclamation marks, and none of "just", "simply", "easy", "don't
worry", "oops". The why lines keep the spec's wording, which Quinn reads as final copy; where this
direction adds the student's own names to one, the addition is marked *(added)*. Proof lines come from
`sample-student.json`. Knowlu says "I" inside the thread and the assistant only; rows, the rail and
status lines never do (an open question in BRAND-VOICE; see §7).

Privacy lines appear only where a secret or a permission changes hands, inside the sign-in sheet,
as one specific sentence (research/onboarding-evidence.md §4.3: over-reassurance reads as nervous).

| Id | Why line (before the ask) | Chips | What it takes / privacy | Proof (after) |
|---|---|---|---|---|
| `O0` | "Knowlu answers one question every morning: what's next? Next, you'll connect the places that already know your work, about 10 minutes." | **Get started** | none | none |
| `A` | "Your plan lives in your account, so it's safe if this computer isn't." | **Continue with Google** · Email me a code | Price, before any school login (NN/g up-front disclosure, research/onboarding-evidence.md §3.5): "Knowlu is $9.99 a month after 7 days free. There's no free plan. You'll see what I find at your school before you pay." Sheet: "Google shares your name and email, nothing else." | "Signed in as maya.okafor.demo@gmail.example. Your plan is kept as plain files on this computer." **Show me the folder** |
| `S` | "One school sign-in lets Knowlu read your classes, class times, assignments, grades and school email. You won't sign in to these again." Then: "Which school?" | **Halverson State University** · Another school | none yet | none |
| `S-need` | none (continues `S`) | **Sign in at Halverson State** | "You'll need your Halverson State email and password, and your phone for Duo." Sheet: "You sign in on Halverson State's own page. Knowlu never sees or keeps your password; it keeps the signed-in session on this computer." | none |
| `S-proof` | none | none until counted | none | "You're in. Here's what Halverson State has for you this term:" then the proof block (§2.1) |
| `S-undo` | "These 4 look turned in. I've archived them so they won't show as tasks. Undo is here if I'm wrong." | **Looks right** · Show me the 4 | none | Opened: four rows (title, course, "Blackboard shows it turned in", **Undo**). After **Looks right**: none |
| `P` | "Everything you just connected keeps running every day for $9.99 a month, after 7 days free." | **Continue to checkout** | The order summary above the chip (§2.2). Sheet: "Checkout opens in your browser on Stripe's page. Knowlu never sees your card." Then: "You pay us, so we don't sell your data." | "Your trial is on. First charge October 25." |
| `E` | "Professors change dates by email. Knowlu reads what arrives and proposes each change for you to approve." | **Continue** (covered) | "Nothing new to do here: your Halverson State sign-in covers your school email." | Follow-up: "School email is on. A test message arrived in 38 seconds. New mail is read as it arrives; older school mail is still coming in, which can take a day." |
| `G` | "Clubs, jobs and internships write to your Gmail, and your calendar says when you're already busy. One Google permission covers both, read-only." | **Continue to Google** · Not now | Sheet: "Read-only: Knowlu can't send, delete or change anything in Gmail or your calendar. Gmail brings in the last 7 days of mail." Then: "Google will warn first that it hasn't finished reviewing Knowlu. Choose Advanced, then Go to Knowlu." | §2.1 (Gmail and Calendar each on their own line) |
| `G-proof` second Undo | none | **Looks right** · Undo | none | "One email from before today looks like something to do, from Pre-Health Society: dues, $15, due Friday, October 30 at 5:00 pm. I added it. Undo is here." |
| `C` | "If your calendar isn't Google's, add it so your free time is honest." | **Paste calendar link** · Not now | "I only read it." | "9 events this week." (variant; not shown for Maya) |
| `H-zybooks` | "Homework sites hold due dates your school's feed doesn't show. Your CS 110 syllabus uses zyBooks." *(added: the course)* | **Sign in to zyBooks** · Not now | Sheet: "Your zyBooks password is stored only on this computer, in Windows Credential Manager. It never goes to Knowlu's servers." | "zyBooks: 1 book, 8 assignments, 3 already done. The 3 are archived like the others." |
| `H-vhl` | "Homework sites hold due dates your school's feed doesn't show. Your SPAN 202 syllabus uses VHL." *(added)* | **Sign in to VHL** · Not now | Same sheet, naming VHL | "VHL: 1 course, 6 assignments, 2 already done, archived." |
| `Y-cs110` | "Your syllabus has every exam date, how each course is graded, and the late-work rules. CS 110's wasn't in Blackboard." *(added: the course)* | **Choose the file** · Not now | "The file is read once to find dates and rules. It stays in your plan folder on this computer." | Follow-up: "CS 110's syllabus is read. That's 5 of 5: 10 exams, weights for 5 courses, late rules for 4. ENG 210's syllabus has no late rule; I'll ask about it in a moment." |
| `K1` | "Check your classes. Everything here can be changed later." Then the single row: "BIO 201 lab is Thursday 2:00-4:50 pm in Kessler Hall 310, per the registrar. Right?" | **Right** · Change it | none | none (the row takes a tick) |
| `K2` | "The registrar has 6 more meetings for your 5 classes." | **All right** · Change one | none | "7 meetings for 5 classes, confirmed." |
| `K3` | "From your syllabi: exams, grade weights and late rules, as they're written." Then: "ENG 210's syllabus has no late rule. Leave it blank?" | **All right** · Change one; for ENG 210: **Leave it blank** · Add one | none | "10 exams, weights for 5 courses, late rules for 4, confirmed. ENG 210 has none, which is fine." |
| `K4` | "Your calendar has 3 things that repeat. How firm is each one?" Per row: "Marsh Library circulation desk, Tuesday 5:00-9:00 pm and Saturday 10:00 am-2:00 pm." | per row: **Firm** · Flexible · Optional · Not mine | Under the chips, once: "Firm: nothing is planned over it. Flexible: it can be overlapped, and I'll say so. Optional: it never counts as busy." | "3 of your own, 2 office hours (optional), your day 7:30 am to 11:30 pm." **This is my week** |
| `W-pin` | "Lab 4 report is the biggest thing this week: about 3h, due Thursday at 2:00 pm. It isn't first, because smaller things are due sooner. To keep it on top until it's done, pin it." | **Pin it** · Keep the order | none | Pinned: "Pinned. It stays on top until it's done; unpin it any time." Kept: "Kept. You can pin anything from the list later." |

Maya's answers in `K4`: the library job **Firm**; Pre-Health Society **Flexible**; intramural soccer
**Optional**. Office hours come from syllabi and are set optional by default, shown as one line with
**Change**.

**Row format inside `K2` to `K4`.** Left to right: the label, the days and times in tabular figures,
the room, the source in quiet text, then **Change**. For example:
`CHEM 231 recitation   Wed 3:00-3:50 pm   Hallett Hall 015   registrar   Change`. Typing appears only
inside **Change**, as the last resort (spec §4; research/stripe.md §1.5).

### 2.1 Proof blocks

Each line lands with a tick as its own read finishes (§5.4). A line still running reads "reading…"
with its expected time.

**`S-proof`, Halverson State:**

```
✓ 5 classes this term          2 from last term left out
✓ Class times for all 5        7 meetings a week, from the registrar
✓ 26 assignments               4 already turned in (next: what I did with them)
✓ Grades for 5 courses
✓ Syllabi for 4 of 5           CS 110's isn't in Blackboard; I'll ask for it later
```

The ticks are the authored tick glyph, not a character. The quiet second column is Markdoc's rule
(research/stripe.md §4.4): every explanation names the student's own course and source.

**`G-proof`, Google:**

```
✓ Gmail      40 messages from the last 7 days; 1 is something to do
✓ Calendar   6 events this week; 3 repeat, and you'll say how firm they are in a few minutes
```

The Gmail line is Notion Mail's proof drawn from your own inbox (research/notion.md §6): a count, who
it came from, and what was done. The 39 messages that could not affect the plan are not listed; on
request ("What about the other 39?") Knowlu answers in one line: "They couldn't change your plan, so
I didn't keep them."

### 2.2 The payment order summary (`P`)

Stripe Checkout's order summary, where the summary is the school proof (research/stripe.md §3.1). It
sits inside Knowlu's reply as a ruled block, Mona Sans, tabular figures:

```
Halverson State · Fall 2026
  5 classes, 7 meetings a week
  26 assignments (4 turned in, archived)
  Grades for 5 courses
  Syllabi for 4 of 5
-------------------------------------------
Knowlu                          $9.99 a month
7 days free; first charge October 25
Cancel any time in Settings. Cancelling before October 25 costs nothing.
Other plans: $39.99 for a 6-month term · Pell Grant recipients pay 75% less
```

A line still being read shows "reading…" in place. **Continue to checkout** is enabled once classes and
assignments are counted.

---

## 3. "Here's your week": the first brief (`R`)

### 3.1 What happens on screen

The reveal is not a new screen. It is the last page of the same paper, and it turns the onboarding
layout into the main page's layout in one move, the direction's one authored moment:

1. A hairline rule is drawn across the thread under the last exchange, and the view scrolls so the
   brief's heading sits at the top of the window (400 ms, exponential ease-out).
2. **The rail becomes the margin.** The rail's column widens from 17rem to 28rem and its done lines
   rewrite themselves as Knowlu's margin notes (a 240 ms crossfade per line, top to bottom). What
   was proof during setup is now commentary beside the week. The student sees that the margin is
   where Knowlu writes from now on.
3. Each margin note draws its leader line to what it is about (a day, a row), a 300 ms stroke.

With reduced motion, steps 1 to 3 are one 150 ms crossfade and the leader lines are already drawn.
The scheduler and "start with Windows" turn on here and not before (spec ruling 12).

### 3.2 Layout at 1280x800, as of Sunday, October 18, 7:19 pm

**Main column (45rem):**

1. **Heading.** "Here's your week." (Literata 2rem), with "Monday, October 19 to Sunday, October 25"
   beneath in Mona Sans.
2. **The brief** (Literata 1.375rem, four sentences):
   > Reading: Ch. 8, Metabolism is next, due tomorrow at 9:00 am, about 30 minutes. It's the first
   > thing due this week. Thursday and Friday carry the most, about 4h each. Tonight has about 3h
   > free, after 9:00 pm.
3. **The week**, as a ruled table of seven rows (one per day), not a calendar grid: day · classes
   and commitments as small tinted chips with their times · what's due, with time and estimate. Thursday
   reads: `Thu  CHEM 231 9:30 · ENG 210 12:30 · BIO 201 lab 2:00-4:50 · Haircut 5:00  |  Lab 4
   report, 2:00 pm, 3h · Literature review outline, 12:30 pm, 1h`. Optional items are outlined chips
   with the word "optional" on focus. Each row opens its own small editor and returns here (Stripe's
   summary with per-section edit, research/stripe.md §1.6). A table fits seven days at 1280x800
   without a grid's empty hours, and reads top to bottom like the brief.
4. **The list begins**: "Tonight and tomorrow", two rows: Reading: Ch. 8, Metabolism · BIO 201 ·
   tomorrow at 9:00 am · 30m; VHL Lección 5: Vocabulario · SPAN 202 · tomorrow at 11:59 pm · 1h. In
   the pinned variant (`R-pin`), Lab 4 report sits first with the pin glyph and the word "Pinned".
5. **One filled button: Open Knowlu.**

**Margin column (28rem), Knowlu's notes, each one short paragraph:**

- Beside Thursday: "Lab 4 report is due at 2:00 pm, before your lab starts. About 3 hours."
- **Tidied**: "52 past items and 9 already turned in were archived. Effort is estimated for 34
  assignments. 2 classes from last term are left out." One sentence each, no lists (spec §1.8).
- **From your email**: "Pay Pre-Health Society dues ($15), due Friday, October 30 at 5:00 pm. From
  an email before today; I added it." **Undo**
- **Still coming**: "Older school email is still coming in. Anything it finds will come to you as a
  question." Skipped-sources variant (`R-skip`): "Not connected: zyBooks and Google Calendar. Without
  them, CS 110's zyBooks dates are missing and your free time ignores your own events. Connect them
  in Settings." Capped-wait variant (`R-cap`): "Still reading 1 syllabus; it will appear on your page."
- **What happens now**: "I'll check again at 12:00 pm and 6:00 pm, and Knowlu starts with Windows.
  Change this in Settings." This hands off to habit (research/onboarding-evidence.md §3.8).

The page then shows exactly what the brief promised: `T` is built from the same read model.

---

## 4. The main page (`T`): a brief, then the list

### 4.1 Layout at 1280x800

- **Top bar (44 px).** Wordmark left. Right: the status line, calm by default ("Up to date, 6:04
  am"), then Settings. When a source needs the student, the status line grows to say so with the
  amber caret ("VHL signed you out. **Sign in to VHL**"); Stripe's banner that exists only when
  action is needed, kept as a calm positive line when healthy (research/stripe.md §2.1).
- **Main column (45rem, from x 32).** It does not scroll with the page: the brief, the day line, the
  class line and the list sit in a column that stays put, and the list scrolls inside its own block
  only when it is longer than the window. So the list never scrolls away, whatever the margin is
  doing.
- **Margin column (28rem, right, behind a hairline).** Knowlu's notes, scrolling independently, with
  the assistant line docked at its foot.
- **Under 1100 px wide** the margin moves below the list, and the list becomes a sticky block under
  the top bar (condensed to 36 px rows) while the notes scroll beneath it.

### 4.2 Regions, top to bottom, with their read-model source

Drawn from `sample-student.json` → `today` at Monday, October 26, 8:15 am. Field names follow
`docs/surface/anatomy.md`.

| # | Where | Region | Read-model source | Shown for Maya |
|---|---|---|---|---|
| 1 | main | **The brief** | `list[0]` + `why`/`late_label`, `verdict.headline`, `free_time`, `delta`, `decisions` header | "Lab 6: Functions is next. It was due Friday; finishing it today keeps it at −30%, and its late window closes Wednesday at 11:59 pm. About 4.7h is due by tomorrow and today has room for it; about 5.7h is yours after 6:00 pm. Since 6:00 pm yesterday, your Ch. 7 pre-lecture quiz was marked done because Blackboard shows it turned in, and VHL signed you out. Four questions are waiting, the oldest for 7 days." |
| 2 | main | Day line | `the_day` / `schedule`, `free_time` | One 28 px strip from 7:30 am to 11:30 pm: BIO 201 9:00, SPAN 202 10:00, CS 110 1:00, CHEM 231 office hours 3:30 (outlined, optional); a "now" tick at 8:15; free time drawn as clear space labelled "about 5.7h free after 6:00 pm" |
| 3 | main | Class line | `domains`, `grades` | One line of quiet chips, word plus shape: "Organic Chemistry · about 2h behind" (down-step glyph), "Programming · about 1h behind", "Spanish · about a day ahead" (up-step), "Cell Biology · on track", "Science Writing · on track", then Library job, Pre-Health Society, Personal. Each course chip ends in a small grade ring (87.8% B+ for BIO 201), hideable in Settings. Clicking a chip filters the list; clicking again clears it |
| 4 | main | **The list** | `must_do` / `recommended` as ranked rows, `done_today` | Header: "Today · about 6.5h" with 13 thirty-minute ticks (the meter, `meter`; never red). Row 1 larger (Mona Sans 1.25rem): **Lab 6: Functions** · CS 110 · zyBooks 40% · "was due Friday at 11:59 pm" · "−30% now; closes Wednesday at 11:59 pm" · about 1h 12m left · **Start** · **Open in zyBooks** · done circle. Then tight rows: zyBooks 7.1-7.4 participation · today at 11:59 pm · 24m; VHL Lección 6: Práctica de vocabulario · today at 11:59 pm · 36m; Problem set 7 · tomorrow at 9:30 am · 1h 30m; Peer review: outline exchange · tomorrow at 12:30 pm · 1h; Study for Exam 2 (Ch. 5-7) · Thursday at 9:30 am · 1h 48m today, of 4h. Done today: Reading: Ch. 9, Cellular respiration, struck through, can be unchecked |
| 5 | main | The fold | `free_time`, `later` | A hairline: "Today's free time ends here." Below it, three `later` rows in secondary ink (still at least 4.5:1), then "21 more this term · about 42h · See all" |
| 6 | margin | **The note for the focused row** | the row's `why`, `source`, the syllabus late rule | Default, beside Lab 6: "Lab 6 is first because it's already past due, and your CS 110 syllabus takes 10% a day for up to 5 days. The two due tonight are shorter and fit after it." A leader line runs to the row |
| 7 | margin | **Asks** (the deck) | `decisions` | Header: "4 questions · oldest 7 days". One at a time, oldest first by `first_proposed_at`, so the page never grows with the queue (anatomy §3.9). Card: Knowlu asks in Literata: "Add 'SURF application' to Personal, due Friday, November 6 at 5:00 pm, about 2h?" Under it the quoted email, plain text, never rendered as instructions: "Office of Undergraduate Research · October 19 · 'Applications for the Summer Undergraduate Research Fellowship are due Friday, November 6 at 5:00 pm. You'll need a one-page statement and a faculty contact.'" Age: "Waiting 7 days." **Add it** · **Not for me** · Not now. "3 more after this one" |
| 8 | margin | What changed | `delta` | "Since 6:00 pm yesterday: Pre-lecture quiz: Ch. 7 marked done (Blackboard shows it turned in) · **Undo** · BIO 201 Lab 4 report graded 44/50 · 1 new question · VHL needs you to sign in again." |
| 9 | margin | Late window closed | `late_window_closed` | "Problem set 6 (CHEM 231) left your list. Your syllabus doesn't take late work after 48 hours, and that passed Thursday at 9:30 am." **Archive** · **Still doing it** (which asks "When will you turn it in?"). VHL Lección 5: Vocabulario the same way, with SPAN 202's rule |
| 10 | margin | Coming up | `upcoming_exams`, `ahead.takeaway`, `coming_up` | "CHEM 231 Exam 2 is Thursday at 9:30 am, in Hallett Hall 120, moved by email. Friday is the heaviest day in the next two weeks, about 10.2h due." Opens the week |
| 11 | margin | Good to know | `good_to_know` | "BIO 201 Lab 4 report graded: 44/50." · "CHEM 231 Problem set 6 is marked missing in Blackboard." Each with its source on request |
| 12 | margin | Closed this week | `closed_this_week` | One line: "Closed this week: 8. I marked 6 done because the site shows them turned in. **Review or undo**" opens the 7-day history, marked you, Knowlu or lapsed |
| 13 | margin foot | **The assistant line** | the assistant (VISION) | A one-line field: "Add something, or ask why". `/` focuses it. Replies are margin notes (§4.3) |

**Empty regions.** Every region renders, as anatomy §4.4 requires, but an empty one is a single
quiet line in the margin, never a hidden hole and never a box: "Nothing waiting on you." · "Nothing
past its late window." On a free day the brief carries it: "Nothing is due today. The day is yours.
Next up tomorrow: …" (`all_clear_example`).

**How the brief is written.** It is assembled from read-model fields by fixed templates in a fixed
order (next and why; the verdict; what changed, by actor; the questions header), at most four
sentences. It is never model-written, so it can never disagree with the list beneath it (anatomy
§3.3: "never a number that disagrees with the meter"). Each phrase is a link to its region: "VHL
signed you out" focuses the status line's sign-in, "Four questions" focuses Asks.

### 4.3 The assistant line

VISION's assistant: capture and explain, no persona name, never drafting anything the student sends.

- **Capture** acts at once, with Undo (VISION: the student asked). "bus pass by saturday noon" →
  "Added 'Buy bus pass' to Personal, due Saturday at 12:00 pm. **Undo**". A missing due date is never
  guessed: "Added 'Ask Dr. Brooks about the outline'. It needs a date; I'll ask later."
- **Explain** answers in the margin beside the row it is about. "why is Lab 6 first?" → the row's note.
  "how much free time do I have tonight?" → "About 5.7h, after 6:00 pm, once today's
  planned work is counted."
- **Change** arrives as a preview card that one press confirms, written through `write` like any other
  agent edit (VISION). "I already did the vocab" → "Mark VHL Lección 6: Práctica de vocabulario
  done? VHL shows 40%." **Mark done** · Leave it
- **Refuses drafting**: "write my TA an email" → "I don't write messages for you to send. I can add
  'Email Priya about Lab 8' to your list." **Add it**

### 4.4 Rules the page keeps

- Due dates always carry a time; no start-by dates; no assigned work times (PRODUCT-DECISIONS).
- No reordering; **Pin** keeps a row on top until it's done, shown by a pin glyph and the word "Pinned"
  on focus.
- Every count carries hours ("21 more · about 42h"), never a bare count (anatomy §3.6).
- Turned-in work never shows as a task. Knowlu's auto-done items appear only in What changed and
  Closed this week, with Undo.
- When more is due than there is time, the brief says so and stops: "About 6h is due by tomorrow,
  and today has about 5h free." It suggests no sacrifice.
- Keyboard accelerators (log D4): `j`/`k` move, `x` done, `p` pin, `Enter` opens, `s` starts Lock in,
  `/` the assistant line, `?` lists them. Each also has a visible control. Focus moves the margin note.

---

## 5. Visual world

**Scene.** A student at a desk at 11 pm, or in a bright library between classes. Light and dark are
designed separately; Quinn works in dark. The world is a **returned paper**: the work is the body of
the page, and the teacher's careful notes sit in the margin beside the line they are about, joined to
it. Here the student's list is the body and Knowlu is the one writing in the margin, calmly, in ink
that is not red. The red-pen cliché of a marked paper is deliberately absent: red is Knowlu's brand
and never an alarm (DESIGN-PRINCIPLES §9), so it lives in the wordmark only.

### 5.1 Type

- **Literata** (Google Fonts, variable, optical size on) for everything Knowlu says: the opening, why
  lines, replies, follow-ups, margin notes, questions, the brief. Weights 400 and 500; no italic as
  display (it is the category's rut, new-work.md §4 calibration). Lining figures (`lnum`) in prose.
- **Mona Sans** (Google Fonts, verified to serve 400, 500, 600) for everything that belongs to the
  student or that they press: chips, answer lines, list rows, times, the rail, the class line, proof
  blocks, the assistant field. Tabular figures (`tnum`) for every time, hour and count.
- The pairing carries meaning, so it is a rule, not a style: if Knowlu said it, it is Literata; if
  it is the student's item or action, it is Mona Sans. A student learns who is speaking without a
  label or an avatar.
- Fixed rem scale, ratio about 1.2 (operate.md): 0.8125 / 0.875 (meta, rail) / 0.9375 (chips) / 1
  (rows, margin notes) / 1.125 (thread prose, 18 px, 1.55 line height) / 1.25 (Today's first row) /
  1.375 (the brief) / 2 (the reveal heading).
- Measure: the thread at 38rem and the brief at 45rem stay inside 60-75 characters for prose
  (research/notion.md §3.2); margin notes run 40-50 characters, like real margin notes.

### 5.2 Palette tokens (OKLCH)

Colour strategy: **Restrained**. Neutrals tinted from blue-black ink, plus one pen-blue accent for
Knowlu's leader lines, links and focus. Lightness steps follow Stripe's step-distance rule (research/stripe.md
§4.1): text sits at least five steps from its ground. Every text pair measures at least 4.5:1, icons
and large text at least 3:1.

| Token | Light | Dark | Job |
|---|---|---|---|
| `--paper` | `oklch(0.985 0.004 250)` cool white | `oklch(0.205 0.022 262)` blue-black | main column and thread ground |
| `--margin` | `oklch(0.962 0.008 252)` | `oklch(0.235 0.026 262)` | margin column and rail ground: one step cooler, so the margin reads as margin |
| `--sheet` | `oklch(0.995 0.002 250)` | `oklch(0.265 0.03 262)` | the sign-in sheet, the decision card, popovers |
| `--ink` | `oklch(0.255 0.045 265)` blue-black | `oklch(0.935 0.01 250)` | text; the filled primary chip |
| `--ink-2` | `oklch(0.44 0.035 262)` | `oklch(0.79 0.02 255)` | secondary text, earlier exchanges, rows past free time |
| `--rule` | `oklch(0.88 0.012 255)` | `oklch(0.34 0.025 262)` | hairlines, the fold, the margin edge |
| `--pen` | `oklch(0.43 0.12 262)` | `oklch(0.78 0.10 255)` | leader lines, links, the "now" underline, focus ring |
| `--on-ink` | `oklch(0.985 0.004 250)` | `oklch(0.205 0.022 262)` | text on the primary chip |
| `--tick` | `oklch(0.48 0.09 170)` | `oklch(0.80 0.09 170)` | done, always with the tick shape |
| `--caret` | `oklch(0.56 0.13 70)` | `oklch(0.82 0.12 80)` | needs you and broken, always with the caret and words |
| `--quote` | `oklch(0.95 0.01 250)` | `oklch(0.25 0.025 262)` | ground of a quoted email inside an ask |
| `--brand` | `#BC4444` | `#BC4444` | the wordmark only; in dark it sits on a small paper-white plate |
| course tints | chip fill L 0.93 C 0.04, label L 0.40 | fill L 0.32 C 0.05, label L 0.88 | BIO h160, CHEM h310, CS h200, SPAN h115, ENG h55; job, club and Personal neutral |

Selection, caret, scrollbars and focus rings take these tokens (craft-floor "browser surfaces").

### 5.3 Density and spacing

- The thread: 12 px between a why line and its chips, 8 px between chips, 28 px between exchanges,
  20 px between a reply and its proof block. More space above an exchange than inside it (Notion's
  adjacency rule, research/notion.md §3.1).
- Today: rows 40 px, 2 px apart; the first row 72 px; 32 px between the brief, the day line, the class
  line and the list. Margin notes 24 px apart.
- No cards on the page. The only enclosed shapes are the sign-in sheet, the decision card and the
  quoted email inside it, and popovers. A note is separated by space and joined to its subject by a
  1 px leader line, never by a box or a coloured left border.

### 5.4 Motion

One grammar: 150-250 ms, exponential ease-out, from an already-visible state (operate.md).

- **A reply arrives:** opacity 0 → 1 with a 4 px rise, 200 ms. Never a typing indicator, never
  letter-by-letter text: that would be theatre about effort (BRAND-VOICE §9).
- **A proof line lands:** "reading…" becomes its count as the tick draws in 240 ms, Stripe
  Checkout's success checkmark (research/stripe.md §4.2). No error shake.
- **Chips answer:** the pressed chip moves to the answer line (FLIP, 200 ms); the others fade out.
  Every chip shows a pressed state within 100 ms, and a waiting chip says what it waits for (first
  install item 13, research/product-truth.md §5).
- **A follow-up** inserts above the open question; the question does not move, the thread above it
  makes room.
- **The reveal** is the one authored moment (§3.1). **Today**: the margin note glides to the focused
  row (200 ms) and redraws its leader line; a checked row settles in place, struck through.

With reduced motion every move is an instant swap or a 150 ms crossfade, leader lines are drawn
already, and the reading dots are static.

### 5.5 Glyphs

Authored SVG, one 1.5 px stroke, round caps: tick, insertion caret (needs you), dash (skipped), three
reading dots, lock (sign-in sheet), pin, the class line's up-step and down-step, the grade ring. No
emoji and no Unicode stand-ins (craft-floor).

---

## 6. What is taken from each reference

### 6.1 Superhuman: the 1:1 onboarding session, run on the user's own inbox

Primary source: Gaurav Vohra, "Superhuman's onboarding playbook", First Round Review, April 23,
2025 (https://review.firstround.com/superhuman-onboarding-playbook/), quotes re-read 2026-10-23;
summarised in research/landscape.md §1 "Superhuman" and §3 candidate 5.

| Taken | Superhuman's own words | Where it lands in Knowlu |
|---|---|---|
| The session happens in the customer's real accounts, not a demo | Playbook tactic: "Log into *all* email addresses in Superhuman"; "We could rapidly diagnose each customer's current setup in our human-led onboarding." | Every exchange works on data just read from the student's own school, and the reply diagnoses it in their terms ("CS 110's isn't in Blackboard", "ENG 210's syllabus has no late rule") |
| Teach the core actions by doing them, encouraged, like a lesson | "Like a piano lesson, our customers benefitted from a real human encouraging them to try new interactions." | Undo (`S-undo`, `G-proof`), Confirm (`K1`), Pin (`W-pin`) are each offered once, on a real item, at the moment it appears |
| Teach only the actions that matter, drop navigation | The self-serve version first taught "j" and "k" to scroll, then dropped navigation to focus on marking done ("e") and reminders ("h"). | Onboarding never teaches keys or layout. It teaches three gestures; `j`/`k` and the rest are shown on the main page only, on focus |
| The taught action is the real action | "We make it so that tapping 'Z' actually undoes the send." | Undo at `S-undo` really un-archives, through the engine's journaled `write`, and the item reappears in the list |
| Clear the old pile before the user meets their real inbox | "Get Me To Zero" "interrupts customers right before they hit their 'real' inbox" and marks everything before a chosen date done; 57% opt in. | Past and turned-in work is archived before the student ever sees a list ("52 past items and 9 already turned in were archived"). Adapted: Knowlu does not ask for a cut-off date; archiving at first sight is ruled (spec §6.3), so the student gets the count and Undo instead of a choice |
| Setup as full-screen steps before the product, not a side checklist | "We shifted from steps in a tucked-away checklist to full-screen panels before hitting the main product"; completion rose "from 30% to over 98%". | The whole onboarding is one full-window thread before the main page exists (spec §1.2); the main page is first shown at `R` |
| A guided session is worth its length | "Nothing activates a customer better than manually onboarding them to your product"; "Over 65% of new customers fully transitioned their email to Superhuman after their human-led onboarding", "more than double" the no-onboarding rate. | Licence for an 8-15 minute onboarding, provided every exchange does real work and shows it (research/landscape.md §2 finding 4) |
| Turn a stumble into a good moment in the session | "An in-person conversation can turn moments of disappointment into moments of delight." | A closed school window, a Duo timeout or a refused VHL sign-in is answered in the thread, in the same calm voice, with the way back (§1.8), not a red error page |
| Speed as part of the promise [observed, not re-verified] | research/landscape.md §1: "every action under 100 ms". | Every chip shows its pressed state within 100 ms and says what it waits for |

**Not taken.** The mandatory human call, and the verbal promise to use the product daily for 30
days ("Have customers verbally commit…"): pressure the brand forbids. The **synthetic inbox** of the
self-serve version: Knowlu has the student's real data from the school sign-in on, and a practice item
in the list would be a lie about the work (research/notion.md §1.4). The playbook's
"delete Gmail and Outlook bookmarks" tactics. Superhuman's keyboard-first identity: here keys are
accelerators beside visible controls (log D4).

### 6.2 Notion

| Taken | Where it lands | Citation |
|---|---|---|
| Notion Mail's proof drawn from your own inbox: what was found, from whom ("found 3 emails that look like assignments, from 2 professors") | `G-proof`: "One email from before today looks like something to do, from Pre-Health Society: dues, $15 …"; the 39 that could not affect the plan are counted on request, never listed | notion.com/blog/introducing-notion-mail; techcrunch.com/2025/04/15/notion-releases-its-ai-driven-email-inbox; research/notion.md §6 |
| Ivan Zhao's "Most of the time, what matters is the dumbest path. Make that great." | Today's dumbest path is designed first: open, read the brief's first sentence ("Lab 6: Functions is next"), press **Start** on the first row. Asks, grades and settings sit one step away in the margin | designerfounders.substack.com/p/ivan-zhao-notion; research/notion.md §6 |
| The first thing the user gives visibly shapes what comes next | From `S-proof` on, every line names Halverson State and the student's course codes | research/notion.md §1.1 |
| Never put an optional ask before the first proof | Nothing skippable comes before `S-proof`; the order is ruled | research/notion.md §1.3 |
| Learn by doing on the product itself, no tours, no fake to-dos | The three gestures are taught on real items; no tutorial rows | research/notion.md §1.4 |
| Adjacency spacing; one narrow measure; text never pure black | §5.3, §5.1; ink is blue-black | notion.com/blog/updating-the-design-of-notion-pages; research/notion.md §3.1, §3.2 |
| "System first" (Zhao) | One chip, one row, one note component everywhere; the rail becomes the margin rather than a new panel | research/notion.md §6 |

**Not taken:** the live preview pane (the ledger direction's), the intent quiz, templates,
user-arranged sections.

### 6.3 Stripe

| Taken | Where it lands | Citation |
|---|---|---|
| **Markdoc**: docs that "feel like an application rather than a traditional user manual", using the reader's own data | Every explanation names the student's own course and rule: "Your CS 110 syllabus takes 10% a day for up to 5 days", "ENG 210's syllabus has no late rule". Every why line and proof template lives in one structured copy table (§2) so the voice stays one voice | stripe.dev/blog/markdoc; research/stripe.md §4.4 |
| **Link**: recognise, then verify, then fill | The kept school session: one sign-in, then "You won't sign in to these again", and `E` says "Nothing new to do here" because it is recognised. At resume, an expired session is recognised and re-verified in place | stripe.com/payments/link; support.stripe.com/questions/integrating-link-with-the-payment-element; research/stripe.md §3.2 |
| Requirement states that separate "we're checking" from "you must act" | The rail's six states, each a word and a shape (§1.6) | docs.stripe.com/connect/handling-api-verification; research/stripe.md §1.2 |
| "Save for later", and leaving is not finishing | `X` and `Q` | docs.stripe.com/connect/hosted-onboarding#return-url; research/stripe.md §1.4 |
| Pending states carry a time expectation | Every "reading…" (§1.9) | research/stripe.md §1.8 |
| Errors as cause plus fix, with a code | §1.8 | research/stripe.md §1.7 |
| Checkout's order summary; one pay button; cancel terms | `P` (§2.2): the proof is the summary | stripe.com/resources/more/payment-page-template-best-practices; research/stripe.md §3.1 |
| Prefill, then confirm | `K1` to `K4` | docs.stripe.com/connect/embedded-onboarding#prefill-information; research/stripe.md §1.5 |
| Summary with per-section edit | The reveal's week rows each open an editor and return | research/stripe.md §1.6 |
| Banner only when action is needed, calm line when healthy | The top bar's status line | research/stripe.md §2.1 |
| Perceptual palette with a step-distance rule; the success checkmark as the one motion | §5.2, §5.4 | stripe.com/blog/accessible-color-systems; research/stripe.md §4.1, §4.2 |

**Not taken:** the unordered, requirements-rendered flow (the order is ruled), red for negative
states, Dashboard density.

---

## 7. Risks and questions for Quinn

1. **Does Knowlu say "I"?** BRAND-VOICE leaves it open for direct conversation. This direction is a
   direct conversation, so the thread and the assistant say "I" ("I've archived them. Undo is here if
   I'm wrong."); rows, the rail and status never do. The alternative rewrites every line in the third
   person ("Knowlu archived them"). The prototype has a toggle; recommendation: "I", in the thread only.
2. **The chat frame can bury the list.** Mitigated structurally: on Today the list owns a column that
   never scrolls, the brief's first sentence names the next task, and the margin can never push the
   list down. Needs the 2-second test on the prototype at 1280x800 and 1024x700.
3. **A conversation invites typing.** Onboarding has no text field outside the sign-in sheet. The
   assistant line appears only after the reveal. Check that no exchange reads as if it expects a typed
   reply (no open questions such as "What else should I know?").
4. **Gestures are offered, not forced.** The thesis says the student "has already undone, confirmed and
   pinned once". Confirm is certain (Your week is required); Undo and Pin are offered on real items
   and may rightly be declined, because Knowlu won't invent a mistake to practise on. Maya's main path
   keeps the order at `W-pin`; `R-pin` shows the pinned branch.
5. **Every region renders, in the margin.** This direction keeps anatomy §4.4 (each empty region says
   so) but as one quiet line, where the one-thing-per-screen direction hides empty regions. Which
   ruling does Quinn want?
6. **Reading time.** Prose is slower to scan than a step map. The rail keeps a scannable summary, and
   the thread caps every Knowlu line at two sentences before its chips. Time a walk-through against
   the spec's 8-15 minutes.
7. **School picker by chips** assumes a short curated list (the friends release). A long list needs a
   search field, which is typing; the `S-other` path covers uncurated schools with the feed link.

---

## 8. Build notes for the prototype (this folder only)

- One `index.html` with local CSS and JS; no network calls except Google Fonts (Literata, Mona Sans).
  The sample data is a copy of `../shared/sample-student.json` inlined at build time, because a
  `file://` fetch fails.
- **State jumper.** A collapsible panel lists every id in §1.3 plus `R`, `R-pin`, `R-skip`, `R-cap`,
  `T`, `T-free` (all clear), `T-narrow` (under 1100 px), and the variants: tester payment, partial
  Google grant, Gmail forwarding, a VHL refusal, a Duo wait, a second closed school window, a resume
  launch, a resume with an expired school session, and a wait that hits the 3-minute cap.
- **Controls.** Light/dark; motion on or reduced; "I" or "Knowlu" in the thread; condensed or
  roomy rows; **Copy my settings** copies every value as text (COLLABORATION).
- **Accessibility.** The thread is an `<ol>` of exchanges; the open question carries `aria-current`;
  replies and follow-ups are announced through a polite live region; chips are buttons with visible
  focus; the rail is an `<ol>` with `aria-current="step"`; the margin note is linked to its row with
  `aria-describedby`. Full keyboard throughout.
- Invented data only: no real school, person, account or address.
