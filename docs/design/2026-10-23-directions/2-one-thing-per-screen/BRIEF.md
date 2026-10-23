# Direction 2: Setup, one screen at a time (`one-thing-per-screen`)

Written 2026-10-23 for the five-direction UI run. Inputs: Notion, Stripe, and **Apple Setup
Assistant** as the third reference. This brief is the build contract for the prototype in this folder.
Sample data is `../shared/sample-student.json` (Maya Okafor, Halverson State University; all
invented). Every number below comes from that file. Ruled facts come from
`docs/specs/2026-10-23-onboarding-redesign-draft.md` ("the spec") and `../research/product-truth.md`.
Where this brief and the spec disagree, the spec wins.

> **Build note (2026-10-23).** The prototype (`index.html`, ids in `screens.json`) resolved two
> red-team rounds and then two critiques. Where §1-§4 below disagree with **§9 Build decisions**,
> §9 wins; where §9 disagrees with **§10 Second pass**, §10 wins. Each section records what changed
> and why.

**Thesis.** The student never sees more than one thing to do. The ruled ask order is told as one
straight story. Each full-bleed sheet gives the why line, then one button, then the proof. The week
stays hidden until the reveal so that it lands as a single event, the way Setup's "Hello" and
transfer screens do. The main page keeps the same discipline: Today is one document, read top to
bottom, that starts with a verdict and then the next task.

**What it refuses.** It refuses a dashboard of tiles during setup, a live preview pane, and a main
page made of boxes. Gaps between groups do the grouping. Every Today region still renders, with its
named empty state (§9).

---

## Direction contract

- **THESIS.** One thing per screen, told in the ruled order. Proof is counted in the student's own
  terms on a step map pinned at the foot of every sheet. Today is one document: a verdict sentence,
  then the next task, then the rest.
- **OWN-WORLD.** Campus wayfinding signage (pass 2, §10.6). Each screen is one sign: a large heading
  and one action. The foot of the screen is a route line, like a transit map, with a station per
  ask. A filled station is done, a ringed station is "you are here", and an open station is still
  to come. The light ground is warm paper and the dark ground is warm charcoal. One signage green
  marks the primary action and the route already travelled. The red block appears only in the
  wordmark, which sits directly on the ground, with a reversed version for dark. Public Sans (the
  US Web Design System face, built for public signage and forms) is used throughout, with tabular
  figures. Thin rules are used, never cards or boxed panels.
- **STORY.** The student understands that each login buys something named and sees it counted at
  once. They believe the counts because the proof names their own classes and turned-in work. They
  type only what a site needs: a homework-site email and password, and perhaps a code or a link.
- **FIRST VIEWPORT (onboarding).** A 1280x800 window. A 52 px top bar holds the wordmark on the
  left and the honest time left plus "Finish later" on the right. A centred 38rem column holds, top
  to bottom, the receipt of the step just finished, a 2.4rem heading, the why line, and one primary
  button. The skip link and its cost always sit above the fold. The route line runs along the foot.
