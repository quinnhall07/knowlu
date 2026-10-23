# Direction 4: Connections hub, one sheet per source (`connector-hub`)

Brief for the 2026-10-23 five-direction run. Written 2026-10-23. **Third reference: Plaid Link.**
Every direction starts from Notion and Stripe; this one adds Plaid Link and does not share a third
reference with any other direction.

Sources this brief obeys, in order: `docs/specs/2026-10-23-onboarding-redesign-draft.md` (ruled,
"the spec"), `VISION.md`, `docs/design/PRODUCT-DECISIONS.md`, `docs/design/DESIGN-PRINCIPLES.md`,
`docs/design/BRAND-VOICE.md`, `docs/surface/anatomy.md`, `../research/product-truth.md`. Sample data
is `../shared/sample-student.json` (Maya Okafor, Halverson State University; invented). Every name,
school, host, room and grade below is invented except the real hosts of real services (Google,
Stripe, zyBooks, VHL), which are shown because a real host in the window chrome is the point.

Research files are cited as `notion.md §n`, `stripe.md §n`, `onboarding-evidence.md §n` (all in
`../research/`). Plaid pages read on 2026-10-23 are cited by URL in §7.

> **Read §8 first.** Two red-team passes found rule breaks and problems in §1-§6. §8 records how
> each was resolved, and the prototype (`index.html`, screen list in `screens.json`) follows §8.
> Where §8 and an earlier section disagree, §8 wins.

---

## 0. The direction in one paragraph

**What Knowlu reads becomes the visible structure.** Onboarding is a board of source tiles, one per
place Knowlu will read, in the ruled order. The student sees the whole journey on the first screen,
but only the next tile is lit; every later tile shows its benefit line and the step it comes after.
Each lit tile opens one Plaid-style sheet over the board: what you'll need, then the place's own
window with its real host shown, then a success state whose proof is written onto the tile as the
sheet closes. The board is drawn from what is still outstanding (Stripe's requirements model), so
quitting and relaunching shows the same board with the first gap lit. The last tile is "Your week";
confirming it starts the reveal. After the reveal the board does not disappear: it becomes the quiet
**Connections** panel of the main page, and the main page itself is **a ranked list plus an
inspector** in which every row carries its receipt (why it's here, where it came from, when it was
read, who changed what, Undo).

**What this direction refuses.** The category default for long setup is a linear wizard with a
step counter ("Step 4 of 11"). This direction refuses the counter as the organising idea: the
student's progress is counted in places connected and things found, and the order is shown as a
path of tiles rather than hidden behind Next.

**Two named risks, and how this brief answers them** (detail in §1.3, §3.1 and §6.4):
1. *A visible board invites out-of-order clicks; locked tiles can read as a paywall preview.* Only
   the lit tile is a button. Later tiles are read-only and say what they come after, in words, never
   with a padlock. The price is printed on the Payment tile from the first screen, so nothing after
   it looks like a teaser.
2. *List plus inspector leans toward operator density.* The inspector is closed by default; when it
   opens it **replaces** the ambient column instead of adding a third. The page never shows more
   than two columns of content, and the list keeps a 60-75 character measure.

---

## 1. Flow map

### 1.1 Screen ids

`W` welcome, `H` the hub (board), `S-*` a sheet over the hub, `P-*` a pane inside a sheet, `E-*` an
error pane, `R-*` the reveal, `M-*` the main page. Sheets and panes are listed in the order a
student meets them. Every transition names its trigger.

| Id | Screen | Shown when | Leaves by (trigger -> next) |
|---|---|---|---|
| `BOOT` | Launch router (not visible; under 300 ms) | every launch | reads `onboarding.json`: no file -> `W`; file with gaps -> `H` (first gap lit) with the return line (§1.4); onboarding finished -> `M-TODAY` |
| `W` | Welcome: the board shown whole, tiles unlit except Installed (done) and Account (lit) | first launch | "Start with your account" -> `S-ACC` |
| `H` | The hub: tiles in ruled order, one lit | between sheets | lit tile or its button -> that tile's sheet; a done tile -> its receipt (read-only, `S-RECEIPT`); a skipped tile -> its sheet at `P-*-NEED` ("Connect now"); "Finish later" -> `P-EXIT` |
| `S-ACC` | Account sheet | Account lit | "Continue with Google" -> `P-ACC-BROWSER`; "Email me a code" -> `P-ACC-CODE` |
| `P-ACC-BROWSER` | "Finish in your browser" (host `accounts.google.com` shown) | Google chosen | sign-in returns -> `P-ACC-DONE`; browser tab closed or 10 min idle -> back to `S-ACC` with the line "The Google sign-in didn't finish. Nothing was saved." |
| `P-ACC-CODE` | Six-digit code entry | code chosen | correct code -> `P-ACC-DONE`; wrong code -> inline "That code doesn't match. Check the newest email from Knowlu, or send a new code." |
| `P-ACC-DONE` | Success: proof written | account made | 1.2 s after the checkmark, or Enter -> sheet folds onto the Account tile -> `H` (School lit). Background: vault folder created with defaults |
| `S-SCH` | School sheet | School lit | opens at `P-SCH-PICK`, or at `P-SCH-NEED` when the school is already known (account email's domain, or a previous attempt) |
| `P-SCH-PICK` | Pick your school (search, curated list) | no school known | a school -> `P-SCH-NEED`; "My school isn't here" -> `P-SCH-FEED` |
| `P-SCH-NEED` | What you'll need + what Knowlu reads (one merged pane) | school picked | "Sign in at Halverson State" -> `P-SCH-WINDOW`; "Use a different school" -> `P-SCH-PICK` |
| `P-SCH-WINDOW` | The school's own window is open beside the app; the sheet shows "Signing in at login.halverson.example" and "Approve the Duo push on your phone" | button pressed | session captured -> `P-SCH-READING`; window closed early -> `E-SCH-CLOSED`; Duo push unanswered (school page shows its timeout, or 3 min with no session) -> `E-SCH-DUO`; school page reports a lockout -> `E-SCH-LOCKED` |
| `E-SCH-CLOSED` | Window closed before sign-in finished | as above | "Try again" -> `P-SCH-WINDOW`; "Finish later" -> `P-EXIT` |
| `E-SCH-DUO` | Duo didn't get an answer | as above | "Try again" -> `P-SCH-WINDOW`; "Finish later" -> `P-EXIT` |
| `E-SCH-LOCKED` | The school says the account is locked | as above | "Finish later" -> `P-EXIT`; "Use the course feed link instead" -> `P-SCH-FEED` |
| `P-SCH-FEED` | Fallback: paste the Blackboard calendar feed link, with its cost named | no curated school, or the student chooses it after a failure | valid link -> `P-SCH-READING` (fewer rows); invalid -> inline "That isn't a calendar feed link. It starts with https and ends in .ics." |
| `P-SCH-READING` | Signed in; proof rows filling one by one ("reading...") | session captured | "Continue to payment" (enabled at once; reading continues on the tile) -> sheet folds onto School tile -> `H` (Payment lit) |
| `S-PAY` | Payment sheet: the order summary is the School tile's proof | Payment lit | "Continue to checkout" -> `P-PAY-BROWSER`; tester -> `P-PAY-TESTER` |
| `P-PAY-BROWSER` | "Finish in your browser" (host `checkout.stripe.com`) | checkout opened | trial started -> `P-PAY-DONE`; checkout closed without paying -> `S-PAY` with "Checkout closed before the trial started. Everything Knowlu found stays here." |
| `P-PAY-TESTER` | "Nothing to pay: you're a tester." | tester entitlement | "Continue" -> `P-PAY-DONE` |
| `P-PAY-DONE` | Proof: "Your trial is on. First charge October 25." | as above | folds onto tile -> `H` (School email lit) |
| `S-MAIL` | School email sheet | School email lit | covered by the school sign-in -> opens at `P-MAIL-COVERED`; not covered -> `P-MAIL-NEED` (one sign-in, same pane pattern as school) |
| `P-MAIL-COVERED` | "Nothing new to sign in" | covered | "Turn on school email" -> `P-MAIL-READING`; "Skip school email" -> `P-SKIP` |
| `P-MAIL-READING` | Test message sent; "usually under a minute" | turned on | "Continue" (enabled at once) -> `H` (Google lit); test arrives -> proof on tile; no test after 3 min -> tile `needs you` with `E-MAIL-CANARY` text |
| `S-GOO` | Google sheet (Gmail and Calendar, one consent) | Google lit | opens at `P-GOO-NEED` |
| `P-GOO-NEED` | What Google will show you (the unverified-app warning, then two boxes), what Knowlu reads | sheet open | "Continue with Google" -> `P-GOO-BROWSER`; "Skip Google" -> `P-SKIP` |
| `P-GOO-BROWSER` | "Finish in your browser" (host `accounts.google.com`) | consent opened | both scopes granted -> `P-GOO-READING`; Gmail only -> `P-GOO-PARTIAL`; Calendar only -> `P-GOO-PARTIAL`; declined, or Gmail read fails -> `P-GOO-FWD` |
| `P-GOO-PARTIAL` | Tile splits by scope: "Gmail connected. Calendar not shared." | partial grant | "Share calendar" -> `P-GOO-BROWSER` (asks only the missing scope); "Continue without it" -> `H`, and `S-CAL` is added to the board (Another calendar) only when no calendar was granted |
| `P-GOO-FWD` | Fallback: Gmail forwarding, about 3 minutes | declined, failed, or past the first 100 permission users | forwarding verified -> proof; "Skip Gmail" -> `P-SKIP` |
| `P-GOO-READING` | Proof rows fill | granted | "Continue" -> `H` |
| `S-CAL` | Another calendar (iCal link) | only when Google granted no calendar | valid link -> proof; "Skip" -> `P-SKIP` |
| `S-ZY`, `S-VHL` | One sheet per homework site found mentioned (zyBooks, VHL) | each lit in turn | `P-HW-NEED` -> credentials in the enclosed local panel -> `P-HW-READING`; rejected -> `E-HW-REJECTED`; "Skip VHL" -> `P-SKIP` |
| `S-SYL` | Syllabi: only courses with none found | Syllabi lit | "Choose a file" for CS 110 -> `P-SYL-READING`; "Skip CS 110" -> `P-SKIP` (per course); nothing missing -> tile goes straight to done, sheet never opens |
| `S-WEEK` | Your week: prefilled rows to confirm (the widest sheet, nearly full window) | Your week lit | "Confirm your week" -> `R-WAIT` |
| `P-SKIP` | Skip confirmation pane (cost named, one tap) | any "Skip" | "Skip for now" -> tile `skipped`, `H` with next tile lit; "Back" -> the pane it came from |
| `P-EXIT` | Finish later pane | "Finish later" anywhere | "Close Knowlu" -> app exits, state saved; "Keep going" -> where it came from |
| `S-RECEIPT` | A done tile's receipt (read-only during onboarding) | click a done tile | Esc or "Close" -> `H` |
| `R-WAIT` | The board narrates the first full run, countable | Your week confirmed | everything ran once -> `R-REV`; 180 s cap reached -> `R-REV` with "still running" lines |
| `R-REV` | "Here's your week" | from `R-WAIT` | "Open Knowlu" -> `M-TODAY` (scheduler and start-with-Windows turn on here) |
| `M-TODAY` | Main page: list (left) + ambient column or inspector (right) | daily | row select -> `M-INSPECT`; Connections -> `M-CONN` |
| `M-INSPECT` | Inspector open on a row, domain, decision or notice | selection | Esc / close -> `M-TODAY` |
| `M-CONN` | Connections panel (the hub, compact) in the right column | rail entry or a health line | a `needs you` source -> `S-REPAIR` |
| `S-REPAIR` | Abbreviated re-sign-in sheet (one source, minimum input) | "Sign in again" | success -> `M-CONN` with proof updated |

### 1.2 The board, and the five tile states

> Superseded in part by §9 (second pass): tiles carry a source icon and a round status badge, not a key tag; later tiles are drawn with a small lock and are clickable.

The board for Maya holds ten tiles, in this order: **Installed · Account · School · Payment · School
email · Google · zyBooks · VHL · Syllabi · Your week**. "Another calendar" appears between Google and
zyBooks only when Google granted no calendar. zyBooks and VHL appear because the School step found
them mentioned in syllabi; a student with no homework site mentioned has no homework tiles at all.
The tiles are drawn from `onboarding.json`'s outstanding list, never from a fixed screen count
(`stripe.md §1.1`).

Every tile carries exactly one state. Each state has a word and a shape, so colour is never the only
signal (principle 10). The shape sits in the tile's key tag (top-left); the word sits under the
tile's name.

| State | Word on the tile | Tag shape | Meaning | Clickable? |
|---|---|---|---|---|
| **Needed now** | "Next" | filled tag with a forward notch | the one lit tile; its sheet is the next ask | yes: opens its sheet |
| **Reading** | "Reading... usually 1-3 min" | tag with diagonal hatching (moves slowly; static under reduced motion) | the ask is answered, background work is running; the student may move on | yes: opens the receipt with rows filling |
| **Done with proof** | the proof itself ("5 classes · 26 assignments") | solid tag with a check | finished; the proof is the label | yes: opens the receipt |
| **Needs you** | the cause, then the fix ("Duo didn't get an answer. Try again") | hollow ring tag | something only the student can do; never shown for waiting | yes: opens the sheet at the failed pane |
| **Skipped** | "Skipped: [cost]" ("Skipped: VHL due dates missing") | dashed-outline tag | the student chose to skip; cost named | yes: opens the sheet with "Connect now" |

Tiles not yet reached are **needed, later**: the same tile at rest, tag drawn as a plain outline, and
the word names what it comes after ("After payment", "After Google"). This is not a sixth state; it
is "needed" before its turn. It is never drawn with a padlock, never greyed below 4.5:1, and never
blurred.

### 1.3 Order is kept by the board, not by locks

> Superseded by §9.1 (second pass): before payment the order is fixed; after payment every remaining tile is open at once, in any order.

- Only the lit tile is a button. Later tiles are `aria-disabled` list items with their benefit line
  and "After ..." word; focusing one by keyboard reads "zyBooks. Due dates and progress Blackboard
  doesn't show. Comes after Google." Clicking one does nothing but a 150 ms nudge of the lit tile
  (motion that says "this one first"); no toast, no error.
- Done and skipped tiles are always open: a done tile opens its receipt; a skipped tile can be
  connected at any time, because skipping never changes the order of what is still ahead.
- The "After ..." words are honest dependencies. Payment comes "After your school" because the price
  is asked after the proof (ruled Q2). Everything after payment says "After payment" because the
  engine reads none of it until the trial is on (`engine/src/entitle.rs`). Your week says "Last".
- **Why this is not a paywall preview:** the Payment tile shows "$9.99 a month after 7 days free"
  from the welcome screen on, and the Account sheet says the same before any school sign-in
  (`onboarding-evidence.md §3.5`, up-front disclosure). A later tile previews what the student will
  connect, not what money unlocks.

### 1.4 Resume after quit

The hub renders from what is outstanding, so resume is the same board (`stripe.md §1.1`, §1.4).

1. `BOOT` reads `onboarding.json`: current step, each step's answer, each background job's state.
2. `H` opens with finished tiles carrying their proof, unfinished background jobs back in **Reading**
   (re-run, idempotent), and the first gap lit.
3. One return line sits above the board for this launch only, naming the open loop in the student's
   terms (`onboarding-evidence.md §1.5`):
   - left at payment: "You're on Payment. Your school is connected: 5 classes, 26 assignments."
   - left during the school window: "You're on your school sign-in. Nothing was saved from the last
     try."
   - left at Your week with rows confirmed: "You're on Your week. 9 rows are confirmed; 23 to go."
4. If the kept school session expired while Knowlu was closed, the School tile is **Needs you**:
   "Halverson State signed you out while Knowlu was closed. Sign in again; everything found so far
   stays." Its sheet opens at `P-SCH-NEED` with the school preselected (Plaid's update mode: only the
   minimum input to repair).
5. A student who left at payment returns to payment; the entitlement is re-checked first (spec §1.6).
   Confirmed Your week rows are never lost (`onboarding-evidence.md §1.3`, the IKEA effect only pays
   when the work lands).

### 1.5 Skip paths, each with its cost named

Skipping always goes through `P-SKIP`: one pane, the cost in one sentence, two buttons ("Skip for
now", "Back"). The tile then reads "Skipped: [short cost]", and the reveal repeats the cost once.
Nothing skipped is asked again in a nudge (spec §1.7).

| Tile | Skippable | `P-SKIP` text | Tile word after skipping |
|---|---|---|---|
| Account, Payment, Your week | No | none (no Skip button exists) | none |
| School | No; the feed link is the way around | `P-SCH-FEED`: "With the feed link, Knowlu gets your assignments and due dates, but not your grades, class times from the registrar, your syllabi, or what you've already turned in. School email then needs its own sign-in." | "Feed link only" (done, with that cost in its receipt) |
| School email | Yes | "Without school email, date changes and new work your professors announce by email are missed." | "Skipped: email date changes missed" |
| Google | Yes | "Without Google, club, job and internship tasks from Gmail are missed, and your free time ignores your own events." | "Skipped: Gmail and your events missing" |
| Another calendar | Yes | "Without a calendar, your free time ignores your own events." | "Skipped: your events missing" |
| zyBooks / VHL | Yes, per site | "Without VHL, its due dates are missing unless Blackboard's feed carries them." | "Skipped: VHL due dates missing" |
| Syllabi | Yes, per course | "Without CS 110's syllabus, Knowlu has no exam dates beyond the feed, no grade weights and no late rule for CS 110." | "4 of 5 · CS 110 skipped" |

### 1.6 A failed login, and Duo

Built from Plaid's error object (a code for logic, a display message for people) and the Duo
evidence (`onboarding-evidence.md §4.1`: each push adds about 8 s; 4.35% of attempts fail because
the Duo step wasn't completed). Rules:

- `P-SCH-NEED` says "Have your phone ready for Duo" **before** the window opens.
- The password is typed only on the school's own page. Knowlu never draws a field for it.
- Knowlu detects three endings it can see (window closed, no session after the Duo timeout, the
  school's lockout page) and turns each into cause + fix, never "failed", never blaming.
- Wrong passwords are the school page's business; it shows its own message and the student retries
  there. Knowlu shows nothing over it.
- Every error pane carries a small support code in mono under the fix (`stripe.md §1.7`).

| Pane | Headline | Body | Buttons | Code |
|---|---|---|---|---|
| `E-SCH-CLOSED` | The sign-in window closed. | Nothing was saved. Halverson State's page opens again where you left it. | Try again · Finish later | `SCH-WINDOW-CLOSED` |
| `E-SCH-DUO` | Duo didn't get an answer from your phone. | Duo stops waiting after about a minute. With your phone unlocked and nearby, try again. | Try again · Finish later | `SCH-MFA-TIMEOUT` |
| `E-SCH-LOCKED` | Halverson State says your account is locked. | Your school's IT help desk can unlock it. Knowlu keeps your place here; everything else waits. | Finish later · Use the course feed link instead | `SCH-LOCKED` |
| `E-HW-REJECTED` | VHL didn't accept the sign-in. | Try again, or skip VHL for now; its due dates will be missing until it's connected. | Try again · Skip VHL | `VHL-AUTH-REJECTED` |
| `E-GOO-READ` | Gmail's read didn't work. | Knowlu can bring Gmail in by forwarding instead, about 3 minutes. | Set up forwarding · Skip Gmail | `GMAIL-READ-FAILED` |
| `E-MAIL-CANARY` | The test message hasn't arrived yet. | It usually takes under a minute. Knowlu keeps checking; you can continue. | Continue · Send another | `MAIL-CANARY-LATE` |

The codes are this brief's proposal for support strings, not engine contracts.

### 1.7 A slow first run

Background work starts the moment its ask is answered and never blocks the next tile (spec §1.5).

- A **Reading** tile shows its expected time ("usually 1-3 min") from the moment it starts
  (`stripe.md §1.8`).
- Past its expected time, the word changes once, calmly, and names the part still running: "Still
  reading grades. Early in a term this can take longer; you can keep going." It never becomes a
  spinner and never turns into **Needs you** unless the student has something to do.
- A source that fails in the background moves to **Needs you** with cause + fix, and the board's
  header gains one line: "1 thing needs you: VHL". The lit tile stays lit; the student finishes in
  any order they like among the needs-you tiles and the lit one.

### 1.8 The reveal wait (`R-WAIT`)

After "Confirm your week" the sheet folds away and the board itself becomes the wait screen
(labor illusion with true progress, `onboarding-evidence.md §1.4`, §3.9):

- Header: "Putting your week together. 7 of 9 done." (a count, not a percent, not a spinner).
- Each Reading tile narrates its real engine step in one line: "Syllabi: reading CS 110's
  syllabus... found the late rule." "Effort and order: estimating 34 assignments." Lines come from
  the engine's step records, never a scripted ticker.
- The cap is 180 s. When everything has run once (Maya: 40 s, when CS 110's syllabus finished),
  `R-REV` opens at once. At the cap it opens anyway and the still-running tiles carry into the
  reveal's "Still coming" lines. Backfills (older school mail) are never waited for.

---

## 2. Copy: every step's benefit line and proof line

Rules applied (BRAND-VOICE): lead with what the student gets; numbers over adjectives; sentence
case; contractions; "about" for estimates; times as "6:02 am"; no "just", "easy", "simply",
"don't worry", no exclamation marks, no emoji. The **why** lines are the spec's wording, which Quinn
reads as final copy; the **tile line** is the short benefit printed on the board, written to fit one
line at tile width (about 48 characters). Each sheet's first pane follows Plaid's messaging
guidance: why, then at most two "you'll need" items, then what Knowlu reads, then where the secret
goes, then one button that names the action (`onboarding-evidence.md §2.4`; Plaid messaging,
"two bullets is best").

### 2.1 Welcome (`W`)

- Heading: **Knowlu answers one question every morning: what's next?**
- Line: "Next, you'll connect the places that already know your work, about 10 minutes. Here they
  are, in order."
- Board header count: "1 of 10 done: Knowlu is installed." (endowed progress with a true reason,
  `onboarding-evidence.md §1.1`)
- Price line, under the board: "Knowlu is $9.99 a month after 7 days free. There's no free tier.
  You'll see what Knowlu finds before you pay."
- Button: **Start with your account**

### 2.2 Per tile

| Tile | Tile line (on the board) | Sheet why line (spec) | You'll need | What Knowlu reads (above the button) | Where the secret goes | Button | Proof on the tile | Proof in the receipt |
|---|---|---|---|---|---|---|---|---|
| **Installed** | Knowlu is on this computer. | none | none | none | none | none | "Installed" | "Knowlu 0.x on this computer. Your plan will be plain files in a Knowlu folder here." |
| **Account** | Keeps your plan safe if this computer isn't. | Your plan lives in your account, so it's safe if this computer isn't. | A Google account or any email address. | Your name and email address. Nothing else from Google at this step. | "Knowlu asks Google for your name and email only." | Continue with Google · Email me a code | "Signed in as maya.okafor.demo@gmail.example" | adds: "A Knowlu folder was made on this computer. Show me the folder" |
| **School** | Classes, times, work, grades and school email, from one sign-in. | One school sign-in lets Knowlu read your classes, class times, assignments, grades and school email. You won't sign in to these again. | Your Halverson State email and password. Your phone, for Duo. | Your classes this term and when they meet · assignments and what you've turned in · grades · syllabi · school email | "You sign in on Halverson State's own page. Knowlu never sees or keeps your password; it keeps the signed-in session on this computer." | Sign in at Halverson State | "5 classes · 26 assignments, 4 turned in" | "5 classes this term (2 from last term left out) · class times for 5 · 26 assignments, 4 already turned in · grades for 5 courses · syllabi found for 4" |
| **Payment** | $9.99 a month after 7 days free. | Everything you just connected keeps running every day for $9.99 a month, after 7 days free. | A card, in Stripe's checkout. | none (order summary instead, §2.3) | "Your card goes to Stripe's checkout page. Knowlu never sees the number." | Continue to checkout | "Trial on · first charge October 25" | "Your trial is on. First charge $9.99 on October 25. Cancel any time in Settings." |
| **School email** | Date changes from professors, for you to approve. | Professors change dates by email. Knowlu reads what arrives and proposes each change for you to approve. | Nothing new. Your school sign-in covers it. | New mail to your school address. Knowlu proposes changes; it never sends mail for you. | "Covered by your school sign-in. Nothing new to type." | Turn on school email | "Forwarding on · test arrived in 38 s" | "Forwarding on. Test message arrived in 38 s. New mail is read as it arrives; older school mail is still coming in." |
| **Google** | Club, job and internship mail, and when you're busy. | Clubs, jobs and internships write to your Gmail, and your calendar says when you're already busy. One Google permission covers both, read-only. | About 30 seconds in Google's window. | Gmail, read-only, from the last 7 days on · your Google Calendar events, read-only | "Read-only. Knowlu can't send, delete or change anything in Gmail or Calendar." | Continue with Google | "40 messages read, 1 to do · 6 events this week" | "Read 40 recent messages; 1 thing to do (Pre-Health dues, added). 6 events this week, 3 repeating ones to check on Your week." |
| **Another calendar** (conditional) | Your busy times, if your calendar isn't Google's. | If your calendar isn't Google's, add it so your free time is honest. | Your calendar's iCal link. | Event times and titles, read-only. | "Knowlu keeps the calendar link, never your password." | Add calendar | "N events this week" | same, with the calendar's name |
| **zyBooks** | Due dates and progress Blackboard doesn't show. | Homework sites hold due dates your school's feed doesn't show. | Your zyBooks email and password. | Your zyBooks books, assignments, due dates and how much of each is done. | "Stored only on this computer, in Windows Credential Manager. It never goes to Knowlu's servers." | Sign in to zyBooks | "1 book · 8 assignments, 3 done" | "zyBooks: 1 book (Programming in Python 3, CS 110), 8 assignments, 3 already done" |
| **VHL** | Due dates and progress Blackboard doesn't show. | (same as zyBooks) | Your VHL Central email and password. | Your VHL course, assignments, due dates and scores so far. | (same as zyBooks) | Sign in to VHL | "1 course · 6 assignments, 2 done" | "VHL: 1 course (SPAN 202 online practice), 6 assignments, 2 already done" |
| **Syllabi** | Exam dates, grade weights and late rules. | Your syllabus has every exam date, how each course is graded, and the late-work rules. | CS 110's syllabus as a file (PDF or Word). | Exam dates, grade weights and late-work rules. | "The file stays in your Knowlu folder." | Choose a file | "5 of 5 read · 10 exams" | "Read 5 of 5: 10 exams, weights for 5, late rules for 4 (ENG 210's syllabus has none)" |
| **Your week** | Check your classes. Then your week opens. | Check your classes. Everything here can be changed later. | About 2 minutes. | (the rows themselves, §2.4) | none | Confirm your week | "Confirmed" | "7 classes, 10 exams, 5 grading rules, 4 late rules and 3 commitments confirmed" |

Proof lines are only ever written from real counts. A count that is still filling shows as
"reading..." in its row; a tile never claims a number the engine hasn't produced (first install
feedback #3, `product-truth.md §5`).

### 2.3 Payment's order summary (`S-PAY`)

Stripe Checkout's transparent order summary, where the summary is the student's own proof
(`stripe.md §3.1`):

> **Here's what Knowlu found at Halverson State**
> 5 classes this term · 26 assignments, 4 already turned in · grades for 5 courses · syllabi for 4
>
> **Knowlu keeps all of it running every day.**
> $9.99 a month after 7 days free. First charge October 25. Cancel any time in Settings.
> You pay Knowlu, so Knowlu doesn't sell your data.
>
> Other ways to pay: $39.99 for a 6-month term · Pell Grant recipients pay 75% less
>
> [Continue to checkout]

### 2.4 Your week (`S-WEEK`)

Prefill, then confirm (`stripe.md §1.5`). Rows are grouped; each group has "Confirm all" and each row
has Confirm and Edit. Typing is the last resort, reached from Edit only. Sources are named per row in
mono ("registrar", "syllabus", "Google Calendar").

- Heading: **Check your classes.** Line: "Everything here can be changed later."
- Groups and counts for Maya: Classes (7 meetings for 5 courses) · Exams (10) · Grading (5) · Late
  work (5 rows; ENG 210 "No rule found. Leave blank") · Your week (3 repeating events, each with kind
  and level: "Marsh Library circulation desk: Tuesday 5:00-9:00 pm, Saturday 10:00 am-2:00 pm · work ·
  never overlapped") · Office hours (2, optional) · Your day ("Awake 7:30 am to 11:30 pm").
- Sample row: "BIO 201 lab · Thursday 2:00-4:50 pm · Kessler Hall 310 · registrar · [Confirm] [Edit]"
- Footer count: "33 of 33 rows answered" and the button **Confirm your week**. A blank row is
  allowed: "Left blank rows come back as a question over the next few days."

### 2.5 Skip, exit and return copy

- `P-SKIP` buttons: **Skip for now** · **Back**. The cost sentence is §1.5's.
- `P-EXIT`: "Knowlu keeps your place. Next time it opens here, with everything found so far." Buttons:
  **Close Knowlu** · **Keep going**. When background work is running: "Reading continues the next
  time Knowlu is open."
- Return lines: §1.4.

### 2.6 The reveal (`R-REV`)

- Heading: **Here's your week.**
- What's next (with its receipt open beside it, §3.4): "Reading: Ch. 8, Metabolism is next. BIO 201,
  due tomorrow at 9:00 am, about 30 minutes." Why: "Due tomorrow at 9:00 am, before BIO 201. It's
  the first thing due this week."
- Free time: "About 3h free tonight, after 9:00 pm."
- What Knowlu tidied, one line each: "52 past items and 9 already turned in were archived." "Effort
  estimated for 34 assignments." "2 classes from last term left out."
- From your email: "Pay Pre-Health Society dues ($15), due Friday, October 30 at 5:00 pm. Undo"
- Still coming: "Older school email is still coming in; anything it finds will be proposed."
- Connected line (the folded board): "9 of 9 connected." With a skip: "8 of 9 connected · VHL
  skipped: its due dates are missing. Connect"
- Hand-off to habit (`onboarding-evidence.md §3.8`): "Knowlu checks again at 12:00 pm and 6:00 pm.
  Your list opens on Reading: Ch. 8."
- Button: **Open Knowlu**

---

## 3. The main page: one ranked list, and an inspector that holds each row's receipt

Moment: Monday October 26, 8:15 am, term week 10 (`sample-student.json` `today`). Window 1280 x 800.

### 3.1 Layout

Three bands, never more than two columns of content:

| Band | Width at 1280 | What it is | Visibility level |
|---|---|---|---|
| **Rail** (left) | 220 px open, 56 px collapsed (Ctrl+\\ or the rail's own button) | Today · Week · the domains (filters) · Connections · Settings; health line at its foot | ambient |
| **List** (centre) | about 620 px; text measure 60-75 characters | the verdict, the meter, the one ranked list | foreground |
| **Right column** | about 400 px | **either** the ambient day column (default) **or** the inspector (when something is selected) | ambient / on request |

The inspector **replaces** the day column; it never stacks beside it. Esc closes it and the day
column returns. Below 1100 px the rail collapses on its own; below 900 px the right column becomes a
sheet over the list. This is the answer to "operator density": at rest the page is a list with a
quiet column beside it, the same weight as a document.

### 3.2 The list (centre)

Top to bottom, from anatomy regions (names in mono are read-model fields):

1. **Delta line** (`delta`), one line, only the changes since the last run: "Since 6:00 pm yesterday:
   1 marked done by Knowlu · Lab 4 report graded 44/50 · 1 new question · VHL needs you". Each item
   opens its receipt in the inspector.
2. **Verdict** (`verdict`): headline "About 4.7h is due by tomorrow, and today has room for it." Sub:
   "Lab 6 is next. After today's work, about 6h is yours, after 6:00 pm." Never a fixed string; never
   disagrees with the meter.
3. **Meter** (`meter`): 30-minute blocks for today's planned 6.5h against capacity. Blocks that fit
   are solid course tints; blocks that would spill are drawn outside the frame as hatched outlines in
   the ink colour, never red. Hover or focus names the task each block belongs to.
4. **The list** (`must_do` + `recommended`), one ordered list with two quiet group labels, each with
   a count and hours: "Due by tomorrow · 5 · 4.7h" and "Recommended today · 1 · 1.8h". No start-by
   dates, ever; every row shows its due time.
   - **Row anatomy** (one line, 44 px): course tint swatch + code ("CS 110") · title · due ("today at
     11:59 pm") in mono · estimate as an estimate ("about 24m") · progress as a thin bar when a source
     reports one · source mark (a small two-letter source tag: Bb, zy, VH, Gm, Kn). Hover or focus
     reveals row actions on the right: Done, Pin, Start focus, More. They stay reachable by Tab and by
     keys (`notion.md §2.4`).
   - **The first row is next.** It is the same row, taller (two lines), with its why line under the
     title: "Lab 6: Functions · CS 110 · was due Friday at 11:59 pm · about 1h 12m left" then
     "−30% now; closes Wednesday at 11:59 pm. Finishing today keeps it at −30%." This keeps D5's "next,
     then the rest" without a separate card.
   - **Steps are toggle children** (`notion.md §2.2`): "Study for Exam 2 (Ch. 5-7)" shows a disclosure
     triangle and "4 steps"; opening it shows "Practice exam 1 · 1h 30m", etc., one level only.
     Checking every step never completes the task; Knowlu asks "Is Study for Exam 2 finished?"
   - **Pinned rows** sit at the top of their group with a pin mark; no drag, no reorder.
   - **Done today** stays as quiet struck rows at the foot of the list for the day, uncheckable:
     "Reading: Ch. 9, Cellular respiration · done 7:55 am by you".
5. **Later** (`later`): collapsed to two lines, "Later this week · 13 · about 18h · Show" and "After
   this week · 8 · about 22.5h · Show". Reachable, checkable, out of the way.
6. **Late window closed** (`late_window_closed`): its own small group under Later, one line each:
   "Problem set 6 · CHEM 231 · closed Thursday · late work not accepted after 48 hours (your
   syllabus) · Archive · Still doing it". "Still doing it" asks "When will you turn it in?" in the
   inspector. Never alarm styling.
7. **Needs you** (`decisions`): one line in the list, "4 questions · oldest waiting 7 days · Answer",
   which opens the deck in the inspector one card at a time (anatomy §3.9: the page never grows with
   the queue). A decision tied to a task (`amend`) also marks that task's row with a small "change
   proposed" tag, so the student meets it where the task is.

Every region always renders (anatomy §1): an empty group collapses to its named empty line
("Queue clear", "Nothing is at risk today."), never to a hole and never to a "0" box.

### 3.3 The rail (left)

A quiet, collapsible rail on a slightly darker slate than the page (`notion.md §2.4`):

- **Today**, **Week** (the 14-day `ahead` view and the Schedule).
- **Domains**, one row each, the filter strip of VISION's main page: tint swatch · name · status
  shape + hours. "Organic Chemistry · behind about 2h", "Spanish · ahead about 1.5h", "Library job ·
  on track". Status uses a shape (behind: a downward notch; ahead: an upward notch; on track: a flat
  bar) plus the word, never colour alone and never red. Selecting a domain filters the list and opens
  the domain's receipt in the inspector: grade ring and breakdown (`grades`), pace sentence ("Organic
  Chemistry is about 2 hours behind. Today's study session and one Wednesday catch it up before
  Thursday's exam."), exams listed plainly, work left this week.
- **Connections** with a needs-you count when there is one: "Connections · 1 needs you".
- **Settings**.
- **Health line** at the foot (`topline`): "Up to date, 6:04 am · in step with your account". It is
  always present, so silence is never ambiguous; it grows only when the student must act (`stripe.md
  §2.1`): "VHL needs you · Sign in".

### 3.4 The inspector: every row's receipt

Opened by click, Enter, or selecting anything in the delta line. Stripe's Dashboard Inspector puts
detail on request (`stripe.md §2.2`); this one is organised as a receipt, top to bottom. For
**Lab 6: Functions**:

| Section | Content | Editable? |
|---|---|---|
| Title block | "Lab 6: Functions" · CS 110 · Programming | title and course: yes, in place |
| **Due** | "Was due Friday at 11:59 pm. −30% now; closes Wednesday at 11:59 pm." | due date and time: yes |
| **Why it's here** | "Past due since Friday. Each day late costs 10%; finishing today keeps it at −30%." | no (Knowlu's reasoning, visible) |
| **Where it came from** | "zyBooks, read 6:04 am. zyBooks shows 40% done." · "Late rule: CS 110 syllabus, confirmed by you on October 18." · Open in zyBooks | no |
| **Estimate** | "About 2h. About 1h 12m left." Knowlu's original estimate shown beside any change: "Knowlu guessed 2h; you set 1h 30m." | estimate: yes (one number); original: no |
| **Steps** | toggle children, checkable | yes |
| **History** (rolling, newest first, each with actor and time) | "Oct 26, 6:04 am · zyBooks · progress 30% -> 40%" · "Oct 18, 7:19 pm · Knowlu · estimated about 2h" · "Oct 18, 7:15 pm · zyBooks · added, due Friday at 11:59 pm". Each change Knowlu or a source made has **Undo** on its line | Undo only |
| Actions | Done · Pin · Start focus · Report a problem (fixed reasons) · Delete (asks first, optional reason) | |

For a task Knowlu marked done, the history line reads "Oct 25, 4:40 pm · Knowlu · marked done
(Blackboard shows it turned in) · Undo". For a decision, the inspector shows one card with its
evidence quoted: "Move the Literature review draft to Monday, November 2 at 11:59 pm? · From Dr.
Hannah Brooks: 'Re: Literature review extension' · waiting 2 days · Move it · Keep Friday · Not now".
For a notice ("CHEM 231 Exam 2 on Thursday is in Hallett Hall 120, not the usual room."), it shows
the quoted email and no action, because a notice changes nothing (VISION's three-way rule).

### 3.5 The ambient day column (right, default)

- **Today's schedule** (`the_day`): BIO 201 lecture 9:00-9:50 am, SPAN 202 10:00-10:50 am, CS 110
  1:00-2:15 pm, CHEM 231 office hours 3:30-4:30 pm (optional, dashed). Times in mono.
- **Free time** (`free_time`): "About 5.7h free, after 6:00 pm." One bar of the day with the open
  window drawn as space, no work assigned to it.
- **Ahead** (`ahead`, compressed): 14 small day bars with hours due and exam marks, and the generated
  takeaway: "Friday is the heaviest day in the next two weeks, about 10.2h due." Opens Week.
- **Good to know** (`good_to_know`): up to 3 one-line notices.
- **Closed this week** (`closed_this_week`): one line, "8 closed this week · 6 by Knowlu · Show",
  which opens the list in the inspector with actor and Undo per line.

### 3.6 Connections (`M-CONN`): the hub, kept

The same tiles as onboarding, compact (one row each), same five states and shapes (principle 13).
Each row: source · state word · last read in mono · action.

- "School sign-in · Blackboard, registrar, grades, school email · read 6:02 am · kept on this
  computer · Forget"
- "Gmail · read-only · read 6:03 am" · "Google Calendar · read 6:03 am" · "zyBooks · read 6:04 am"
- **"VHL · needs you · VHL signed you out this morning; earlier data stays. · Sign in again"** with
  the code `VHL-SESSION-EXPIRED` under it.
- "Sync · in step with your account" · "Backup · backed up"

"Sign in again" opens `S-REPAIR`: a sheet with only VHL's credential panel (Plaid update mode's
abbreviated flow). Several broken sources are repaired one at a time, most recently broken first.

### 3.7 All clear, and Lock in

- All clear (Sunday November 1): the verdict reads "Nothing is due today. The day is yours." The list
  shows its empty line and, quietly, "Next due: Monday, 2 tasks, about 1.5h." The day column keeps
  intramural soccer (optional) and the family call.
- Lock in starts from a row's Start focus or the verdict's "Start with Lab 6". It takes the list area
  over (the rail and the right column dim, not hidden), serving today's rows in order: Done, Skip,
  Pause, Exit; the pomodoro and the per-task clock are separate (PRODUCT-DECISIONS).

### 3.8 Keyboard (accelerators, every action keeps a visible control; log D4)

j / k move · Enter opens the inspector · Esc closes it · x done · p pin · f start focus · d open
Needs you · c Connections · ? shows the list. Shortcuts appear in tooltips, never as a legend.

### 3.9 The reveal's layout (`R-REV`)

The reveal is made of the main page's parts, so the student meets the inspector before Today:

- Top: the board folds into one strip of small tiles with their proof ("9 of 9 connected"), then
  stays as the strip until "Open Knowlu".
- Left: the week, Monday October 19 to Sunday October 25, seven columns. Class blocks as course-tint
  bands, due items as small marks at their due time, commitments as outlined bands (work, hard; club,
  soft, dashed edge; optional, dotted). Each block opens its own editor and returns (`stripe.md
  §1.6`).
- Right: the inspector, already open on **Reading: Ch. 8, Metabolism** with its receipt (why, source
  "Blackboard, read 7:08 pm", estimate "about 30 minutes", history "Knowlu · estimated"). This teaches
  the main page's one new gesture (select a row, read its receipt) on a real item.
- Under the week: free time, tidied lines, from your email (with Undo), still coming, the habit
  line, and the one button.

---

## 4. Visual world: a key cabinet

### 4.1 Scene and world

Physical scene: a student at a desk on a Sunday evening (setup, about 7 pm) and on weekday mornings
and late nights (daily use), laptop on battery, a desk lamp or overhead dorm light; Quinn works in
dark. Both modes are designed; the app follows Windows' setting.

The world: the **key cabinet** behind a residence-hall front desk. A slate-grey board, matte tags on
hooks in a fixed order, each tag stamped with what it opens; an empty hook is visible and says which
key belongs there. It carries the mechanism directly: each source is a key Knowlu holds (a kept
session, never a password), the order of hooks is the order of setup, and the board after setup is
the Connections panel. Material cues: matte card stock, a cut **notch** on the edge of a tile marking
a step where a password is entered (School, zyBooks, VHL only; reserved so it keeps its meaning,
`onboarding-evidence.md §3.4`), the **key tag** at each tile's corner holding its state shape, and
the stamped host line in mono. No skeuomorphic keys, rings, brass textures or photos: the cabinet is
a grammar (fixed order, labelled hooks, tags), not a costume.

### 4.2 Type

- **Schibsted Grotesk** (Google Fonts, 400 / 500 / 600 / 700) for every word people read: headings,
  rows, sentences, buttons.
- **Red Hat Mono** (Google Fonts, 400 / 500) only for data: hosts (`login.halverson.example`), times
  ("11:59 pm"), durations ("1h 12m"), last-read stamps ("read 6:02 am"), support codes. Never for
  labels or decoration (`craft-floor.md`: mono for data, not as a costume).
- Fixed rem scale, ratio 1.2, base 15 px at 1280: 12.5 (meta, never below 12) · 15 (rows, body) · 18
  (verdict sub, sheet why line) · 21.6 (sheet heading) · 26 (verdict headline, reveal heading).
  Weights do the hierarchy; no eyebrows above headings, no section numbers on screen.
- Reading measure 60-75 characters for the list and sheet text (`notion.md §3.2`).

### 4.3 Palette as tokens (OKLCH)

Colour strategy: **Restrained**, slate neutrals plus the brand red for the one primary action at a
time; course tints as soft fills with dark labels (`notion.md §3.3`). Steps follow Stripe's rule:
text and its ground at least five lightness levels apart, icons and large text four (`stripe.md
§4.1`). Red is the brand, never an alarm: it never marks behind, busy, late or failed.

| Token | Light | Dark | Job |
|---|---|---|---|
| `--ground` | `oklch(0.955 0.006 250)` | `oklch(0.200 0.012 255)` | page |
| `--cabinet` | `oklch(0.925 0.009 250)` | `oklch(0.170 0.012 255)` | hub board, rail, sheet backdrop |
| `--tile` | `oklch(0.990 0.003 250)` | `oklch(0.255 0.013 255)` | tiles, sheets, inspector |
| `--tile-edge` | `oklch(0.830 0.012 250)` | `oklch(0.350 0.014 255)` | 1 px tile and row rules |
| `--ink` | `oklch(0.240 0.020 255)` | `oklch(0.950 0.008 250)` | primary text |
| `--ink-2` | `oklch(0.430 0.018 255)` | `oklch(0.790 0.012 250)` | secondary text (≥ 4.5:1 on `--tile`) |
| `--brand` | `#BC4444` (`oklch(0.550 0.150 25)`) | `oklch(0.640 0.150 25)` | wordmark, the lit tile's tag, the one primary button |
| `--on-brand` | `oklch(0.990 0.003 25)` | `oklch(0.160 0.020 25)` | text on brand |
| `--focus` | `oklch(0.500 0.130 255)` | `oklch(0.760 0.110 255)` | 2 px focus ring, offset 2 px |
| `--done` | `oklch(0.480 0.080 165)` | `oklch(0.780 0.090 165)` | done tag (always with its check shape) |
| `--reading` | `oklch(0.560 0.050 235)` | `oklch(0.740 0.060 235)` | hatch lines of the reading tag |
| `--needs-you` | `oklch(0.470 0.100 75)` text, `oklch(0.930 0.050 85)` fill | `oklch(0.840 0.110 85)` text, `oklch(0.330 0.050 80)` fill | needs-you tag and line (the "broken" signal; not red, per log D1) |
| `--skipped` | `--ink-2`, dashed | `--ink-2`, dashed | skipped tag |

Course and domain tints (fill / label; same hue in both modes, fill lightness flipped):

| Domain | Light fill / label | Dark fill / label |
|---|---|---|
| BIO 201 (moss) | `oklch(0.920 0.045 135)` / `oklch(0.360 0.070 135)` | `oklch(0.330 0.045 135)` / `oklch(0.880 0.060 135)` |
| CHEM 231 (plum) | `oklch(0.915 0.040 320)` / `oklch(0.380 0.090 320)` | `oklch(0.330 0.045 320)` / `oklch(0.880 0.060 320)` |
| CS 110 (cobalt) | `oklch(0.915 0.040 255)` / `oklch(0.380 0.100 255)` | `oklch(0.330 0.050 255)` / `oklch(0.880 0.060 255)` |
| SPAN 202 (marigold) | `oklch(0.935 0.060 90)` / `oklch(0.400 0.080 75)` | `oklch(0.340 0.050 85)` / `oklch(0.890 0.080 90)` |
| ENG 210 (teal) | `oklch(0.920 0.040 195)` / `oklch(0.380 0.070 200)` | `oklch(0.330 0.045 200)` / `oklch(0.880 0.060 195)` |
| Library job, Pre-Health Society, Personal | slate-blue, lilac, sand at the same steps | same |

The course code is always printed beside its tint, so the tint is identity, never state.

### 4.4 Shapes and components

- **Tile**: 168 x 120 px at 1280, radius 10 px, 1 px `--tile-edge`, a soft offset shadow
  (`0 1px 2px` and `0 4px 12px` at low alpha). Key tag 20 px at the top-left. The credential notch is
  a 14 x 6 px cut in the top edge, drawn with a mask (no multi-vertex clip-path).
- **Board**: two rows of five tiles at 1280 x 800, joined by a 1 px track line in `--tile-edge` that
  runs through the tags in reading order (position carries the order, principle 6). Tiles reflow to
  rows of four, then three, keeping the order left-to-right, top-to-bottom.
- **Sheet**: 520 px wide, centred over the dimmed board (backdrop `--cabinet` at 70%), radius 14 px,
  a pane header with Back and Finish later. One pane at a time; panes slide left/right inside the
  sheet. Your week's sheet is 960 px wide.
- **School window**: a real separate window; the sheet shows a mini "window card" with the host in
  mono and a lock-free line "Halverson State's own page" (no padlock icon: it would claim more than
  Knowlu checks).
- **Local credential panel** (zyBooks, VHL): an enclosed panel on `--cabinet` with the notch and one
  line "Stored only on this computer", the only place Knowlu draws a password field.
- **Row**: 44 px, the first row 72 px. Inspector sections separated by 24 px gaps, not boxes.

### 4.5 Density and spacing

Adjacency spacing (`notion.md §3.1`): rows of one group 2 px apart; 24 px between groups; 32 px
above a group label and 8 px below it. Base unit 4 px. The list at rest shows the verdict, the meter
and all 6 of today's rows plus the Later lines within 800 px height. The inspector holds about 12
lines of receipt before scrolling.

### 4.6 Motion

One authored moment and a few state motions; all 150-250 ms, exponential ease-out, from a visible
default; every one has a reduced-motion equivalent (an instant state change).

- **Signature: the proof writes onto the tile.** On success the sheet's checkmark draws (240 ms,
  Stripe Checkout's success mark, `stripe.md §4.2`), then the sheet scales down into the tile it came
  from (220 ms) and the proof text types into the tile's label line at reading pace (no faster than
  the eye; 300 ms total). The next tile's tag fills with brand red (180 ms). Reduced motion: the sheet
  closes, the tile shows its proof, the next tag is red.
- Reading tags: the hatch drifts one stripe every 2 s; static under reduced motion.
- Inspector: slides 24 px and fades in, 180 ms; the day column fades out under it.
- Reveal: the board's tiles shrink into the connected strip (300 ms) while the week's columns fill
  from Monday to Sunday (one pass, 400 ms). The only orchestrated sequence in the product.
- Never: shakes on error, bouncing, confetti, ticking countdowns.

### 4.7 Brand mark

`docs/design/assets/Knowlu_Wordmark_Red.png` at 24 px height in the hub header and the rail top; the
red block of "lu" is the source of `--brand`. In dark mode the wordmark sits on a `--tile` chip so
the near-black "Know" stays legible.

---

## 5. What is taken from Notion, from Stripe and from Plaid Link

### 5.1 From Notion

| Taken | Where it lands | Citation |
|---|---|---|
| Toggle children: a task's steps hide under a one-line row until opened; one level only | list rows with steps ("Study for Exam 2", "Lab 5 report") and the inspector | `notion.md §2.2` |
| A quiet, collapsible left rail on a second neutral; row actions appear on hover **and** focus and stay keyboard-reachable | the rail (§3.3) and row actions (§3.2) | `notion.md §2.4`, §7 (hover-only discovery is the wrong model) |
| Adjacency spacing instead of boxes | list groups, inspector sections | `notion.md §3.1` |
| Soft colour fills with a semantic job: tint is identity | course tints everywhere a course appears | `notion.md §3.3` |
| One object, many renders (the block model): a task is the same object as a row, an inspector receipt, a week mark and a history line | §3.2, §3.4, §3.9 | `notion.md §2.1` |
| Not taken: user-configurable layout; hide-until-relevant (anatomy §1's "every section always renders" wins; empty groups collapse to a named line instead) | | `notion.md §2.3`, §7 |

### 5.2 From Stripe

| Taken | Where it lands | Citation |
|---|---|---|
| Connect's requirements model: the flow is drawn from what is outstanding; resume lands on the first gap; leaving is supported and not finishing | the hub, `BOOT`, resume (§1.4), `P-EXIT` | `stripe.md §1.1`, §1.4 |
| Distinct states, so "checking" never looks like "failed" | the five tile states (§1.2) and Connections rows | `stripe.md §1.2` |
| Pending states carry an expected time | "Reading... usually 1-3 min"; the reveal's 180 s cap | `stripe.md §1.8` |
| Prefill, then confirm; every row editable | Your week (§2.4) | `stripe.md §1.5` |
| A summary step with per-section edit | the reveal's blocks each open their editor (§3.9) | `stripe.md §1.6` |
| Checkout's order summary, one price, one button | Payment (§2.3), where the summary is the School proof | `stripe.md §3.1` |
| The Dashboard Inspector: many states collapse to a few labels in the list; detail lives in the inspector on request | the main page (§3.4) | `stripe.md §2.2` |
| Every error is cause + fix in plain words, with a code underneath for support | all `E-*` panes (§1.6), Connections | `stripe.md §1.7` |
| A banner that speaks only when the student must act, over a calm positive line | the health line (§3.3) | `stripe.md §2.1` |
| OKLCH steps with a contrast-distance rule; the success checkmark as the one motion for proof | tokens (§4.3), motion (§4.6) | `stripe.md §4.1`, §4.2 |
| Not taken: opaque step order (Knowlu's is ruled), operator density, red for negative states, the error shake | | `stripe.md`, "Where Stripe does not transfer" |

### 5.3 From Plaid Link (the third reference)

Plaid Link is the closest analogue to Knowlu's school sign-in: a drop-in flow that sits over a host
app, hands the person to their institution's own sign-in, and comes back with a connection the host
keeps (`onboarding-evidence.md §2.4`). Pages read 2026-10-23.

| Taken | Where it lands | Citation |
|---|---|---|
| **Expectations before the window.** "Link conversion is highest when users have the right expectations set going into Link"; explain why, the benefit, and "what information they will need to provide" | every sheet's first pane: why line, "You'll need" (two items at most), then the button (§2.2) | https://plaid.com/docs/link/best-practices/ ; `onboarding-evidence.md §2.4` |
| **Brief value props, security in one specific line, unlink later.** "two bullets is best"; "Explain that your app does not have access to their credentials"; tell users "they can choose to unlink their account later" | the "Where the secret goes" line per tile; "Every connection can be removed later in Settings" on each first pane | https://plaid.com/docs/link/messaging/ |
| **Data transparency above Continue.** Link's Data Transparency Messaging lists the data types a product reads above the Continue button, with use cases behind "Learn more" | the "What Knowlu reads" line above each button; "What it's used for" behind a Learn more disclosure | https://plaid.com/docs/link/data-transparency-messaging-migration-guide/ |
| **Institution tiles inside the host app.** Embedded Institution Search puts the institution choices directly in the app's page; "Users are more likely to adopt ... when they see their institution's logo" | the hub itself: every source is a tile on Knowlu's own board, named by its own name (Halverson State, zyBooks, VHL), before any sheet opens | https://plaid.com/docs/link/embedded-institution-search/ |
| **Institution pre-selection.** "If you already know the institution ... you can highlight this institution" | `S-SCH` skips `P-SCH-PICK` when the school is known (account email domain or a previous attempt) | https://plaid.com/docs/link/best-practices/ |
| **The institution's own window, with guidance first.** For OAuth institutions Link gives guidance "before handing the user off" and opens the bank's site "in a new pop-up window if possible" | `P-SCH-WINDOW` (school's own page, real host in mono); `P-GOO-NEED` prepares the student for Google's unverified-app warning and its two boxes before the browser opens | https://plaid.com/docs/link/oauth/ |
| **Panes as named views** (select institution, consent, credential, MFA, loading, connected, error, exit) with MFA typed as device / code / questions / selections | the sheet's pane set: `PICK`, `NEED`, `WINDOW`, the Duo pane, `READING`, `DONE`, `E-*`, `P-EXIT`; Duo is a "device" MFA handled on the school's page | https://plaid.com/docs/link/web/ |
| **Merge adjacent consent screens.** Plaid combined its consent and phone panes into one and reports "a statistically significant lift in conversion"; returning users who opted in convert about 11% higher | "You'll need" and "What Knowlu reads" are one pane, not two; Google's Gmail and Calendar are one consent (ruled Q8) | https://plaid.com/blog/ten-years-plaid-link/ |
| **Update mode for repair.** On `ITEM_LOGIN_REQUIRED`, Link re-opens "requesting only the minimum user input required to repair"; several broken items are fixed one at a time, the most recently broken first | `S-REPAIR` from Connections ("VHL signed you out this morning; earlier data stays. Sign in again"); resume when the school session expired (§1.4 item 4) | https://plaid.com/docs/link/update-mode/ |
| **Error object: a code for logic, a display message for people** (`display_message` is "null if the error is not related to user action") | each `E-*` pane shows a human cause + fix and a support code; when nothing is the student's to do, there is no needs-you message, only a reading state | https://plaid.com/docs/errors/ |
| **The exit pane and resume from where the person left** (Link reports the pane the person exited on) | `P-EXIT` names what is kept; `onboarding.json` records the pane, and `BOOT` reopens there | https://plaid.com/docs/link/web/ ; https://plaid.com/docs/link/best-practices/ |
| Not taken: the "256-bit encryption" reassurance (one specific sentence per credential beats a wall of security words, `onboarding-evidence.md §4.3`); a logo grid as marketing; phone-number enrollment and the cross-app network; account selection | | |

---

## 6. Checks, open questions and the risks answered

### 6.1 Ruled facts, checked

| Ruling | Where this brief keeps it |
|---|---|
| One long onboarding, every login inside it, why before each ask, proof after it | §1.1, §2.2 |
| One school sign-in covers Blackboard, course list, syllabi, registrar, grades and school Outlook | School tile; School email tile reads "Covered by your school sign-in" |
| Order: account -> school -> proof -> payment -> school email -> Google -> (calendar) -> homework sites -> syllabi -> Your week -> reveal | the board's fixed order (§1.2, §1.3) |
| $9.99 a month, 7 days free, no free tier; testers see "nothing to pay" | §2.1 price line, §2.3, `P-PAY-TESTER` |
| Gmail by Google permission, one consent; forwarding only on decline, failure or past 100 users; partial grants per scope | `S-GOO`, `P-GOO-PARTIAL`, `P-GOO-FWD` |
| Class times from registrar and syllabi, confirmed, never typed | §2.4 |
| The reveal waits up to 3 min, then names what is still running; backfills never waited for | §1.8, §2.6 |
| Optional sources skippable with cost named; account, payment, school, Your week not | §1.5 |
| Resume after quit at the same step | §1.4 |
| Turned-in work never a task: archived at first sight, later auto-marked done with Undo | reveal tidied line; inspector history and Closed this week with Undo |
| Late work past its window leaves Today for "Late window closed" | §3.2 item 6; in-window label on the Lab 6 row |
| Vault plain text on this machine; portal credentials never leave it | "Show me the folder" (Account receipt); "Where the secret goes" lines; Connections "kept on this computer" |
| Onboarding asks nothing but sign-ins, consents, payment and confirms | no name, timezone, slot or class-time fields anywhere |
| Red is the brand, never an alarm; broken has its own non-red signal | §4.3 `--needs-you` |
| No start-by dates; no assigned work times; no sacrifice suggestions; pin, never reorder | §3.2 |

### 6.2 The 11 pm test

- **Next action in two seconds:** the first row is next, taller, with its why; the verdict names it.
- **Okay or not without reading a paragraph:** the meter and the domain shapes in the rail.
- **Nothing that doesn't help:** the inspector is closed until asked for; the day column is ambient.
- **Nothing that makes them feel worse than the facts:** no red, no counters of overdue work; Late
  window closed is a quiet group with Archive.

### 6.3 Open questions for Quinn (one at a time, with a recommendation)

1. **Do done tiles stay editable during onboarding?** Recommendation: no; a done tile opens a
   read-only receipt until the reveal, so the order can't be re-entered from behind. Forget and
   reconnect live in Connections afterwards.
2. **Should the inspector open by default on the first row each morning?** Recommendation: no; the
   first row already carries its why line. The reveal opens it once to teach the gesture.
3. **Support codes visible to students, or in a "Details" disclosure?** Recommendation: visible, small,
   in mono under the fix; they cost one line and save a support round-trip.

### 6.4 The direction's risks, answered and remaining

- **Out-of-order clicking.** Answered by §1.3: only the lit tile is a button; later tiles are
  read-only with their dependency in words; a click on one nudges the lit tile. *Remaining:* a
  student may still read the board as a menu. The prototype should test whether the track line and
  the single red tag make the order obvious without the "After ..." words.
- **Paywall preview.** Answered by printing the price from the welcome screen, never drawing a lock,
  and naming each later tile by what it reads. *Remaining:* "After payment" on six tiles is honest
  but repetitive; the prototype should try "After payment" on the first post-payment tile only, with
  the rest reading "Then".
- **Operator density.** Answered by §3.1: two content columns at most, inspector on request,
  replacing the day column, list at a document measure. *Remaining:* the receipt itself is dense;
  its History section starts collapsed to the last three lines.
- **The hub costs a screen of chrome before the first ask.** The welcome screen is the board, so the
  first ask (Account) is one click from launch.

### 6.5 Prototype notes

- One HTML file per moment is enough: `W`/`H` (with a state switcher to show each tile state),
  `S-SCH` through its panes including `E-SCH-DUO`, `P-GOO-PARTIAL`, `R-WAIT`, `R-REV`, `M-TODAY`,
  `M-INSPECT` on Lab 6, `M-CONN` with VHL needing sign-in. Light and dark both.
- Data from `../shared/sample-student.json` only. Note its encoding: "Lección", "Gramática", "−30%"
  appear mis-encoded in the file and should be shown correctly.
- No network except Google Fonts (Schibsted Grotesk, Red Hat Mono). Hosts are text, never links.

---

## 8. Red-team resolutions and what the build changed (2026-10-23)

The prototype is `index.html` (one file, sample data inline), 64 screens listed in `screens.json`,
screenshots in `shots/`. A **Prototype** button (bottom right) jumps to any screen and holds the
toggles Quinn asked for in `COLLABORATION.md`: theme, why lines, estimates as ranges, grades, an
avoided task, "school shares one sign-in", "past the first 100 Gmail-permission users", and **Copy
my settings**. `?shot=1` hides it; `&theme=dark` gives dark mode.

### 8.1 The structural change: the board is always visible

> Superseded by §9.1: the docked vertical rail is gone; the full board stays on screen beside a docked sheet.

The first draft dimmed the board under a centred sheet. The build **docks the board as a left
column whenever a sheet is open** (`.dock`, 312 px): every tile keeps its state shape and its proof
while the student works, which is the spec's "progress rail that stays visible to the end" (§1.2).
The full board appears only where it carries the moment: welcome (`W`), mid-way (`H`), resume
(`H-RETURN`), a new computer (`H-RESTORED`) and the reveal wait (`R-WAIT`). After a proof,
"Continue" opens the next sheet directly; the board is one click away ("Board" in every sheet
header).

### 8.2 Rule breaks (violations), resolved

| Finding | Resolution in the build |
|---|---|
| Account dropped "18+ and terms" | `S-ACC` has one required check, "I'm 18 or older, and I agree to Knowlu's Terms and Privacy page"; both sign-in buttons explain and refuse until it is ticked. |
| Welcome showed zyBooks/VHL and "1 of 10" before the school step | No homework tiles before the school step. A dashed **"Found at your school"** slot holds their place; after the school proof the slot is replaced by zyBooks, VHL Central and Syllabi, each flagged "New" (`P-SCH-PROOF`). No step counter anywhere. |
| "Older school email ... will be proposed" | Removed. Copy says what is true for an M365 school until lane L1: "School mail from before today isn't brought in yet; new school mail is read as it arrives." Gmail's 7 setup days are **applied and listed with Undo** (`P-GOO-READING`, `R-REV`), per VISION commitment 5's setup exception. |
| School email hid the mailbox writes | `S-MAIL` and `P-MAIL-NEED` list what Knowlu does in the mailbox: turns on forwarding, sends one test, forwards missed mail and removes those Sent copies, turns forwarding back on if it stops and says so; "It never sends mail to anyone else." Privacy page linked. |
| Calendar-only grant skipped the forwarding fallback | `P-GOO-PARTIAL` offers **Share Gmail**, **Use forwarding instead**, or continue without Gmail (cost named, `P-SKIP-GMAIL`). "Gmail only" adds the Another calendar tile (`S-CAL`). |
| Past-100 students saw the Gmail permission ask | Knowlu's own count decides before the sheet opens: `P-GOO-NEED-FWD` asks Google for **Calendar only** and sets Gmail up by forwarding, saying why. |
| Syllabi offered only a file | `S-SYL` offers **Choose a file** and **Connect Simple Syllabus** (shown when the campus row says the school has it), plus Skip per course. The typing last resort is named: CS 110's times came from the registrar, so there is nothing to type. |
| Row anatomy broke log D5 | Default settings are Quinn's: **one large "Next" card**, why lines **off** (toggle in Prototype), **estimates as ranges** ("45m–1h 15m"), and rows beyond today's time (Later) **greyed**. The card's "Why it's here" opens the receipt. |
| No grades on the main page | The rail shows a **small ring per course coloured by letter** with the letter beside it, and a **term GPA with its band** ("about 3.2; likely 3.0–3.5; narrows as more is graded"). **Hide grades** sits in the rail's section header. |
| Collapsed rail lost "behind" marks | Collapsed (Ctrl+\ or the rail button, and below 1100 px) each domain keeps its tint and its **status shape**; the accessible name carries the word and hours. |
| No place for an avoided task | Rows carry a dashed tag, e.g. "waited 4 mornings" (on by default in the prototype; it is **not in the sample data**, so it's a toggle). Ranking an avoided task higher is the engine's job, not the UI's. |
| Undo on a source's change | History lines from a source (zyBooks progress) and Knowlu's estimates have **no Undo**; the receipt says why. Undo appears only on Knowlu's marked-done (`M-AUTODONE`) and on the student's own actions (`M-UNDONE`, toasts). |
| `BOOT` repeated onboarding for an existing account | `H-RESTORED`: an account with data skips what the account holds (payment, Google, school email forwarding, syllabi, Your week), shows their proof, and asks only for the device-held sign-ins (school, zyBooks, VHL). |
| "No start-by dates" cited as ruled | It is **not signed**: PRODUCT-DECISIONS marks it "Needs a VISION amendment" and anatomy §3.6 still sorts by `start_by`. The build shows no start-by dates (log D3: ranking unchanged, display only); the amendment stays open for Quinn. |

### 8.3 Problems, resolved or named

| Finding | Resolution |
|---|---|
| Counts drifted (1 of 10, 7 of 9, 9 of 9) | One definition everywhere: **places connected** = School, School email, Google, Another calendar, zyBooks, VHL (Account, Payment, Syllabi and Your week are not places). Maya: "5 places connected · found 5 classes, 40 assignments, 9 already turned in" (26+8+6; 4+3+2). The wait counts **first runs** separately ("10 of 12 first runs done"). Your week: 27 rows, 7 set from the registrar, 20 to check. |
| "Installed" tile was padded progress | Removed. |
| Copy hard-coded Duo, Blackboard, shared sign-in | Copy reads a curated campus row (`CAMPUS`: `mfa`, `lms`, `shared_sign_in`, `host`, `simple_syllabus`); the School tile and sheet drop "school email" when `shared_sign_in` is off, and School email then asks for its own sign-in. A school with no `mfa` reads "your phone, if your school asks for a second step". |
| Locked account was sent to the feed link | `E-SCH-LOCKED` offers only Finish later and says why the calendar-link route can't help (it needs the same sign-in). |
| Feed link had no guidance | `P-SCH-FEED` has Blackboard/Canvas tabs with where-to-find steps, the cost named, an inline format error, and "Tell Knowlu your school isn't listed". |
| Payment could open before counts | `P-SCH-READING` keeps "Continue to payment" unavailable until the **assignment count** is in ("Waiting for your assignment count"). Grades and syllabi may still be reading: `S-PAY-SLOW` shows only the counts that exist and says the rest is still reading. **Open:** whether the school's hidden windows keep reading while Stripe's checkout is in the browser is spike S-G's. |
| Two free-time numbers on one screen | Verdict sub and day column both read the read model's one value: "about 5.7h ... after 6:00 pm". (Sample-data note: 6:00-11:30 pm is 5.5h; the 5.7h includes the minutes before 6:00. Flag for whoever owns the shared data.) |
| Sheet hid the proof rail | §8.1. |
| Your week widened the ruled confirm | `S-WEEK` confirms only class times, exams, grading and late rules (spec §1.3 step 10). Repeating calendar events, office hours and the wake-to-bed window are **not asked**; the sheet says Knowlu asks about each event from tomorrow (no commitment card on day one). Rows Knowlu is least sure of come first and are answered one by one; **Confirm all** exists only for the high-confidence syllabus groups; registrar rows show as "Set" with Edit (R24 writes them as the student). This **departs from the sample JSON**, which records commitments and "Your day" as confirmed at step 10. |
| Read-model fields that don't exist | The UI shows no field names. Building it needs read-model additions the surface does not have: the "Late window closed" group (lane O6), per-row source and read time, per-row history with actor, a per-line Undo flag (O1 adds it to Closed this week only), domain grades for the rail. Each changes `surface` output and falls under the `surface-today-*.json` rule (console spec §4.6). |
| Unruled payment copy | Removed "you pay Knowlu, so Knowlu doesn't sell your data" and "Cancel any time in Settings". `S-PAY` says "No charge today. Cancel before October 25 and nothing is charged." (**copy for Quinn**) and adds the ruled term "Monthly billing pauses June to August". |
| No privacy-page link or secret line on credential steps | `P-SCH-NEED`, `S-MAIL`, `P-MAIL-NEED`, `P-GOO-NEED`, `P-GOO-FWD` and Connections link the privacy page; `P-GOO-FWD` names where the Gmail session is kept. Every connection's first pane has a "Later" line saying it can be removed or turned off in Connections. |
| "Think in the cloud" never disclosed | A "What goes to Knowlu's cloud" line on School, Google, forwarding and Syllabi: assignment and syllabus text, and only the messages that could change the plan; never a password or session; Knowlu keeps what it found, never the message. |
| Dead clicks on later tiles | A click or Enter on a later tile writes one line under the board (what it reads, what it comes after) and nudges the lit tile. Later tiles read "After payment" only on the first post-payment tile, then "Then", then "Last". |
| Payment early, scary Google warning after it | Order is ruled (Q2). `S-PAY` adds the soonest due item ("Due soonest: Reading: Ch. 8, Metabolism, tomorrow at 9:00 am"), which needs the device-side summary to carry one title (a proposal). `P-GOO-NEED` prepares the warning step by step and offers forwarding as an equal choice. |
| Duo window could hide; timings disagreed | `P-SCH-WINDOW` has "Bring it to the front"; the Duo error no longer quotes a timeout. |
| No re-engagement after quitting | `P-EXIT` offers one opt-in reminder email tomorrow evening (**a proposal**: VISION allows one daily email at most; start with Windows still turns on only at the reveal). |
| Reveal diluted the aha; taught a gesture | `R-REV` leads with **What's next**, then **Open Knowlu**, then "Already handled: 9 already turned in and 52 past items were archived". The week grid sits beside it. No inspector is opened on the reveal. |
| Main page had about 15 regions | Centre: date and delta line, verdict, meter, Next card, Also due by tomorrow, Recommended today, Later (two collapsed lines), Late window closed, one Needs-you line, Done today. Right: **Today** (one timeline with free time + schedule), **What changed** (delta, Knowlu's closes with Undo, notices), **Next two weeks**. The inspector replaces the right column. |
| "Effort estimated for 34 assignments" | "34 tasks". |
| "Done today ... uncheckable" | "Uncheck it if it isn't"; the box unchecks. |
| `?` legend; no quick-capture key | No legend; shortcuts live in tooltips (j/k, Enter, Esc, x, p, f, c, d, Ctrl+\). Quick capture is not built in this prototype. |

### 8.4 Still open for Quinn

1. Copy that needs a ruling: the trial-cancel line on `S-PAY`, the reminder-email offer on `P-EXIT`,
   "Due soonest" on `S-PAY`.
2. The VISION amendment for "no start-by dates" (still unsigned).
3. Whether Your week should ever ask about calendar events, or leave them to day-two cards (built:
   day-two cards).
4. The three questions in §6.3 stand.

## 9. Second pass: critique fixes and what the build changed (2026-10-23)

Two critiques scored the first build 56 and 63. Their shared verdict: the hub was a stepper with a
new skin, sheets ran past 800 px, the main page answered "how many hours" before "what's next", and
the reveal's week grid looked broken. This pass revises `index.html` in place. It now has 65
screens, and `P-LOCKED` is new.

### 9.1 The structural change: one board, always whole, and open after payment

- **The board is never folded into a rail.** On every onboarding screen the full tile grid stays on
  the left (two columns), and the sheet docks on the right as a full-height drawer (620 px). The
  drawer's header, body and footer are separate, so the action is always on screen at 1280x800. The
  board is the host page the sheet sits on (Plaid Link's model), so the "Board" link is gone; the
  close button and Esc open the whole board (`BOARD`). Outside windows (the school's sign-in, Duo,
  Google, Stripe) are drawn as separate windows over the board, as Windows would show them.
- **Order is ruled only where it is ruled.** Account, school, then payment, in that order. **After
  payment every remaining tile opens at once and can be done in any order**. Knowlu marks one
  "Suggested next" (school email first, because it's one click when the school shares its
  sign-in). Your week opens when the rest are connected or skipped. Spec §1.3 gives the order as
  "roughly"; this pass reads that as permission. **Decision for Quinn** (§9.4).
- **Every tile is a button.** A tile that isn't open yet opens a short sheet saying what it is,
  when it opens and why (`P-LOCKED`), with a button to the open step. Clicking it never does
  nothing.
- **States:** a round badge (check, arrow, empty ring, spinning arc, ring with "!", dashed ring,
  lock) beside a drawn source icon (person, building, card, envelope, @, book, speech bubble, file,
  list). The key-tag shape read as a price tag and is gone. So is the "◡" notch glyph.
- **Counts:** "N of M places connected · found …" in the top bar only (the rail footer copy is
  gone). Places = school, school email, Gmail and Calendar, another calendar, zyBooks, VHL and
  syllabi, so Maya has 6. The denominator grows when the school step finds the homework sites.
  "About N min left" comes from the open tiles' estimates.
- **No interstitial.** `P-PAY-DONE` continues straight to the suggested sheet. `H` stays as the
  board you see when you close a sheet after payment.
- **Board views** (`W`, `H`, `H-RETURN`, `H-RESTORED`, `R-WAIT`, `BOARD`) are one layout: the words
  and the one button on the left, the board on the right. No connector lines.

### 9.2 Fixes by finding

| Finding | Change |
|---|---|
| Sheets clipped at 800 px | The drawer body is shortened to three fact rows (You'll need / What Knowlu reads / Where it goes). The cloud line is folded into "Where it goes", and "Later" is one footnote line. The footer is always visible. Checked in every screenshot. |
| Raw error codes shown to students | Removed from every pane (school, VHL, broken state, Connections, repair). |
| Mono as costume | Red Hat Mono is kept only for hosts, file paths and pasted links. Times, source labels and schedule times are set in the text face. |
| zyBooks "found in CS 110's syllabus" while that syllabus was missing | zyBooks is "linked from CS 110's Blackboard page", and VHL is "named in SPAN 202's syllabus" (`P-SCH-PROOF`, `S-ZY`). |
| Your week was a 20-row audit, with typing | `S-WEEK` asks only the **2 rows Knowlu is least sure of**, each answered by picking. ENG 210 has no late rule: not accepted / ask me when late / leave blank. The CS 110 weights come from your file: looks right / read it again. The other **25** are four summary lines (classes, exams, grading, late rules) with Show and "Not right?". A wrong registrar row opens a pick of the registrar's sections, never a text field. One button: "Confirm the 25" or "Confirm all 27". The tile's proof matches: "27 confirmed", or "25 confirmed · 2 asked later". |
| R-WAIT duplicated the board with a list | The list is gone. The board itself shows the two last runs on the Syllabi and Your week tiles. |
| Reveal was a report | `R-REV`: the headline, then What's next with its button inside ("See today's list", not "Open Knowlu"), then a green **"9 things you'd already turned in are archived"** line, then two short lines: From your email (with Undo) and Still coming (plus Not connected after the cap). The source chips in the header are gone. |
| Week grid collisions | Due items sit in their own **Due lane** above the hours: at most 3 a day, then "+N more", with the full title on hover. They never overlap a class. Event names are one line with an ellipsis; a time shows only when the block is tall enough. Same grid on `M-WEEK`. |
| M-TODAY led with hours | The **Next card is the first and largest thing** (title at 1.8rem). The hours verdict, the 13-block meter and the delta line are gone. One hour figure per group ("4 · about 3.5h · today has room for it"). One free-time figure, only in the day column. "What changed" became "Since last night" and holds the delta. The two-week chart became one "Coming up" sentence with "See the week". |
| Rail was a third content column | The rail is navigation. Each course is one line: the name, a status shape (behind is amber, and its word is in the accessible name), and the grade ring. The GPA is one line. The page has the rail plus two content columns, and the inspector replaces the right one. |
| M-BROKEN stayed confident | It opens with a one-line amber strip ("signed you out at 12:00 pm. Dates and grades below are from before then"). The group reads "as of 12:00 pm", grade rings hide and the GPA reads "Grades paused", and free time adds "if class times haven't changed". Next stays above the fold. |
| S-MAIL read as a second ask | When the school shares its sign-in, the sheet's one button is "Turn on forwarding". The tile says "about 1 min" and "Uses your school sign-in". |

### 9.3 Kept, as both critiques asked

The Stripe-style order summary built from the school proof (`S-PAY`). The school's own window with
its host and "Bring it to the front". The fixed set of rows on every sheet. Proof written onto the
tile. Skip with its cost in one sentence. Errors that give the cause, then the fix. The inspector
receipt (`M-INSPECT`). Late window closed with "When will you turn it in?" (`M-LATE`). Undo on
Knowlu's marked-done (`M-AUTODONE`, `M-UNDONE`). Connections as what the board becomes after
onboarding (`M-CONN`). `M-CLEAR`'s voice.

### 9.4 New open question for Quinn

**Any order after payment.** Recommendation: keep it. It is what makes the board a hub instead of
a wizard, the ruled order still holds where it carries meaning (proof before price), and the
engine reads nothing past payment until the trial is on, so no dependency is lost. If you rule a
fixed order, the build changes in one place: `bAt` marks the tiles after payment "later"
instead of "ready".

## Round 2 (2026-10-23)

What changed after the comparison README named this direction the most template-like:

- **The board is ranked, not a grid.** The nine equal padlock cards are gone. The board now reads
  top to bottom by state, then importance: one large tile for what needs you or comes next; "Reading
  now"; "Also open" (any order, one row each); "Connected", where each place shows what it found;
  "Not open yet", one quiet line per gate with name chips and no padlocks; and Your week at the end
  as the finish line, set off below a dashed rule. Homework sites and syllabi sit indented under the
  school that found them (open ones say "Found at your school"). Account and payment drop to one
  line of small print under Connected, because they aren't places.
- **Proof is never cut.** Every connected row shows its full proof and wraps when the column is
  narrow (the 320 px column on Your week included). The school's row is the primary one, set larger.
  The ellipsis rule on the compact board is gone.
- **The pre-sheet and repair are kept.** "You'll need / What Knowlu reads / Where it goes" and the
  update-mode repair ("VHL signed you out; earlier VHL work stays") are unchanged.
- **The wait is real (R-WAIT).** "Building your week, 5 of 7 done" now counts, with a bar, the
  finished jobs and the two still running (CS 110's syllabus, and estimates at 21 of 34). It also
  states the 3-minute cap, and the board's "Reading now" group sits at the top.
- **The daily page answers what's next first (M-*).** The rail starts collapsed (the old default was
  open), so the page is no longer a three-column console. The next card is bigger: "Do this next",
  a 2rem title, and one plain sentence ("Finish it today: about 55m–1h 30m left. It's late, so it's
  worth 70% now and closes Wednesday at 11:59 pm") ahead of the meta and actions. The center column
  is wider (720 px) and the day column narrower (380 px).
- **"Free after" is computed.** The day column's free-time line now comes from the same numbers that
  draw the bar, never from a literal.
- All 65 screens and the three dark shots were re-shot into `shots/`.