- **FIRST VIEWPORT (Today).** The verdict names the next task ("Lab 6 is next.") at 2.25rem, with
  one number under it (the free time after today's work). The task follows with two actions, then
  the rest of the list. A 17rem ambient column on the right holds only the status and the day.
- **FORM.** Direction 2 of the five. Its structural bet (§9.1): every ask is its own sheet and is
  counted by name; the week is confirmed on the week itself; and the daily page opens with a short
  morning pass (what changed, then the questions, one at a time) before Today.
- **FINISH.** The prototype is built and screenshotted (`shots/` light, `shots/dark/` for key
  screens). Pass 2 resolved two critiques and had one fix round. No DESIGN.md has been written.

---

## 1. Flow map

### 1.1 Frame shared by every onboarding sheet

- **Top bar.** On the left is the wordmark plate. On the right are the time left ("About 8 minutes
  left") and **Finish later**. Finish later is never shown while a school, Google or checkout window
  is open, because quitting then would strand that window.
- **The sheet.** It holds one numeral, one heading, one why line, one primary button, and at most
  one quiet secondary link (Set up later, or a fallback). The proof appears in place on the same
  sheet after the ask. No sheet carries a second primary action.
- **The step map (foot).** It holds the head start, then seven stops: `School · Payment · Email ·
  Google · Homework · Syllabi · Your week`. Each stop shows one of six states, always as a word plus
  a shape and never by colour alone (Stripe's requirement states, research/stripe.md §1.2):

  | State | Shape | Word | Example under the stop |
  |---|---|---|---|
  | now | filled signage-blue bar under the label | "Now" | none |
  | not yet | hollow ring | none | none |
  | reading | ring with a slow quarter-turn (static with reduced motion) | "reading…" | "usually 1-3 minutes" |
  | done | check | the proof, short | "5 classes · 26 assignments" |
  | needs you | ochre diamond | the fix | "VHL: sign in again" |
  | skipped | dash | "Skipped" | "zyBooks due dates missing" |

  The head start is the count at the left end: **"2 of 9 done: Knowlu installed, your account"**.
  It is endowed progress, but both items are real completed steps, and the line says what they are
  (Nunes & Drèze 2006, research/onboarding-evidence.md §1.1). The nine are: installed, account, and
  the seven stops. Clicking a done stop opens its proof read-only in a popover. Stops are never a way
  to jump ahead, because the order is ruled.
- **Time left.** The sum of the spec's median student time for the remaining steps (spec §1.3),
  rounded to the minute and never shown falling below "About 1 minute". It counts the student's time
  only, never background time.

### 1.2 Screens in order

Ids are stable, so the prototype's routes and copy table use them. A sheet whose id has a letter
suffix is a state of the same sheet, not a new screen.

| Id | Screen | Shows | Leaves when | Goes to |
|---|---|---|---|---|
| `W0` | Welcome | numeral none; the welcome line; **Get started**; head start "1 of 9 done: Knowlu installed" | button | `A1` |
| `A1` | Your account | why line; price disclosed; **Continue with Google**; quiet link **Email me a code instead**; 18+ and terms checkbox inline under the buttons | Google returns / code link chosen | `A1-ok` / `A1-code` |
| `A1-code` | Your account: code | "We sent a 6-digit code to …"; one code field; **Continue** | code accepted | `A1-ok` |
| `A1-ok` | Your account: proof | "Signed in as maya.okafor.demo@gmail.example"; the folder line with **Show me the folder**; **Continue to your school**. Auto-advances after 1.5 s unless focus is on the folder link | button or timer | `S1` |
| `S1` | Your school | why line; school preselected from the email domain when known, otherwise a search field; "You'll need" line; **secure step panel**; **Sign in at Halverson State**; quiet link **My school isn't listed** | button / link | `S2` / `S2-feed` |
| `S2` | Signing in (school window open) | the sheet dims to a waiting state: "Finish signing in in Halverson State's window." The real host is shown ("login.halverson.example"). Duo hint. No Finish later | window reports a session / window closed / Duo stalls | `S3` / `S2-closed` / `S2-duo` |
| `S2-duo` | Waiting for Duo | the same sheet with the Duo line raised to the top (copy in §2) | session arrives / window closed | `S3` / `S2-closed` |
| `S2-closed` | Window closed early | the cause and its fix; **Open Halverson State sign-in again**; after the second closed window, also **Trouble signing in?** | button / link | `S2` / `S2-help` |
| `S2-help` | Trouble signing in | three choices, one per line: try again; use the calendar link instead (cost named); finish later (says plainly that setup can't continue without school) | choice | `S2` / `S2-feed` / quit (`X1`) |
| `S2-feed` | Calendar link fallback | how to copy the Blackboard calendar link (3 short steps); one paste field; the cost line | valid link pasted | `S3` (fewer proof lines) |
| `S3` | Your school: proof | proof lines resolve in place from "reading…" to counts (§2). **Continue to payment** is enabled once classes and assignments are counted; grades and syllabi may still be reading and finish on the step map | button | `P1` |
| `P1` | Payment | the order summary is the school proof; the price; how to cancel; "You pay us, so we don't sell your data."; **secure step panel** with **Continue to checkout** | button | `P2` |
| `P1-tester` | Payment (tester) | the same summary; "Nothing to pay: you're a tester."; **Continue** | button | `E1` |
| `P2` | Checkout open | "Finish checkout in your browser. This sheet moves on by itself when Stripe confirms, usually under a minute."; **Open checkout again** | Stripe confirms / 10 min with no confirmation | `P3` / `P2-stale` |
| `P2-stale` | Checkout not confirmed | "Stripe hasn't confirmed yet. If you finished, it can take a minute. If you closed the page, open checkout again." | confirm / button | `P3` / `P2` |
| `P3` | Payment: proof | "Your trial is on. First charge October 25." Auto-advances after 1.5 s | timer | `E1` |
| `E1` | School email | why line. Covered: "Nothing new to do: your school sign-in covers it." with the canary reading. Not covered: one sign-in in the secure panel. Quiet **Set up later** with its cost | canary passes or student continues while reading / skip | `G1` |
| `G1` | Google: Gmail and Calendar | why line; what Google will show (warning screen while unverified); read-only line; 7-days line; **Continue to Google**; quiet **Set up later** with its cost | button / skip | `G2` / `H1` (or `C1` when no calendar) |
| `G2` | Google window open | "Finish in Google's window. Choose Advanced, then Go to Knowlu, when Google shows its warning." | consent returns / declined / closed | `G3` / `G4` / `G1` with the closed line |
| `G3` | Google: proof | Gmail and Calendar each reported separately. Partial grant: "Calendar not shared." with **Share calendar** (re-opens consent) or **Add another calendar** | button | `H1` or `C1` |
| `G4` | Gmail by forwarding | shown only on decline or a failed read: "Knowlu can bring Gmail in by forwarding instead, about 3 minutes." with 3 steps; **I've set it up** waits for the canary | canary / skip | `G3` (Gmail via forwarding) / next |
| `C1` | Another calendar | shown only when no calendar was granted; one paste field for an iCal link; **Set up later** with its cost | link / skip | `H1` |
| `H1-<site>` | Homework site, one sheet per site the syllabi mention (`H1-zybooks`, then `H1-vhl`) | why line, with the course named; **secure step panel** with email and password fields; **Sign in to zyBooks**; **Set up later** with its cost | signed in / failed / skip | next site or `Y1` / `H1-<site>-fail` |
| `H1-<site>-fail` | Site refused the sign-in | the failure copy (§2), with fields kept and the password cleared | retry / skip | `H1-<site>` / next |
| `Y1-<course>` | Syllabus missing, one sheet per course with no syllabus found (Maya: `Y1-cs110`) | why line; **Choose the syllabus file** (drop also accepted); quiet **Skip CS 110 for now** with its cost | file chosen / skip | `Y1-<course>` reading, then next / next |
| `K1` | Your week: classes | 7 meeting rows from the registrar, prefilled; Edit per row; **These are right** | button | `K2` |
| `K2` | Your week: from your syllabi | exams (10), grading (5), late rules (5, one blank) in three tight groups; Edit per row; **These are right** | button | `K3` |
| `K3` | Your week: your own week | repeating Google events with kind and level; office hours; your day; **This is my week** | button | `R0` |
| `R0` | Building your week (transfer screen) | countable per-source lines; "10 of 12 done"; up to 3 minutes | all done, or 3:00 elapsed | `R1` |
| `R1` | Here's your week | the reveal (§3); **Open Knowlu** | button | `T` (Today) |
| `X1` | Finish later (from any sheet) | the honest leaving line; **Quit for now** / **Keep going** | choice | app quits / back |
| `Q` | Resume (next launch) | the same sheet the student left, with one line above the heading (§1.4) | as that sheet | as that sheet |

`K1` to `K3` are one stop on the step map ("Your week · 1 of 3"). Each homework site and each
missing syllabus also counts within its stop ("Homework · 2 of 2").

### 1.3 Skip paths and their named costs

A skip is always a quiet text link under the primary button, set apart by a 24 px gap, and its label
names the cost. These are Setup's "Set Up Later" links, extended by the ruled cost line (spec §1.7).
The cost is said again at the reveal and is listed in Settings with **Connect**. A skipped step is
never asked about again in a nudge.

| Step | Link label | Cost line (said on the sheet, under the link) |
|---|---|---|
| School email `E1` | Set up later | "Without it, date changes and new work your professors announce by email are missed." |
| Google `G1` | Set up later | "Without it, club, job and internship tasks from Gmail are missed, and your free time ignores your own events." |
| Another calendar `C1` | Set up later | "Without it, your free time ignores events on that calendar." |
| Homework site `H1-*` | Set up zyBooks later | "Without it, zyBooks due dates are missing unless Blackboard's feed carries them." |
| Syllabus `Y1-*` | Skip CS 110 for now | "Without it, CS 110 has no exam dates beyond the feed, no grade weights and no late rule." |
| Your week rows | Leave blank (per row) | "Blank classes come back as one question each over the next few days." |

Account, school, payment and Your week cannot be skipped. School's only way around is `S2-feed`,
whose cost line reads: "With the link only, Knowlu can't see grades, class times from the registrar,
your syllabi or what you've turned in, and school email needs its own sign-in."

### 1.4 Resume after quit

- State lives in `onboarding.json` in the profile's app data, never in the vault (spec §1.6). On the
  next launch the app opens on the same sheet id (`Q`).
- One line sits above the heading, and only on the first sheet after a relaunch. Its wording follows
  the Ovsiankina finding: pick up the open loop explicitly (research/onboarding-evidence.md §1.5).
  > Picking up where you left off. School and payment are done; 4 steps left.
- The step map renders with every earlier proof intact. Background jobs that did not finish restart
  and show "reading…" again.
- A student who quit at payment returns to `P1`, with the entitlement re-checked first. If the
  school session expired, `S2` opens on its own with the line "Halverson State signed you out while
  you were away. Sign in once more; nothing you connected is lost." It never shows an error page
  (research/stripe.md §1.4).
- A student who quit while a school or Google window was open returns to `S2-closed` or to `G1`
  respectively, never to a dead waiting state.

### 1.5 Finish later (`X1`)

Stripe's "Save for later" (research/stripe.md §1.4) makes quitting a supported path. The sheet is
honest that leaving is not finishing:

> **Leaving now is fine.**
> Knowlu keeps what you've connected and opens right here next time. It won't check your work until
> setup is done: 4 steps left, about 5 minutes.
>
> [Quit for now]   Keep going

### 1.6 Slow first run

- **During the steps.** Every "reading…" carries an expected time, taken from the spec's table: "usually under a minute"
  for course list, assignments and grades; "about a minute each" for syllabi; "usually 1-3 minutes"
  for the school email check; "usually under a minute" for Gmail and Calendar
  (research/stripe.md §1.8). A read that runs past twice its expected time adds one honest line on
  the step map: "Still reading Halverson State's grades. You can keep going."
- **At the reveal (`R0`).** The wait is capped at 3 minutes after `K3`. It shows a countable
  indicator, never a spinner (NN/g progress indicators, research/onboarding-evidence.md §3.9). At
  the cap it opens `R1` and names what is still running. Backfills are never waited for.
- **A source that fails** is named with its fix on the step map at once, and again on `R1`. It never
  stays as a silent "reading…".

---

## 2. Onboarding copy, sheet by sheet

These follow BRAND-VOICE: sentence case, "you", contractions, numerals, lowercase am/pm, no
exclamation marks, and no "just", "simply", "easy" or "don't worry". The why lines are the spec's
wording, which Quinn reads as final copy. The proof lines come from `sample-student.json`. A privacy
line appears only on the sheets where a secret or a permission changes hands, inside the **secure
step panel**. The panel is enclosed, with a 1 px signage-blue border, a slightly raised ground, a
small lock glyph (authored SVG) and one plain sentence. No sheet other than `S1`, `P1`, `E1` (when it
asks), `H1-*` and `G1` uses this panel, so it keeps its meaning (Baymard,
research/onboarding-evidence.md §3.4).

| Id | Numeral | Heading | Why line (benefit, before the ask) | Primary button | Privacy line / extra | Proof line (after) |
|---|---|---|---|---|---|---|
| `W0` | none | Knowlu | "Knowlu answers one question every morning: what's next? Next, you'll connect the places that already know your work, about 10 minutes." | Get started | none | none |
| `A1` | 1 | Your account | "Your plan lives in your account, so it's safe if this computer isn't." | Continue with Google | The price is disclosed here, before any school login (NN/g up-front disclosure, research/onboarding-evidence.md §3.5): "Knowlu is $9.99 a month after 7 days free. There's no free plan. You'll see what Knowlu finds at your school before you pay." Google line: "Google shares your name and email, nothing else." | "Signed in as maya.okafor.demo@gmail.example." Then: "Your plan is kept as plain files on this computer." **Show me the folder** |
| `S1` | 2 | Your school | "One school sign-in lets Knowlu read your classes, class times, assignments, grades and school email. You won't sign in to these again." | Sign in at Halverson State | Expectation (Plaid, research/onboarding-evidence.md §2.4): "You'll need your Halverson State email and password, and your phone for Duo." Panel: "You sign in on Halverson State's own page. Knowlu never sees or keeps your password; it keeps the signed-in session on this computer." | Lines resolve one by one: "5 classes this term (2 from last term left out)" · "Class times for all 5: 7 meetings a week" · "26 assignments, 4 already turned in" · "Grades for 5 courses" · "Syllabi found for 4 of 5" |
| `P1` | 3 | Payment | "Everything you just connected keeps running every day for $9.99 a month, after 7 days free." | Continue to checkout | Order summary above the button (§2.1). Panel: "Checkout opens in your browser on Stripe's page. Knowlu never sees your card." Then: "You pay us, so we don't sell your data." | "Your trial is on. First charge October 25." |
| `E1` | 4 | School email | "Professors change dates by email. Knowlu reads what arrives and proposes each change for you to approve." | Continue (covered) / Sign in to school email (not covered) | Covered: "Nothing new to do: your school sign-in covers it." | "Forwarding on. A test message arrived in 38 s. New mail is read as it arrives; older school mail is still coming in, which can take a day." |
| `G1` | 5 | Gmail and Calendar | "Clubs, jobs and internships write to your Gmail, and your calendar says when you're already busy. One Google permission covers both, read-only." | Continue to Google | Panel: "Read-only: Knowlu can't send, delete or change anything in Gmail or your calendar. Gmail brings in the last 7 days of mail." Then: "Google will show a warning first, because Google is still reviewing Knowlu. Choose Advanced, then Go to Knowlu." (first-install item 1) | "Gmail: read 40 recent messages; 1 thing to do (Pre-Health Society dues, added)." · "Calendar: 6 events this week. 3 of them repeat; you'll check those on Your week." |
| `G3` partial | 5 | Gmail and Calendar | none | Share calendar | none | "Gmail connected: read 40 recent messages; 1 thing to do." · "Calendar not shared. Your free time won't count your own events until it is." |
| `C1` | 5 | Another calendar | "If your calendar isn't Google's, add it so your free time is honest." | Add calendar | "Paste the calendar's iCal link. Knowlu only reads it." | "9 events this week." (variant; not shown for Maya) |
| `H1-zybooks` | 6 | zyBooks | "Homework sites hold due dates your school's feed doesn't show. Your CS 110 syllabus uses zyBooks." | Sign in to zyBooks | Panel: "Your zyBooks password is stored only on this computer, in Windows Credential Manager. It never goes to Knowlu's servers." | "zyBooks: 1 book, 8 assignments, 3 already done." |
| `H1-vhl` | 6 | VHL | "Homework sites hold due dates your school's feed doesn't show. Your SPAN 202 syllabus uses VHL." | Sign in to VHL | Same panel, naming VHL | "VHL: 1 course, 6 assignments, 2 already done." |
| `Y1-cs110` | 7 | CS 110's syllabus | "Your syllabus has every exam date, how each course is graded, and the late-work rules. CS 110's wasn't in Blackboard." | Choose the syllabus file | "The file is read once to find dates and rules. It stays in your plan folder on this computer." | Reading: "Reading CS 110's syllabus… usually under a minute." Then: "Read 5 of 5: 10 exams, weights for 5, late rules for 4. ENG 210's syllabus has no late rule." |
| `K1` | 8 | Your classes | "Check your classes. Everything here can be changed later." | These are right | "From Halverson State's registrar." | "7 meetings for 5 classes, confirmed." |
| `K2` | 8 | From your syllabi | "Exams, grade weights and late rules, as your syllabi state them." | These are right | none | "10 exams, weights for 5 courses, late rules for 4, confirmed." |
| `K3` | 8 | Your own week | "Your job, clubs and office hours, from your calendar and syllabi. Say how firm each one is." | This is my week | Level words: "Firm" (hard: never overlapped), "Flexible" (soft: may be overlapped, says so), "Optional" (never counts as busy) | "3 of your own, 2 office hours, your day 7:30 am to 11:30 pm." |

**Row format on `K1` to `K3`.** Each row reads, left to right: the label, the times in tabular
figures, the place, the source in quiet text, then **Edit** and **Not mine**. For example:
`BIO 201 lab   Thu 2:00-4:50 pm   Kessler Hall 310   registrar   Edit`. ENG 210's late-rule row reads
"No late rule in ENG 210's syllabus." with **Add one**, and leaving it blank is fine. Typing appears
only inside Edit, as the last resort.

### 2.1 Payment order summary (`P1`)

This is Stripe Checkout's order summary, and here the summary is the proof (research/stripe.md §3.1):

```
Halverson State · fall 2026
  5 classes, 7 meetings a week
  26 assignments, 4 already turned in
  Grades for 5 courses
  Syllabi for 4 of 5 (you'll add the last one in a few minutes)
------------------------------------------
Knowlu                        $9.99 a month
7 days free; first charge October 25
Cancel any time in Settings. Cancelling before October 25 costs nothing.
Other plans: $39.99 for a 6-month term · Pell Grant recipients pay 75% less
```

A line that is still being read shows "reading…" in place. The button is enabled once classes and
assignments are counted.

### 2.2 Failure and waiting copy

Each line gives the cause and the fix in plain words, with a code underneath for support
(research/stripe.md §1.7). None of it uses red, and nothing shakes.

| Where | Copy |
|---|---|
| `S2` Duo hint (always, while the window is open) | "Halverson State will ask your phone to approve. Keep it nearby." |
| `S2-duo` (window on the Duo page for 45 s) | "Waiting for your phone. If the request timed out, ask for a new push in Halverson State's window." |
| `S2-closed` | "The Halverson State window closed before sign-in finished. Open it again when you're ready." |
| `S2-help` | "Trouble signing in? Try again · Use your Blackboard calendar link instead (Knowlu then can't see grades, class times, syllabi or turned-in work) · Finish later. Setup continues once your school is connected." |
| `H1-vhl-fail` | "VHL didn't accept the sign-in. Try again, or set up VHL later; its due dates will be missing until it's connected." |
| `G2` decline / failed read | "Gmail's read failed. Knowlu can bring Gmail in by forwarding instead, about 3 minutes." |
| `G2` closed | "Google's window closed before you chose. Continue to Google when you're ready." |
| Read running long (step map) | "Still reading Halverson State's grades. You can keep going." |
| `E1` canary slow | "Waiting for the test message. Usually 1-3 minutes; you can keep going." |

---

## 3. "Here's your week": the reveal

### 3.1 `R0`: building your week (Setup's transfer screen)

This sheet has no numeral. The heading is "Building your week", with "10 of 12 done · usually under
3 minutes" beneath it. Twelve per-source lines follow, one for each rail entry in the sample. Each
line starts as "reading…" and turns into its proof as a check draws in 240 ms. The lines show true
engine progress, never a scripted ticker (labor illusion, Buell & Norton 2011,
research/onboarding-evidence.md §1.4). The last three lines are the ones doing real work at this
point:

> Reading CS 110's syllabus… found the late rule: −10% a day, up to 5 days.
> Checking 40 assignments against what you've turned in… 9 already turned in, archived.
> Estimating effort for 34 assignments… done.

No timer is visible. At 3:00 the screen moves to `R1` and carries any unfinished line over as "still
coming". Maya's opened after 40 s, when CS 110's syllabus finished.

### 3.2 `R1`: the reveal

This is the product's one expressive moment. The step map at the foot rises and unfolds into the
week, so the stops' proof becomes the week's content. It is a single 600 ms FLIP transition, or a
crossfade with reduced motion. The layout at 1280x800:

- **Heading.** "Here's your week." at 2.5rem, with "Monday, October 19 to Sunday, October 25" under
  it.
- **The week** (full column width, about 60% of the height). There are seven day columns. Class and
  commitment blocks are soft course tints with the course code printed on them. Due work is a small
  marker at its due time, labelled with the title and estimate. Optional items are outlined, never
  filled. A block's room shows on hover or focus. Thursday shows the BIO 201 lab 2:00-4:50 pm and
  "Lab 4 report · due 2:00 pm · 3h".
- **Below the week**, a single 40rem reading column holds these groups, separated by wide gaps:
  1. **What's next.** "Reading: Ch. 8, Metabolism is next. BIO 201 · due tomorrow at 9:00 am · about
     30 minutes." Why: "It's the first thing due this week, before BIO 201."
  2. **Tonight.** "About 3h free tonight, after 9:00 pm."
  3. **Tidied.** One line each, no lists: "52 past items and 9 already turned in were archived." ·
     "Effort estimated for 34 assignments." · "2 classes from last term left out."
  4. **From your email.** "Pay Pre-Health Society dues ($15), due Friday, October 30 at 5:00 pm."
     with **Undo**.
  5. **Still coming.** "Older school email is still coming in; anything it finds will be proposed."
     In the skipped-sources variant: "2 things not connected: zyBooks, Google Calendar. Connect them
     in Settings." Each one is named with its cost.
  6. **What happens now.** "Knowlu checks again at 12:00 pm and 6:00 pm, and starts with Windows.
     Change this in Settings." The scheduler and autostart turn on at this point (ruling 12).
- **One button: Open Knowlu.** It is the only filled button on the screen.
- Each block on the week opens its own small editor and returns to the reveal. This is Stripe's
  summary with per-section edit (research/stripe.md §1.6).

---

## 4. The main page: Today as one document

### 4.1 Structure (top to bottom), with its source in the read model

The page is drawn from `sample-student.json` → `today`, at Monday, October 26, 8:15 am. Field names
follow `docs/surface/anatomy.md` and research/product-truth.md §2C.

| # | Region | Read-model source | Shown | Exists when |
|---|---|---|---|---|
| 1 | Date line | `topline` | "Monday, October 26 · week 10 of 16" | always |
| 2 | **Verdict** | `verdict.headline`, `free_time` | "About 4.7h is due by tomorrow, and today has room for it. About 5.7h is yours after 6:00 pm." | always (empty vault: "Nothing active.") |
| 3 | Meter | `meter` | One row of 30-minute blocks: 10 must-do blocks inside a 13-block "today's room" outline. Spill blocks would sit outside it, hatched, never red. Hovering names the tasks | always |
| 4 | What changed | `delta` | "Since 6:00 pm yesterday: Pre-lecture quiz: Ch. 7 marked done by Knowlu (Blackboard shows it turned in) · Undo · BIO 201 Lab 4 graded 44/50 · 1 new question · VHL needs you to sign in again." | only when non-empty |
| 5 | **Next** (large) | `list[0]` | "Lab 6: Functions" at 1.75rem; "CS 110 · zyBooks · 40% done"; "Was due Friday at 11:59 pm · −30% now; closes Wednesday at 11:59 pm"; "About 1h 12m left"; why: "Each day late costs 10%; finishing today keeps it at −30%." Buttons: **Start** (Lock in), **Open in zyBooks**, a done circle; Pin, Edit and Report a problem under "More" | always (all clear: "Nothing is due today. The day is yours." plus "Next up tomorrow: …") |
| 6 | The rest of today | `list[1..]`, `done_today` | Tight rows, each with a check, title, course chip, due time and estimate: zyBooks 7.1-7.4 participation · today at 11:59 pm · 24m; VHL Lección 6: Práctica de vocabulario · today at 11:59 pm · 36m; Problem set 7 · tomorrow at 9:30 am · 1h 30m; Peer review: outline exchange · tomorrow at 12:30 pm · 1h; Study for Exam 2 (Ch. 5-7) · Thursday at 9:30 am · 1h 48m today (of 4h left). Done today: "Reading: Ch. 9, Cellular respiration", quietly struck through, checked at 7:55 am, and can be unchecked | always |
| 7 | The fold | `free_time`, `later` | A hairline labelled "Today's free time ends here." Below it, the first 3 `later` rows in secondary ink (still ≥ 4.5:1), then "18 more this term · about 36h · See all" | when `later` is non-empty |
| 8 | Needs you | `decisions` | A deck showing one card at a time, so the page never grows with the queue. Header: "Needs you · 4 questions, oldest 7 days". Card, oldest first by `first_proposed_at`: "Add 'SURF application' to Personal, due Friday, November 6 at 5:00 pm, about 2h?" **Add it** · **Not for me** · Not now. "From an email, October 19 · Show the email" (quoted as untrusted text). "3 more behind this one" | only when pending > 0 |
| 9 | Late window closed | `late_window_closed` | "Problem set 6 · CHEM 231 · was due last Tuesday at 9:30 am · Late work not accepted after 48 hours (your syllabus)" with **Archive** and **Still doing it** (which asks "When will you turn it in?"). VHL Lección 5: Vocabulario appears the same way | only when non-empty |
| 10 | Good to know | `good_to_know` | "CHEM 231 Exam 2 on Thursday is in Hallett Hall 120, not the usual room." · "BIO 201 Lab 4 report graded: 44/50." · "CHEM 231 Problem set 6 is marked missing in Blackboard." Each line carries its source on request | only when non-empty |
| 11 | Closed this week | `closed_this_week` | One link line: "Closed this week: 8 · 6 marked done by Knowlu because the site shows them turned in · Review or undo". It opens the 7-day history, marked you, Knowlu or lapsed, with Undo | always, as one line |

**Ambient column** (15rem, on the right, top-aligned with the verdict). It can be read at a glance
and speaks up only when something changes (DESIGN-PRINCIPLES §5).

- **Today** (`the_day`, `schedule`). "9:00-9:50 am BIO 201 lecture · Kessler Hall 104", "10:00-10:50
  am SPAN 202 lecture", "1:00-2:15 pm CS 110 lecture", "3:30-4:30 pm CHEM 231 office hours" (outlined,
  with the word "optional"). Free time is drawn as the gaps, with "About 5.7h free after 6:00 pm"
  beneath.
- **Coming up** (`upcoming_exams`, `ahead_takeaway`). "CHEM 231 Exam 2 · Thursday 9:30 am · Hallett
  Hall 120 (moved, by email)" and "Friday is the heaviest day in the next two weeks, about 10.2h
  due." Clicking opens the week.
- **Your classes** (`domains`). One line each, shown as a word plus a shape: "Organic Chemistry ·
  about 2h behind", "Spanish · about a day ahead", and so on. Clicking filters the list, and clicking
  again clears it. The sentence with the way back ("Today's study session and one Wednesday catch it
  up before Thursday's exam.") shows on hover or focus.
- **Status** (`sources`, `topline`), at the foot. It is calm by default: "Up to date, 6:04 am ·
  Nothing else needs you". It speaks up only when the student must act, marked with the ochre
  diamond: "VHL signed you out. Sign in again to bring in new VHL work; what Knowlu already has is
  still here. **Sign in to VHL**"

### 4.2 Rules the page keeps

- Due dates always carry a time. There are no start-by dates and no assigned work times
  (PRODUCT-DECISIONS).
- Rows can't be reordered. **Pin** keeps a row at the top until it's done, and a pinned row shows a
  pin glyph, with the word "Pinned" on focus.
- Every count carries hours ("18 more · about 36h"), never a bare count (anatomy §3.6).
- Turned-in work never shows as a task. Knowlu's auto-done items are reachable only through Closed
  this week, with Undo.
- When more is due than there is time, the verdict says so plainly and stops there: "About 6h is due
  by tomorrow, and today has about 5h free." It suggests no sacrifice.
- Keyboard accelerators (log D4): `j`/`k` to move, `x` for done, `p` to pin, `Enter` to open, `s` to
  start Lock in, `?` for the list. Each one also has a visible control.
- **Hide until relevant** (Notion Home) departs from anatomy §1's "every section always renders".
  Silence is still never ambiguous, because the status line always states "Nothing else needs you".
  This goes to Quinn as a question (§7).

---

## 5. Visual world

**Scene.** A student at a desk, often at night, in a dim dorm room or a bright library. Both modes
are designed separately, and Quinn works in dark. The world is campus wayfinding signage: calm, large
numerals and the plain sans of building and room signs, an ink-blue sign field, white letters. A sign
says one thing and tells you where you are.

### 5.1 Type

- **Hanken Grotesk** (Google Fonts) only, at weights 400, 500 and 600, with 300 for numerals. Every
  time, hour and count uses tabular figures (`font-variant-numeric: tabular-nums`).
- A fixed rem scale with a ratio of about 1.2: 0.8125 / 0.875 / 1.0625 (body, 17 px) / 1.25 / 1.5 /
  1.75 (next task) / 2 (sheet heading) / 2.5 (reveal heading) rem. The step numeral is 5.5rem at
  weight 300 and sits left of the heading's first line, never above it (no eyebrow, per
  craft-floor).
- Measure: 60-75 characters for every reading column (onboarding 36rem, Today 40rem), even at
  1280 px (Notion, research/notion.md §3.2).

### 5.2 Palette tokens (OKLCH)

The lightness steps follow Stripe's step-distance rule: text and its ground sit at least five steps
apart (research/stripe.md §4.1). Every pair below must measure at least 4.5:1 for text, and at least
3:1 for the numeral and icons.

| Token | Light | Dark | Job |
|---|---|---|---|
| `--ground` | `oklch(0.975 0.006 250)` cool off-white | `oklch(0.205 0.035 262)` deep ink-blue | page |
| `--sheet` | `oklch(0.995 0.003 250)` | `oklch(0.24 0.04 262)` | raised surfaces (secure panel, popovers) |
| `--ink` | `oklch(0.24 0.045 262)` | `oklch(0.955 0.008 250)` | text |
| `--ink-2` | `oklch(0.43 0.035 262)` | `oklch(0.80 0.02 250)` | secondary text, rows past free time |
| `--numeral` | `oklch(0.60 0.04 255)` | `oklch(0.62 0.05 255)` | step numerals (large text, ≥ 3:1) |
| `--rule` | `oklch(0.90 0.01 250)` | `oklch(0.33 0.03 262)` | hairlines, the fold |
| `--signal` | `oklch(0.46 0.13 255)` | `oklch(0.80 0.09 245)` | "you are here", primary button fill, focus ring |
| `--on-signal` | `oklch(0.99 0 0)` | `oklch(0.205 0.035 262)` | text on primary buttons |
| `--proof` | `oklch(0.50 0.09 165)` | `oklch(0.80 0.09 165)` | done checks (always with the check shape) |
| `--needs` | `oklch(0.58 0.13 80)` | `oklch(0.84 0.12 85)` | needs you and broken (always with the diamond and words) |
| `--secure-edge` | `oklch(0.70 0.08 255)` | `oklch(0.52 0.08 255)` | the secure step panel's border |
| `--brand` | `#BC4444` | `#BC4444` | wordmark only; in dark mode it sits on a light sign plate |
| course tints | fill at L 0.93, C 0.04; label at L 0.40 | fill at L 0.32, C 0.05; label at L 0.88 | BIO h160, CHEM h310, CS h200, SPAN h115, ENG h55; work and club are neutral |

The colour strategy is Restrained: neutrals tinted from the ink-blue world, plus one signal accent.
Red never means late, behind or broken (DESIGN-PRINCIPLES §9). Selection, caret, scrollbars and focus
rings are themed from these tokens.

### 5.3 Density and spacing

Notion's adjacency rule (research/notion.md §3.1): rows within a group sit 4 px apart on 36 px row
heights, and groups sit 40 px apart. Why lines get paragraph spacing, and there is more space above a
heading than below it. Rules and gaps replace boxes, and there are no cards. The only enclosed shapes
are the secure step panel, the decision card and popovers.

### 5.4 Motion

There is one grammar: 150-250 ms, exponential ease-out, starting from an already-visible state.

- **Sheet to sheet.** The current sheet slides 24 px left and fades out while the next arrives from
  the right, like Setup's push.
- **Proof landing.** "reading…" turns into its proof as a check draws in 240 ms. This is Stripe
  Checkout's success checkmark (research/stripe.md §4.2). There is never an error shake.
- **The reveal.** The one authored moment (§3.2).
- **Today.** A row checked off settles in place, struck through; nothing jumps.

With reduced motion, slides become instant swaps, the check is static, and the reveal crossfades.

---

## 6. What is taken from each reference

### 6.1 Notion

| Taken | Where it lands | Citation |
|---|---|---|
| Home's order: now, then soon, then my work | Today: verdict, next, rest, then what's coming | notion.com/help/home-and-my-tasks; research/notion.md §2.3 |
| Sections hidden until relevant (Favorites appears only after a first favorite) | Needs you, Late window closed, Good to know and What changed exist only with content | same |
| Adjacency spacing: rows tight within a group, wide gaps between groups, paragraph spacing split from line spacing | Today's list and the onboarding proof lines; whitespace replaces boxes | notion.com/blog/updating-the-design-of-notion-pages; research/notion.md §3.1 |
| One type family, a narrow centred column, near-black text that is never pure black | Hanken only; 36-40rem columns at 1280 px; ink-blue text | notion.com/help/customize-and-style-your-content; blakecrosley.com/guides/design/notion-calendar; research/notion.md §3.2 |
| Toggle children for detail on request | A task's steps stay collapsed under the next task until opened | notion.com/blog/data-model-behind-notion; research/notion.md §2.2 |
| "Make the dumbest path great" (Zhao) | Open the app, read the verdict, press Start on the next task | designerfounders.substack.com/p/ivan-zhao-notion; research/notion.md §6 |

Not taken: the live preview pane (it belongs to the week-as-a-balance direction), user-arranged
sections, and templates.

### 6.2 Stripe

| Taken | Where it lands | Citation |
|---|---|---|
| "Save for later": quitting is supported, and the return re-checks what is outstanding | `X1` and resume `Q`; an expired session re-opens sign-in instead of an error page | docs.stripe.com/connect/hosted-onboarding#return-url; research/stripe.md §1.4 |
| Pending states carry a time expectation | Every "reading…" gives one ("usually 1-3 minutes"), and `R0` says "usually under 3 minutes" | docs.stripe.com/connect/handling-api-verification#confirm-id-verification; research/stripe.md §1.8 |
| Checkout's order summary, one visible pay button, and the cancel terms | `P1`: the school proof is the summary above the single $9.99 button | stripe.com/resources/more/payment-page-template-best-practices; research/stripe.md §3.1 |
| Distinct requirement states that separate "we're checking" from "you must act" | The step map's six states, each a word and a shape | docs.stripe.com/connect/handling-api-verification; research/stripe.md §1.2 |
| Errors written as cause plus fix, with a code | §2.2 failure copy | research/stripe.md §1.7 |
| Prefill, then confirm, with every field editable | Rows on `K1`-`K3` | docs.stripe.com/connect/embedded-onboarding#prefill-information; research/stripe.md §1.5 |
| A perceptual palette with a step-distance contrast rule | §5.2 OKLCH tokens | stripe.com/blog/accessible-color-systems; research/stripe.md §4.1 |
| Checkout's success checkmark as the one motion for proof | §5.4 | research/stripe.md §4.2 (Villar essay, secondary) |

Not taken: the requirements-rendered, unordered flow (the order here is ruled and linear), red for
negative states, and Dashboard density.

### 6.3 Apple Setup Assistant

| Taken | Where it lands | Citation |
|---|---|---|
| A long, linear setup with one decision per screen | One sheet per ask; `H1` and `Y1` split per site and per course | research/landscape.md §1 "Apple Setup Assistant"; support.apple.com/guide/imac/apd831707cb3/2022/mac/13 (the ordered setup screens) |
| Skip now, finish later, and say where to find it ("You can respond to all the prompts, or skip some steps and choose to complete them later.") | "Set up later" links, plus the named cost and "Connect in Settings" at the reveal | support.apple.com/guide/imac/apd831707cb3/2022/mac/13 |
| A pre-permission screen with **one** button that names what it opens, never "Allow" ("Use a term like 'Continue' or 'Next'…") | "Continue to Google", "Sign in at Halverson State", "Continue to checkout" | developer.apple.com/design/human-interface-guidelines/privacy (Pre-alert screens); research/onboarding-evidence.md §2.1 |
| A permission request inside onboarding, with its why and benefit, when the app can't work without it | Every ask is preceded by its why line | developer.apple.com/design/human-interface-guidelines/onboarding ("consider integrating the permission request into your onboarding flow") |
| "Postpone nonessential setup… Provide reasonable default settings" | No name, timezone or slot questions; defaults set silently (spec §1.9) | same HIG onboarding page |
| The transfer screen: background work with honest timing ("Transfer times can vary…"), and with iCloud "your apps and data download in the background" so you can start at once | `R0` per-source lines and the 3-minute cap; backfills never waited for; "still coming" named on `R1` | support.apple.com/102659 (Quick Start) |
| The "Hello" welcome moment as a single event | The week is withheld until `R1`, which arrives as one transition | research/landscape.md §1 (observed; Hello screen not re-verified) |
| Privacy said at the moment of asking, in one sentence | The secure step panel's single line | developer.apple.com/design/human-interface-guidelines/privacy ("a brief, complete sentence that's straightforward, specific") |

**Adapted, not copied.** Apple's pre-alert rule forbids a cancel option on the pre-permission screen,
but the spec rules that optional sources can be skipped with their cost named. So `G1`, `E1` and
`H1-*` keep one primary button that opens the real window, and place "Set up later" 24 px below it as
plain text that names the cost. That is Setup Assistant's own "Set Up Later" pattern, used here on
screens that are not the system alert. `S1` has no skip, only the fallback link.

Not taken: Express Setup, which turns many settings on at once and would hide asks the spec wants
explicit; the multilingual "Hello" animation; and device-to-device language.

---

## 7. Risks and questions for Quinn

1. **The withheld week can feel long.** The direction's known cost is 8-15 minutes with nothing
   assembling on screen. The step map's accumulating counts and the real head start are the
   mitigation. This needs a timed walk-through on the prototype.
2. **Hiding empty sections vs anatomy §1 ("every section always renders").** This direction hides
   Needs you, Late window closed, Good to know and What changed when they are empty, and keeps one
   status line that is always present so absence isn't ambiguous. Is that the ruling Quinn wants?
3. **"Knowlu installed" in the head start.** It is true and its reason is stated, but some students
   may read it as padding. The alternative is to start at "1 of 8 done: your account" after `A1`.
4. **Your week as three sheets** keeps one thing per screen but adds two clicks. The alternative is
   one long sheet with three groups.
5. **The reveal's week strip at 1280x800** is tight for Wednesday's six blocks. Show it at 1280 and at
   1024 wide in the prototype.

---

## 8. Build notes for the prototype (this folder only)

- One `index.html` plus local CSS and JS, with no network calls except Google Fonts. The sample data
  is a copy of `../shared/sample-student.json` inlined at build time, because a `file://` fetch
  fails.
- **State jumper.** A collapsible off-canvas panel lists every id in §1.2. It includes the failure
  states, the tester variant, a skipped-sources variant, a partial Google grant, a resume launch and a
  run that hits the 3-minute cap. Quinn judges by clicking through (COLLABORATION).
- **Controls.** Light/dark, motion on or reduced, the density of Today's rows, and hide-until-relevant
  vs always-render. A **Copy my settings** button copies every value as text.
- **Keyboard.** Tab order follows reading order. The step map is an `<ol>` with
  `aria-current="step"`. Each proof landing is announced through a polite live region. Every control
  has visible focus.
- Invented data only: no real school, person or address.

---

## 9. Build decisions (2026-10-23, the prototype)

The build resolved two red-team rounds. Each row below names the finding, what the prototype now does,
and the screen where it can be checked. Where §1-§4 disagree with this section, this section wins.

### 9.1 Structure: what changed from "wizard plus dashboard"

1. **The foot rail counts asks by name, not stops.** The rail lists each ask this student will meet:
   Account, School, Payment, School email, Google, zyBooks, VHL, CS 110 syllabus, Your week (9 for
   Maya). Before the school proof it shows two placeholders, "Homework sites" and "Syllabi". The
   school proof replaces them with the sites and syllabi the school named (`S3`), and the rail says
   so. Each sheet is one stop, so the rail advances on every sheet, including the last ones, which is
   where goal-gradient wants it to speed up. There are no sub-counters, no head start ("Knowlu
   installed" is gone) and one denominator. The step numeral is the ask's number on this list, so it
   carries information.
2. **Your week is one confirm screen, on the week itself (`K`).** K1-K3 are merged. The left side is
   the week of commitments: classes from the registrar, office hours (optional), and the student's
   own repeating Google events. Each block opens its source. The right side holds only what needs a
   look: a Firm / Flexible / Optional switch per repeating event, prefilled; the syllabus rules folded
   into one disclosure; the ENG 210 blank late rule; the wake-to-bed window as a shown default with
   Change, not an ask. One button: **This is my week**.
3. **The reveal adds the work (`R1`).** `K` showed time. `R1` lays the due work over the same grid:
   a due list per day, plus a due line at each due time. Beside it sit what's next, tonight's free
   time, what was tidied, From your email (with Undo) and what is still coming. Nothing on `R1`
   repeats a confirm the student just made.
4. **The daily page starts with a morning pass (`T0`, `T0-q`).** The pass shows only when something
   changed. It lists what changed since the last look, with Undo on Knowlu's closes, then the
   questions, one card at a time, oldest first. "Go to today" is always one click. That takes the
   one-thing-per-screen thesis into daily use. Today's own regions stay as the record.
5. **Today keeps every region and its named empty state (`T`, `T-empty`).** What changed, Needs you
   (a deck), Late window closed, Good to know and Closed this week always render. The Notion
   hide-until-relevant rule is now a jumper setting (`regions: hide`) for comparison only.

### 9.2 Ruling violations resolved

| Finding | Now | Screen |
|---|---|---|
| Older school mail "still coming", "will be proposed" | M365 older mail is held until L1: "Mail from before today isn't brought in at Halverson State yet. Knowlu will offer it in Settings once it can." Pre-setup Gmail is applied and listed under From your email with Undo (VISION commitment 5's setup exception) | `E1-done`, `G3`, `R1` |
| Gmail forwarding hand-set, no secure panel | Knowlu turns forwarding on in a Gmail sign-in window. The secure panel names the kept session, says the password is never seen or kept, and discloses the cloud read. It says forwarding reads 120 days back, where the permission reads 7 | `G4`, `G4-done` |
| No past-100 variant | Calendar comes by permission, and Gmail by forwarding in a sign-in window | `G1-fwd` |
| Google events editable on the week | A Google block shows "Times are read-only here; change them in Google Calendar." Only Knowlu's own level (Firm, Flexible, Optional) changes. Registrar and syllabus rows have Edit and Not mine | `K` (select a block) |
| Grades rings missing | Where you stand: a ring per course, filled to its percentage and coloured by letter, with the letter printed. Selecting a ring opens the breakdown by category, with what is waiting to be graded. A grades setting hides them | `T`, `T-grades` |
| Avoided task, offline | An avoided task rises and says for how long ("Passed over 4 days running…"). Offline: "Offline since 7:40 am. Everything here is from the 6:04 am check; changes stay on this computer…" The status moves to the top of the ambient column whenever it has something to say | `T-avoided`, `T-offline`, `T-broken` |
| Empty regions hidden | Always rendered, with named empty states (see 9.1.5) | `T-empty` |
| Single-school copy (Duo, Blackboard, M365) | The copy reads a campus row (`CAMPUS` in the prototype): `mfa`, `host`, `lms`, `mail`, `shared`, `simple_syllabus`, `covers`. The Duo line appears only when `mfa` is set. The feed fallback names the campus LMS | `S1`, `S2`, `S2-feed`, `S1-separate` |
| No branch without `shared_sign_in` | `S1-separate` (an invented Brightwater College on Canvas): the why line says grades and email each have their own sign-in. Grades become their own ask (`S2-sep`), and school email asks for its own sign-in (`E1-own`) | `S1-separate`, `S2-sep`, `E1-own` |
| No path for non-Gmail personal mail | "My personal email isn't Gmail" leads to guided forwarding (Outlook.com, iCloud, others) with a private Knowlu address, then `C1` for the calendar | `G5`, `C1` |
| No restore or second desktop | "I already use Knowlu on another computer" leads to `Q-restore`. The account's steps show as done "From your account". Only the on-device sign-ins (school, zyBooks, VHL) are asked. No payment | `Q-restore`, `Q-restore-ok` |
| Three confirm sheets, extra asks | One confirm screen (9.1.2). The wake-to-bed window is a shown default, not an ask | `K` |
| Simple Syllabus path dropped | "Look in Simple Syllabus" appears beside the file button when the campus row says the school publishes there | `Y1-cs110` |
| Cloud processing undisclosed | Every sheet where mail or text goes to the cloud says so: "Knowlu's cloud reads … keeps what it finds, never the message." The homework panel says titles, due dates and progress go to the account and the password never does. `S3` says its counts were read on this computer and nothing has gone to the cloud yet | `E1`, `G1`, `G4`, `G5`, `H1-*`, `Y1-cs110`, `S3` |

### 9.3 Problems resolved

- **Course codes on the proof.** `S3` names BIO 201, CHEM 231, CS 110, SPAN 202 and ENG 210. It
  quotes two turned-in titles. It counts **14 meetings a week** (the 7 rows hold 14 meetings), here
  and in the payment summary.
- **How Knowlu knew about zyBooks.** The school read finds that CS 110 links to zyBooks in Blackboard,
  and `S3` says so. "Use another homework site?" answers the case of a site nothing mentioned.
  "I sign in to zyBooks through Blackboard" covers students with no password (`H1-zybooks-lti`).
- **No timed auto-advance.** `A1-ok` and `P3` wait for the student. `P3` is a receipt beat: the plan,
  the first charge, where the receipt went, and how to cancel. Only waiting sheets move on by
  themselves (`S2`, `P2`, `R0`), and each says so.
- **Back** is in the top bar on every sheet except the waiting ones.
- **Payment.** A plan choice (Monthly with the summer pause, Term $39.99, Pell Grant 75% less), the
  cancel terms beside the button, and a privacy link. "You pay us, so we don't sell your data" is
  removed. *Open for Quinn:* how a Pell award is verified. The prototype says only "checkout confirms
  your award".
- **Privacy page** (`PV`): what the account holds (the plan, grades), what never leaves this computer
  (the school sign-in, the saved mail sign-in, homework passwords), and how email is read. It is
  linked from `A1`, `A1-ok` and `P1`.
- **Past-term count** is never shown, on `S3`, `R0` or `R1`.
- **Where you stand** replaces "Your classes", because the domains include the library job, the
  Pre-Health Society and Personal. **Add or ask** (Ctrl+Space, `T-capture`) is in the top bar.
- **Resume without surprise windows.** `S2-expired` asks the student to press **Sign in**. A sign-in
  window never opens by itself.
- **The verdict names the bleeding task.** "Lab 6 is next: it's past due, and finishing it today keeps
  it at −30%." The Next block shows Start, Open in zyBooks and the done circle, with the rest under
  More.
- **Time left** counts the student's time only. While an outside window is open, it reads "Waiting on
  …" instead of a number.
- **Where Knowlu's closes appear.** They are listed in What changed (with Undo) and in Closed this week
  (with Undo). Both are the record, and the morning pass shows them first.
- **Copy.** "just" is gone. R0 says "34 tasks", not "34 assignments". The STORY line is corrected.
- **E1 consent.** Covered schools still press **Turn on forwarding**, under a panel that says Knowlu
  adds one rule to the school mailbox and that the cloud reads the copies.
- **G1.** The sheet ties the permission to the Google account the student signed in with on `A1`. The
  aside shows what Google's unverified-app screen looks like and why it appears.
- **R1.** Autostart and the 12:00 / 6:00 checks are a visible, checked option before **Open Knowlu**:
  "in the tray; no window opens by itself".

### 9.4 Still open for Quinn

1. The morning pass (`T0`): a habit opener that keeps Today calm, or one screen too many each morning?
2. The Pell Grant verification step is undesigned (see 9.3).
3. `K`'s Firm/Flexible/Optional switch per repeating event. Is that inside "asks for sign-ins, and one
   confirm screen", or should levels default silently and be changed later?
4. Keyboard accelerators (log D4) are wired on Today: `j`, `k`, `x`, `p`, `Enter`, `s`, `?` and
   Ctrl+Space. Anatomy §6 still lists shortcuts as cut. D4 is the newer ruling, so check that it holds.

## 10. Second pass (2026-10-23): two critiques resolved

Two critiques scored the first build 56 and 61. This pass revised the prototype in place. Where this
section and §9 disagree, this section wins. Kept as the critiques asked: the S3 proof, skip links
that name their cost, the S2 waiting sheet, T0's morning pass, Q's resume line, the late-window
syllabus citations, and Closed this week with per-row Undo.

### 10.1 Structure

1. **One denominator, asks only.** The route line has 7 stops for Maya: Account, School, Payment,
   Google, Homework, Syllabi, Your week. Before the school proof, two stops read "Homework sites"
   and "Missing syllabi". After it they read "zyBooks, VHL" and "CS 110 syllabus". The count never
   changes. The lead says "N of 7 done". A stop is done when the student's ask is done. Machine work
   never appears on a stop: it goes on one quiet line under the route ("In the background: reading
   CS 110's syllabus"), so S3's heading and the route always agree. Stop labels are full size and
   carry no sub-lines; the proof opens from a done stop.
2. **No proof-only sheets.** A1-ok, P3, G3 and H1-zybooks-ok are folded in. Each step's proof is a
   receipt at the top of the next ask: the account and the plan folder on S1, the trial and receipt
   on G1, Gmail and Calendar (with Undo on the added task) on H1-zybooks, zyBooks on H1-vhl, and VHL
   on Y1-cs110. The old ids still route to those screens. S3 stays a full proof sheet because it is
   the beat before payment. About 4 clicks are gone.
3. **School email is not its own step (ruling: the school sign-in covers it).** E1 is removed from
   Maya's path. S1 lists school email among the things the one sign-in covers, and its lock line
   says Knowlu adds one forwarding rule and that the cloud reads the copies. "Leave school email
   out" names the cost. S3 then proves it: "forwarding on, and a test message arrived", plus the
   held older-mail line. E1-own and E1-done stay for a school without one shared sign-in
   (Brightwater).
4. **The week is confirmed as a list; the grid is kept for the reveal.** K is one confirm list: the
   5 classes and 7 meeting rows from the registrar, each with Change, then two plain lines. The
   repeating calendar events take silent default levels (desk firm, society flexible, soccer
   optional), which are stated and not asked. There is one button. The Firm/Flexible/Optional
   switches, the syllabus disclosure and ENG 210's "Add one" are gone. This answers §9.4 Q3: levels
   default silently.
5. **The reveal has one peak (R1), and the rest is the next beat (R2).** R1 shows the week grid and
   one "What's next". A load bar under each day shows how heavy it is, and the heaviest days are
   marked and named ("Thursday and Friday carry the most, about 4h due on each"). Due work is a
   diamond in a lane at each column's edge, so it never cuts a block. Selecting a diamond names the
   task. R2 holds what was tidied, From your email with Undo, the held older mail, and the
   background-check choice. That choice is two equal buttons, never a pre-checked box.
6. **R1 knows the hour.** At 7:19 pm it says how much of tonight is free. R1-late (11:10 pm) says
   nothing else is due tonight and names tomorrow's first thing.
7. **Late window closed is its own page (T-late).** Today shows only a pointer: "2 items left Today
   because their late windows closed. See them." T-late lists them with the syllabus rule, Archive,
   and Still doing it, which puts the item back on Today with Undo. The scheduler is gone.
8. **Today leads with the action.** The verdict is "Lab 6 is next.", and its only number is the free
   time after today's work (5.7h, the same figure as the day caption). The 4.7h, 6.5h and gauge
   readings are gone. The next block has two actions (Open in zyBooks, Mark done) and Details, which
   opens the drawer, where the focus session starts. Rows show the course as plain muted text, not
   chips. The ambient column holds only the status and the day. Grades (a ring per course, VISION)
   and Coming up move into the document as regions. Good to know icons follow their source.

### 10.2 Rulings and facts corrected

| Finding | Now | Screen |
|---|---|---|
| Pell "checkout confirms your award" | "After checkout you upload your Pell award letter; it's checked once a year." What Knowlu keeps from the letter stays open (cloud spec amendment 2026-10-16) and isn't claimed. | `P1-plans` |
| Term "paid once"; Pell term missing | Four plans: Monthly $9.99, 6-month term $39.99 "renews every 6 months until you cancel", Pell monthly $2.49, Pell term $9.99. The summer pause is named on Monthly only. | `P1-plans` |
| P1 dense, prototype bar clipped | Monthly is shown with its price; the other plans sit behind one disclosure; the recap is one line. Prototype controls float outside the design. | `P1` |
| R1-skipped draws Google events | R1-skipped draws no Google Calendar event and says why under the heading. | `R1-skipped` |
| "Problem set 6" closed and live | They are two courses' problem sets (CHEM 231 closed; BIO 201 live on Friday). Week lists now print the course under every due item. | `T-week`, `T-late` |
| G1 teaches clicking past "unverified app" | The warning is designed out. Gmail permission is offered only when Knowlu's Google app is verified, or to listed testers told in advance. Everyone else gets G1-fwd (Calendar by permission, Gmail by forwarding). G2 no longer mentions Advanced. | `G1`, `G2`, `G1-fwd` |
| 18+ checkbox on A1 | Removed. A1 has one terms line. Minors are open (see 10.4). | `A1` |
| X1 implies the trial runs while nothing happens | X1 names the steps and minutes left from the same table as the top bar. Before payment it says nothing is charged. After payment it says Knowlu reads what's connected each time it opens. | `X1` |
| T-undo "Redo" | "Mark done again". | `T-undo` |

### 10.3 Craft

- Bordered "secure" cards are now one rule and a lock line. A1 has none.
- The step numerals and the wordmark plate are gone. The wordmark sits on the ground, with a
  reversed asset (`assets/Knowlu_Wordmark_Red_reverse.png`, letters outside the red block recoloured
  to paper white) for dark.
- Palette: warm paper and warm charcoal, with signage green in place of pale blue on navy. Public
  Sans replaces Hanken Grotesk. Atkinson Hyperlegible was tried first and dropped, because its
  slashed zero made every course code read as monospace.
- Grid: it runs 8 am to midnight, so 11:59 pm work has a mark. Blocks use short labels (Library
  desk, Pre-Health, Soccer, Family call), with the full title in the label and popover. Blocks are
  clipped to the grid, and due items are sorted by time.
- Every onboarding sign keeps its primary action, and its skip link with the cost, above the fold
  at 1280x800 (checked on S1, G1, H1-zybooks, K and P1).
- W0 makes a promise ("Every morning, Knowlu tells you what's next.") and doesn't explain the UI.

### 10.4 Still open for Quinn

1. Minors: A1 no longer asks for an age. Whether students under 18 can hold an account is open
   (syllabus spec §12).
2. Pell: what Knowlu keeps from the award letter (cloud spec amendment 2026-10-16, open item).
3. The Google gate: offering Gmail permission only after verification, or to listed testers, means
   most pilot students get forwarding. Is that the intended trade?
4. §9.4 items 1 and 4 still stand (the morning pass; keyboard accelerators).

## 11. Round 2 (2026-10-23): structure fixes from the comparison README

- **Main list titles no longer truncate at 1280 px.** The reading column widened from 40rem to
  47rem (ambient column 16rem, gap 48px), and a long title now wraps onto a second line with its
  course code kept whole. Nothing on Today uses an ellipsis any more.
- **Today is about 1,400 px tall instead of about 2,600 px**, cut from seven regions below the list to
  three: What changed, Needs you, and one closing pointer. Good to know merged into What changed (the
  duplicate Lab 4 grade line went). Where you stand became "Your classes" in the ambient column (one
  line each with its ring; clicking still opens the grade breakdown). Coming up became one exam line
  plus the heaviest-day sentence in the ambient column. Closed this week and Late window closed are
  now two plain lines; the closed list opens only on "Review or undo" (its `hidden` was being
  overridden by `.lines{display:grid}`, so it used to render open).
- **No morning gate.** What changed folds under what's next and the list, as one summary line
  ("since 6:00 pm yesterday · 5 things, 1 you can undo · Show"). `T0` is now Today on the first open
  of a morning with that fold open; `T0-q` is Today scrolled to Needs you. The headline stays
  "Lab 6 is next."
- **Reveal grid (`R1`, and the Week page).** A "You are here: Sunday, October 18, 7:19 pm. The week
  starts tomorrow." line sits above the grid and Monday carries a "Tomorrow" tag; on the Week page,
  Monday carries "Today", a tinted column and a now line at 8:15 am. Optional blocks carry the word
  "optional" on their edge, and the legend reads "Optional: never counted as busy". The heaviest
  day's load bar is ink, never green; green stays reserved for actions and "now".
- Screens re-shot: T, T0, T0-q, T-task, T-undo, T-grades, T-avoided, T-offline, T-broken,
  T-allclear, T-empty, T-capture, T-week, R1, R1-late, R1-cap, R1-skipped, and dark T and R1.
