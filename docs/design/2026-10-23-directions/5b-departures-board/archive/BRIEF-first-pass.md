# Direction 5b: first-pass brief (superseded)

Moved here from `../BRIEF.md` on 2026-10-23 in the second pass. Everything below is withdrawn where
`BRIEF.md` (R2 and R3) says otherwise, which includes the airline metaphor in student-facing copy
(departures, Boarding soon, Arrived, the ticket line) and every turned-in animation. Kept only so the
reasoning behind the direction stays readable.

---

## 0. The direction in one paragraph

**What students fear is less the list than being surprised, so status and change become the
structure.** One object runs through the whole journey: **the board**, a strip of the student's week
in which every due item is a "departure" with its due time. Onboarding builds that board in front of
the student. Each source's proof adds departures to the week strip ("Thu · BIO 201 Lab 4 report ·
2:00 pm"), and work the student already turned in lands stamped "Turned in", then folds into an
"Arrived" count and leaves. Sources still reading are named in words ("Boarding soon: zyBooks and
VHL due dates"), never shown as a spinner. The reveal is the week's board, complete. The daily page
is one live card (what's next and why) above today's board, where each row carries one status word.
Changes arrive the way Flighty reports a gate change: what changed, new value (was old value), the
source and who made it ("CHEM 231 Exam 2 is in Hallett Hall 120 (was 220). From a CHEM 231
Blackboard announcement, Saturday.").

**What this direction refuses.** It refuses a hidden week, the way direction 2 holds the week back
until the reveal: here the week is visible from the first proof and grows with every login. It also
refuses an abstract progress counter. Progress is the board filling, counted in departures and
sources, never in screens.

**The risk it carries, and the rules that answer it** (detail in §5.2 D, §6.4 and §7):
1. *Countdowns can create the urgency the brand forbids.* The board shows **due times**, never
   start-by times. Its big figures are clock times ("11:59 pm"), not timers. A relative phrase
   ("in about 15 hours") appears only on the live card and the tray. It is rounded down to the hour
   and changes at most every 15 minutes. Nothing ticks by the second or the minute.
2. *A departures metaphor can feel like missing a flight.* Nothing on the board ever "departs
   without you". A past-due item inside its late window stays on the board as "Late window open,
   −30% now". Past its window it moves to "Late window closed", where Archive and Still doing it
   are equal choices. The words "missed", "gone" and "too late" never appear. Status words are
   amber at most, never red, and always paired with a shape.

---

## 1. The board: the one object this direction is built on

Notion's block model (`notion.md §2.1`): one task note renders as a strip mark, a board row, the
live card and the tray card, with the same handle, the same click-to-open detail and the same
edit-in-place controls. Moving a task between Today and later changes where it renders, never what
it is.

### 1.1 The week strip (onboarding, reveal, and the top of the main page)

- Seven day columns (Monday to Sunday) and one narrow **Later** column for anything due after Sunday.
  In onboarding and the reveal the week is October 19-25. On the main page it is the current week.
- Each column header carries the weekday, the date and an **hours-due figure** in board type ("Thu 22
  · 4.0h"). Show amounts: a 4 px bar under the header is filled to hours due against the busiest
  day of the strip.
- Inside a column, departures are one line each, ordered by due time: course code chip (soft course
  tint), a short title, and the due time right-aligned in tabular figures. Columns show up to four
  lines, then "+2 more", which opens the day.
- **Fixed marks** are commitments: classes, labs, the job, the club. They are drawn as thin bands
  along the column's left edge at their clock position, never as lines in the departure list. Until
  Your week confirms them, they are **dotted** and labelled "to confirm" on hover or focus. After
  confirmation they are solid.
- Grid recipe from Notion Calendar (`notion.md §5.4`): 1 px column rules at about 9% opacity,
  tabular figures, soft course fills with dark labels, no card boxes.

### 1.2 Board rows (today's board on the main page, and the reveal's "tonight and tomorrow")

One row per departure, one line, 44 px tall at default density:

| Column | Content | Example |
|---|---|---|
| Order | Knowlu's position, small ("1", "2") | 2 |
| Due | clock time in board type, tabular, with a day word under it | 11:59 pm / today |
| Course | course code chip in its tint | CS 110 |
| Title | the task title, one line, ellipsis with the full title on focus | zyBooks 7.1-7.4 participation |
| Left | the remaining estimate, as an estimate | 24m |
| Status | **one** word with its shape (§1.3) | On track |
| Source | small, last | zyBooks |

The board is in **Knowlu's order**, not time order. A real departures board sorts by clock, so the
header says so in words: "In order: what's next first." The Order column is there so position reads
without labels (principle 6).

### 1.3 Status words (Stripe's collapse: many system states, a few human labels)

Stripe's Dashboard maps seven PaymentIntent states to a handful of labels, with detail one click
away (`stripe.md §2.2`), and types its badges by what the reader must do (`stripe.md §2.3`). The
board has five row words. Each has a shape, so colour is never the only signal (principle 10). The
reason behind a word is one click or one Enter away.

| Word | Shape | Tone | Means | Detail on request |
|---|---|---|---|---|
| **On track** | filled dot | ink | nothing for the student to do but the work | the "why" line, the source, last read |
| **Changed** | diamond | amber | an authoritative source changed it in the last 24 h; already applied | the change in Flighty form: new (was old), source, who, when, Undo where Knowlu applied it |
| **Late window open** | half-filled circle | amber | past due, inside the syllabus late window | "−30% now; closes Wednesday at 11:59 pm", and the rule ("Late: −10% a day, up to 5 days, your syllabus") |
| **Needs you** | hollow ring | cool blue | the student has to answer something (a proposed date change) or a source needs a sign-in | the question, with its evidence |
| **Done** | check | ink, struck title | checked off today; stays for the day and can be unchecked | who and when |

Rules:
- One word per row. If two apply, the one that asks the most of the student wins: Needs you, then
  Late window open, then Changed, then On track.
- Amber is the strongest tone on the board. **Red never marks a state**; it appears only in the
  wordmark and the primary button (principle 9, `log.md` D1).
- **Late window closed is not a row word.** Such a task leaves Today into its own group (§5.2 F).
- **Turned in is not a row word on Today.** Work turned in leaves the board: first sight archives it
  (onboarding), and afterwards Knowlu marks it done with Undo and lists it under "Arrived this week".

### 1.4 Change lines (Flighty's gate-change form)

Flighty states a change as the new value with the old one beside it, plus the cause: "New
Departure: 8:28pm (was 11:30pm)", "Changed to ORD Terminal 2 • Gate 7" [F2], "Delayed 45m. Due to
late arriving aircraft from New York." [F4]. Knowlu's change line has five parts, always in this
order:

> **What** · **new (was old)** · **from where** · **who** · **when**, then an action when there is one.

| Who started it (VISION) | How it lands | Example |
|---|---|---|
| An authoritative source (Blackboard, the registrar, zyBooks, VHL) | applied and listed as **Changed** | "CHEM 231 Problem set 7 now due Tuesday at 9:30 am (was 11:59 pm Monday). From Blackboard, read 6:02 am." (illustrative) |
| A notice that changes nothing in the plan (a room) | shown, not proposed | "CHEM 231 Exam 2 is in Hallett Hall 120 (was Hallett Hall 220). From a CHEM 231 Blackboard announcement, Saturday 10:03 am." |
| Knowlu noticed it (an email) | proposed, **Needs you**; the official time stays until the student answers | "Lab 8: Dictionaries. Official: Wednesday, November 4 at 11:59 pm. Email says: Friday, November 6 at 11:59 pm. From Priya Raman, CS 110 TA, Sunday 9:41 pm." [Move it] [Keep Wednesday] |
| Knowlu acted (turned-in work) | applied, with Undo | "Pre-lecture quiz: Ch. 7 done. Blackboard shows it turned in, Sunday 4:40 pm. Marked by Knowlu. Undo" |
| The student | acts at once, with Undo | "You moved Recitation worksheet 8's estimate to 45m (was 1h). Undo" (illustrative) |

"Official ... / Email says ..." is Flighty's practice of showing its predicted time beside the
airline's official one [F5]. The official value stays in force until the student accepts the
change; Knowlu never presents what it noticed as settled.

---

## 2. Onboarding layout (1280x800)

Three horizontal bands, top to bottom. They never swap places, so position carries meaning:

1. **The step** (top, about 52% of the height). The step's name, then its benefit line, then the
   one ask, then its proof. The column is left-aligned with a 64ch measure. Credential moments
   (the school window, homework-site logins, checkout) sit in one enclosed **secure panel** used
   nowhere else (`onboarding-evidence.md §3.4`), with one plain line about where the secret goes.
2. **The board** (middle, about 36%). The week strip of §1.1. Above it, one line of **sources**:
   each connected or expected source with one word (Reading, Done, Needs you, Not on the board).
   Under the strip, the **"Boarding soon"** line names what is still to come ("Boarding soon: zyBooks
   and VHL due dates, after homework sites"). Under that, any **"Not on the board"** lines for
   skipped sources, each with what it would add.
3. **The ticket line** (bottom, fixed, 32 px). Disclosed at the account step, it rides along until
   payment: "Knowlu: $9.99 a month after 7 days free. You pay after you've seen your classes. No
   free tier." At payment it becomes the order summary. After the trial starts it reads "Trial on.
   First charge October 25." for one step, then leaves. (Up-front disclosure,
   `onboarding-evidence.md §3.5`. Jones on Flighty: "There's something comforting about information
   always being there" [F1].)

Right of the step title, small: "About 6 minutes left" (an honest estimate, recomputed from the
spec's per-step times) and "Finish later".

Before the school proof the strip is drawn empty with its seven dated columns ("Your week will
build here."). Real data only: no placeholder tasks, ever (`notion.md §1.2`).

---

## 3. Flow map

### 3.1 Screen ids

`BOOT` router · `O*` onboarding · `X*` exits and resume · `W*` the wait · `R*` the reveal · `M*`
the main page · `T*` the tray. Sub-ids: `-x-` is a failure, `-s` a skip, `-p` partial.

| Id | Screen | Shown when | Leaves by (trigger -> next) |
|---|---|---|---|
| `BOOT` | Launch router, invisible, under 300 ms | every launch | reads `onboarding.json`: no file -> `O0`; file with gaps -> `X2` resume at the saved step; finished -> `M1` |
| `O0` | Welcome. Empty strip with dated columns | first launch | "Start with your account" -> `O1` |
| `O1` | Your account: Google or an emailed code; 18+ and terms. The ticket line appears | from `O0` | "Continue with Google" -> `O1g`; "Email me a code" -> `O1c` |
| `O1g` | "Finish in your browser" (host `accounts.google.com` shown) | Google chosen | signed in -> `O1d`; browser closed or 10 min idle -> `O1` with "The Google sign-in didn't finish. Nothing was saved." |
| `O1c` | Six-digit code | code chosen | right code -> `O1d`; wrong code -> inline line, stays |
| `O1d` | Proof: checkmark (the one motion), "Signed in as ..." | account made | 1.2 s, or Enter -> `O2` (background: vault folder created with defaults) |
| `O2` | Your school: pick (preselected from the account email's domain when known) | from `O1d` or resume | a school -> `O2n`; "My school isn't listed" -> `O2f` |
| `O2n` | What you'll need, and what Knowlu reads | school known | "Sign in at Halverson State" -> `O2w`; "Not my school" -> `O2` |
| `O2w` | The school's own window is open beside the app; the step shows its host (`login.halverson.example`) | button pressed | Duo screen reached -> `O2m`; session captured -> `O2r`; window closed early -> `O2-x-closed`; lockout page -> `O2-x-locked` |
| `O2m` | Duo: "Approve the push on your phone" (the school window shows Duo; the step mirrors it in words) | school page shows Duo | approved -> `O2r`; no session 3 min after Duo appeared, or the school's timeout page -> `O2-x-duo` |
| `O2-x-closed` | The sign-in window closed | as above | "Open it again" -> `O2w`; "Finish later" -> `X1` |
| `O2-x-duo` | Duo got no answer | as above | "Try again" -> `O2w`; "Finish later" -> `X1` |
| `O2-x-locked` | The school says the account is locked | as above | "Finish later" -> `X1`; "Use the course feed link instead" -> `O2f` |
| `O2f` | Fallback: paste the Blackboard calendar feed link, cost named | school not listed, or chosen after a failure | valid link -> `O2r` (fewer sources); invalid -> inline line, stays |
| `O2r` | Reading. Sources line fills one source at a time; departures land on the strip as each source finishes | session captured | "Continue to payment" is enabled at once -> `O3` (reading carries on) |
| `O2p` | The school proof, in full (shown in place of `O2r` when reading finishes before the student moves) | all school reads done | "Continue to payment" -> `O3` |
| `O3` | Payment. The order summary is the board so far | from `O2r`/`O2p` | "Continue to checkout" -> `O3b`; tester entitlement -> `O3t` |
| `O3b` | "Finish in your browser" (host `checkout.stripe.com`) | checkout opened | trial started -> `O3d`; checkout closed unpaid -> `O3-x-closed` |
| `O3-x-closed` | Checkout closed before the trial started | as above | "Open checkout again" -> `O3b`; "Finish later" -> `X1` |
| `O3t` | "Nothing to pay: you're a tester." | tester | "Continue" -> `O3d` |
| `O3d` | Checkmark; ticket line reads "Trial on. First charge October 25." | paid or tester | 1.2 s, or Enter -> `O4` |
| `O4` | School email | from `O3d` | covered by the school sign-in -> body shows "Nothing new to sign in"; "Turn on school email" -> `O4r`; not covered -> one sign-in in the secure panel, same endings as `O2w`; "Skip school email" -> `O4s` |
| `O4r` | Test message sent, "usually under a minute" | turned on | "Continue" (enabled at once) -> `O5`; test arrives -> proof on the sources line; none after 3 min -> source reads Needs you (§4.3) |
| `O4s` | Skip pane: cost in one sentence | Skip | "Skip for now" -> `O5`, a Not-on-the-board line added; "Back" -> `O4` |
| `O5` | Google: Gmail and Calendar, one consent | from `O4*` | "Continue with Google" -> `O5b`; "Skip Google" -> `O5s` |
| `O5b` | "Finish in your browser" (host `accounts.google.com`); the step names the unverified-app screen ahead | consent opened | both granted -> `O5r`; one scope only -> `O5p`; declined, or the Gmail read fails -> `O5f` |
| `O5p` | Partial: "Gmail connected. Calendar not shared." | partial grant | "Share calendar" -> `O5b` (only the missing scope); "Continue without it" -> `O6` if no calendar was granted, else `O7` |
| `O5f` | Fallback: Gmail forwarding, about 3 minutes | declined or failed | forwarding verified -> `O5r`; "Skip Gmail" -> `O5s` |
| `O5r` | Reading, then proof; commitments land on the strip as dotted fixed marks | granted | "Continue" -> `O7` (or `O6` when no calendar) |
| `O5s` | Skip pane | Skip | "Skip for now" -> `O6`; "Back" -> `O5` |
| `O6` | Another calendar (iCal link). Only when no calendar was granted. Not shown for Maya | as above | valid link -> proof -> `O7`; "Skip" -> `O7`, Not-on-the-board line |
| `O7` | Homework sites, one secure panel per site found mentioned (zyBooks, then VHL) | from `O5*`/`O6` | sign-in accepted -> that site reads; next site; both done or skipped -> `O8`; rejected -> `O7-x`; "Skip VHL" -> `O7s` |
| `O7-x` | The site didn't accept the sign-in | as above | "Try again" -> the same panel; "Skip VHL" -> `O7s` |
| `O7s` | Skip pane, per site | Skip | "Skip for now" -> next site or `O8`; "Back" |
| `O8` | Syllabi: only courses with none found (Maya: CS 110) | from `O7*`; when nothing is missing, `O8` is passed with a one-line note on the sources line | "Choose a file" -> reading -> `O9`; "Skip CS 110" -> `O8s` |
| `O8s` | Skip pane, per course | Skip | "Skip for now" -> `O9` |
| `O9` | Your week: prefilled rows to confirm; dotted marks on the strip turn solid as rows are confirmed | from `O8*` | "Confirm your week" -> `W1` |
| `W1` | The wait. The strip stays; the step band becomes a countable list of what is still running | confirmed | everything ran once -> `R1`; 180 s cap -> `R1` with "still running" lines |
| `R1` | "Here's your week": the week's board | from `W1` | "Open Knowlu" -> `M1` (the scheduler and start-with-Windows turn on here) |
| `X1` | Finish later | "Finish later" anywhere | "Close Knowlu" -> app exits, state saved; "Keep going" -> back where it came from |
| `X2` | Resume: the saved step, with one return line above it | `BOOT` finds gaps | continues as that step |
| `X3` | Resume with the school session expired | `BOOT`, school session gone | "Sign in again" -> `O2w` with the school preselected; then back to the saved step |
| `M1` | Main page: week strip, the live card, today's board, the right column | daily | a row -> `M2`; a change line -> `M3`; Late window closed -> `M4`; Needs you -> `M5`; VHL's line -> `M6` |
| `M2` | A row open: its detail drawer under the row (fields, why, history with actor, Undo) | row clicked or Enter | Esc -> `M1` |
| `M3` | A change open: the full change line with its evidence | change clicked | Esc -> `M1` |
| `M4` | Late window closed, opened | group clicked | Archive / Still doing it ("When will you turn it in?") -> `M1` |
| `M5` | Needs you deck, one card at a time | card clicked | an answer -> next card; Undo for 10 s |
| `M6` | Source repair: VHL sign-in in the secure panel | "Sign in to VHL" | accepted -> `M1`, rows' status words recompute |
| `M7` | All clear (a day with nothing due) | no departures today | none |
| `T1` | Tray flyout: the live card alone | tray icon clicked | "Open Knowlu" -> `M1`; "Start" -> focus session |

### 3.2 Transitions, stated once

- **Every proof lands with the checkmark** (Stripe Checkout, `stripe.md §4.2`), then the step
  advances on Enter or after 1.2 s. The checkmark is drawn once. Under reduced motion it appears
  drawn.
- **Departures land on the strip the moment their source finishes**, not when the student
  advances. A source that finishes during a later step still drops its lines into the strip, and its
  sources-line word changes from Reading to Done. This is the board growing visibly through every
  later login.
- **Turned-in work lands, then leaves.** Each turned-in item appears in its column for 1.5 s with the
  stamp "Turned in", then folds into the strip's Arrived count ("Arrived: 4, archived"). Under
  reduced motion it appears only in the count. Items due after Sunday go straight to the Later
  column's count.
- **Nothing blocks on background work.** Every Continue is enabled as soon as the ask is answered
  (spec §1.5).

### 3.3 Resume after quit (`X2`, `X3`)

The saved step reopens with every finished proof intact and the strip as full as it was. Background
jobs that had not finished are re-run (idempotent) and their sources read Reading again. One return
line sits above the step for this launch only, naming the open loop (`onboarding-evidence.md §1.5`):

| Left at | Return line |
|---|---|
| the school window | "You were signing in at Halverson State. Nothing was saved from that try." |
| payment | "You're at payment. Your school is on the board: 5 classes, 6 departures this week." |
| Google | "You're at Google. Your school and payment are done; 6 departures are on the board." |
| a homework site | "You're at homework sites. zyBooks is on the board; VHL is next." |
| Your week, part done | "You're on Your week. 14 rows confirmed, 19 to go." |

`X3`, school session expired while Knowlu was closed: "Halverson State signed you out while Knowlu
was closed. Sign in again; everything on the board stays." The school is preselected. A student who
left at payment returns to payment, and the entitlement is checked first (spec §1.6). Confirmed
Your week rows are never lost (`onboarding-evidence.md §1.3`).

### 3.4 Skip paths, each with its cost named

Every skip goes through one pane with the cost in one sentence and two buttons ("Skip for now",
"Back"). The source then leaves a **Not on the board** line under the strip, saying what it would
add. The reveal repeats each line once. Nothing skipped is asked again in a nudge (spec §1.7).

| Source | Skippable | Skip pane | Not-on-the-board line |
|---|---|---|---|
| Account, payment, Your week | No | none | none |
| School | No; the feed link is the way around | `O2f`: "With the feed link, your assignments and due dates come in, but not your grades, registrar class times, syllabi or what you've already turned in. School email would need its own sign-in." | "Feed link only: no grades, class times, syllabi or turned-in check." |
| School email | Yes | "Without school email, date changes and new work your professors announce by email are missed." | "School email: date changes by email would land here." |
| Google | Yes | "Without Google, club, job and internship tasks from Gmail are missed, and your free time ignores your own events." | "Gmail and Calendar: club and job tasks, and your own events, would land here." |
| Another calendar | Yes | "Without a calendar, your free time ignores your own events." | "Your calendar: your own events would land here." |
| zyBooks / VHL | Yes, per site | "Without VHL, its due dates are missing unless Blackboard's feed carries them." | "VHL: SPAN 202 practice due dates would land here." |
| Syllabi | Yes, per course | "Without CS 110's syllabus, there are no exam dates beyond the feed, no grade weights and no late rule for CS 110." | "CS 110 syllabus: exam dates, weights and the late rule would land here." |

### 3.5 A failed login, and Duo

- Before the school window opens, `O2n` says "Have your phone ready for Duo"
  (`onboarding-evidence.md §4.1`: each Duo push adds about 8 s, and 4.35% of attempts fail because
  the Duo step wasn't finished).
- The password is typed only on the school's own page. Knowlu never draws a field for it, and shows
  nothing over the school's own wrong-password message.
- Knowlu turns the three endings it can see into cause plus fix, never "failed", never blame
  (`stripe.md §1.7`). Each pane keeps a small support code under the fix.
- Jones: "We really have to shine when things go awry" [F1]. On the board a failure never empties
  the strip: what already landed stays.

| Pane | Headline | Body | Buttons | Code |
|---|---|---|---|---|
| `O2-x-closed` | The sign-in window closed. | Nothing was saved. Halverson State's page opens again at the start. | Open it again · Finish later | `SCH-WINDOW-CLOSED` |
| `O2-x-duo` | Duo didn't get an answer from your phone. | Duo stops waiting after about a minute. With your phone unlocked and nearby, try again. | Try again · Finish later | `SCH-MFA-TIMEOUT` |
| `O2-x-locked` | Halverson State says your account is locked. | Your school's IT help desk can unlock it. Knowlu keeps your place; nothing else is asked until then. | Finish later · Use the course feed link instead | `SCH-LOCKED` |
| `O7-x` | VHL didn't accept the sign-in. | Try again, or skip VHL for now; its due dates will be missing until it's connected. | Try again · Skip VHL | `VHL-AUTH-REJECTED` |
| `O5f` | Gmail's read didn't work. | Knowlu can bring Gmail in by forwarding instead, about 3 minutes. | Set up forwarding · Skip Gmail | `GMAIL-READ-FAILED` |
| `O3-x-closed` | Checkout closed before the trial started. | Everything on the board stays. Nothing was charged. | Open checkout again · Finish later | `PAY-CHECKOUT-CLOSED` |

The codes are proposed support strings, not engine contracts.

### 3.6 A slow first run

- A source that is reading shows its expected time on the sources line: "Grades · Reading, usually
  under a minute" (`stripe.md §1.8`).
- Past its expected time, the word changes once and names the part still running: "Grades · Still
  reading. Early in a term this can take longer; you can keep going." It never becomes a spinner and
  never turns into Needs you unless the student has something to do.
- A source that fails in the background moves to **Needs you** with cause plus fix, and the step
  band gains one line above the step: "1 thing needs you: VHL. Sign in again." The current step
  stays where it is.
- The strip keeps its "Boarding soon" line for every source whose departures are not in yet, so a
  slow source is named in words, not hinted at by an empty column.

---

## 4. Onboarding copy, step by step

Rules (BRAND-VOICE): lead with what the student gets; numbers over adjectives; sentence case;
contractions; "about" for estimates; "7:19 pm"; no "just", "easy", "simply", "don't worry",
exclamation marks or emoji. The **benefit lines are the spec's wording** (spec §1.3), which Quinn
reads as final copy; this direction does not rewrite them. Proof lines use Maya's numbers from
`sample-student.json`. The **strip** column says what lands on the board when that proof lands.

### 4.1 Every step

| Step | Benefit line (before the ask) | The ask (button) | Privacy line (at the ask) | Proof line (after) | What lands on the strip |
|---|---|---|---|---|---|
| `O0` Welcome | "Knowlu answers one question every morning: what's next? Next, you'll connect the places that already know your work, about 10 minutes." | Start with your account | none | none | the empty week, "Your week will build here." |
| `O1` Your account | "Your plan lives in your account, so it's safe if this computer isn't." | Continue with Google · Email me a code | "Google shares your name and email, nothing else." | "Signed in as maya.okafor.demo@gmail.example." | nothing; the ticket line appears |
| `O2` Your school | "One school sign-in lets Knowlu read your classes, class times, assignments, grades and school email. You won't sign in to these again." | Sign in at Halverson State | "You sign in on Halverson State's own page. Knowlu keeps the session on this computer, never your password." | "5 classes this term; class times for 5; 26 assignments, 4 already turned in; grades for 5 courses; syllabi found for 4." | 6 departures across the week (Blackboard), 7 class meetings as dotted marks, "Arrived: 4, archived" |
| `O3` Payment | "Everything you just connected keeps running every day for $9.99 a month, after 7 days free." | Continue to checkout | "You pay us, so your data isn't the product. Cancel any time in Settings." | "Your trial is on. First charge October 25." | nothing; the ticket line is used up |
| `O4` School email | "Professors change dates by email. Knowlu reads what arrives and proposes each change for you to approve." | Turn on school email | "Nothing new to sign in: your school sign-in covers it." | "Forwarding on. Test message arrived in 38 s. New mail is read as it arrives; older school mail is still coming in." | nothing yet; a "Changes by email will arrive here" note sits under the strip until the first one |
| `O5` Google | "Clubs, jobs and internships write to your Gmail, and your calendar says when you're already busy. One Google permission covers both, read-only." | Continue with Google | "Read-only. Gmail brings in the last 7 days of mail. Google will first say Knowlu isn't verified yet: choose Advanced, then continue." | "Read 40 recent messages; 1 thing to do (Pre-Health dues, added). 6 events this week, 3 repeating ones to check on Your week." | Haircut (Thursday 5:00 pm) and Family video call (Sunday 7:00 pm) as solid marks; the library job, Pre-Health Society and soccer as dotted marks; dues in Later (Friday Oct 30, 5:00 pm) |
| `O6` Another calendar | "If your calendar isn't Google's, add it so your free time is honest." | Add calendar link | "Knowlu reads the link's events. It never writes to your calendar." | "9 events this week." (not shown for Maya) | that calendar's events as marks |
| `O7` Homework sites | "Homework sites hold due dates your school's feed doesn't show." | Sign in to zyBooks · Sign in to VHL | "This sign-in is stored only on this computer, in Windows Credential Manager." | "zyBooks: 1 book, 8 assignments, 3 already done." / "VHL: 1 course, 6 assignments, 2 already done." | zyBooks: Lab 6 (Friday 11:59 pm), with 6.1-6.6 participation landing as Turned in and leaving. VHL: Lección 5 Vocabulario (Monday 11:59 pm) and Gramática 5.2 (Wednesday 11:59 pm). "Arrived: 9, archived" |
| `O8` Syllabi | "Your syllabus has every exam date, how each course is graded, and the late-work rules." | Choose CS 110's syllabus | "The file is read to find dates, weights and the late rule. It stays in your plan folder on this computer." | "Read 5 of 5: 10 exams, weights for 5, late rules for 4 (ENG 210's syllabus has none)." | exams as marks with an "Exam" tag (none fall in October 19-25; Later shows "CHEM 231 Exam 2, Thursday Oct 29") |
| `O9` Your week | "Check your classes. Everything here can be changed later." | Confirm your week | "Everything confirmed here can be changed later, where you see it." | the confirmed week | dotted marks turn solid as rows are confirmed |
| `R1` Here's your week | "Here's your week." | Open Knowlu | "Your plan is plain files on this computer. Show me the folder" | see §4.5 | the full board |

The strip counts after each step, for Maya: after school 6 departures, 4 arrived; after Google 6
departures and 1 in Later; after homework sites **9 departures, 9 arrived**. That matches the
reveal's week, which has 9 due items Monday to Friday.

### 4.2 The sources line, word by word

The sources line under the step band reads left to right in the order sources were connected. Each
source has one word (Stripe's requirement states, `stripe.md §1.2`):

| Word | Shape | Example |
|---|---|---|
| Reading | three short dashes | "Grades · Reading, usually under a minute" |
| Done | check | "Course list · Done: 5 classes (2 last-term classes left out)" |
| Needs you | hollow ring | "VHL · Needs you: sign-in not accepted" |
| Not on the board | dashed outline | "VHL · Not on the board (skipped)" |

"Boarding soon" names **future** departures in words, under the strip. It is never a state of a
source already being read:

- after `O2`: "Boarding soon: your own events, after Google. Homework-site due dates, after homework
  sites."
- after `O5`: "Boarding soon: zyBooks and VHL due dates, after homework sites."
- after `O7`: "Boarding soon: exam dates from CS 110's syllabus."
- after `O8`: the line is gone.

### 4.3 Waits and failures in the step band

| Moment | Copy |
|---|---|
| School reading, first 60 s | "Reading your school. You can go on to payment; the board keeps filling." |
| School reading past 3 min | "Still reading grades. Early in a term this can take longer; you can keep going." |
| School email canary late (3 min) | "The test message hasn't arrived yet. It usually takes under a minute. Knowlu keeps checking; you can go on." [Send another] |
| Google partial grant | "Gmail connected. Calendar not shared, so your free time won't count your own events." [Share calendar] [Continue without it] |
| Syllabus unreadable | "CS 110's file didn't read as a syllabus. Choose another file, or skip CS 110 for now." |

### 4.4 `O3` Payment: the board is the order summary

Stripe's checkout guidance asks for a transparent order summary and one visible pay button
(`stripe.md §3.1`). Here the summary is the board itself, held still above the price:

> **What keeps running**
> Your week so far: 6 departures, 4 already turned in and archived.
> 5 classes, grades for 5 courses, syllabi for 4.
>
> **$9.99 a month after 7 days free.** First charge October 25. Cancel any time in Settings.
> Also: $39.99 for a 6-month term. Pell Grant recipients pay 75% less.
>
> [Continue to checkout]
>
> Checkout opens in your browser on Stripe (checkout.stripe.com).

The ticket line under it shows the same price it has shown since `O1`, so nothing on this screen is
new.

### 4.5 `O9` Your week

- Prefilled rows in seven groups (Classes, Exams, Grading, Late work, Your week, Office hours, Your
  day), each row with its source ("registrar", "syllabus", "Google Calendar, repeating") and two
  controls: **Confirm** (check) and **Edit**. Typing is the last resort, inside Edit. "Confirm all in
  this group" sits on each group header.
- The strip stays visible above: confirming "BIO 201 lab: Thu 2:00-4:50 pm, Kessler Hall 310" turns
  Thursday's dotted lab mark solid. The student sees each answer land on their week
  (`onboarding-evidence.md §1.3`, the IKEA effect only pays when the work lands).
- "ENG 210: no late rule found" offers "Leave blank" as a normal answer: "Past-due work stays on the
  list until you close it."
- Commitment levels are named in words: "Library job: hard (Knowlu never plans over it)", "Pre-Health
  Society: soft (may be overlapped; Knowlu says when)", "Intramural soccer: optional (never counted as
  busy)".

### 4.6 `W1` The wait

The step band becomes a countable list (`onboarding-evidence.md §1.4`, §3.9). The strip stays.

> **Putting your week in order. 8 of 9 done.**
> Syllabi · Reading CS 110's syllabus... found the late rule.
> Effort and order · Estimating 34 assignments.

- Lines come from the engine's step records, never a scripted ticker.
- Maya's case: everything ran once after 40 s, when CS 110's syllabus finished, and `R1` opens.
- At 180 s, `R1` opens anyway, and anything still running becomes a "Still coming" line. Backfills
  (older school mail) are never waited for.

### 4.7 `R1` Here's your week: the week's board

Built from the same read model as the main page, so the page then shows exactly what the reveal
promised. Top to bottom:

1. **"Here's your week."** and, under it, "Monday, October 19 to Sunday, October 25."
2. **The week strip at full height**: seven columns with every departure, fixed marks solid, the
   hours-due figure per day. The heaviest day of the strip carries its figure in board type.
3. **What's next** (the live card, first appearance): "Reading: Ch. 8, Metabolism is next. BIO 201.
   Due tomorrow at 9:00 am. About 30m. Why: it's due before BIO 201, and it's the first thing due
   this week."
4. **Tonight**: "About 3h free tonight, after 9:00 pm." and the two items due by tomorrow night as
   board rows.
5. **What Knowlu tidied**, one line each: "52 past items and 9 already turned in were archived."
   "Effort estimated for 34 assignments." "2 classes from last term left out."
6. **From your email**: "Pay Pre-Health Society dues ($15), due Friday, October 30 at 5:00 pm. From
   Pre-Health Society's email, October 12. Undo"
7. **Still coming**: "Older school email is still coming in; anything it finds will be proposed."
8. **Not on the board** (empty for Maya; otherwise one line per skipped source, §3.4).
9. **"Knowlu checks again at 12:00 and 6:00, and starts with Windows."** and **[Open Knowlu]**.

Stripe's summary step lets each section reopen its own editor (`stripe.md §1.6`): clicking any
departure, mark or tidied line opens it in place and returns to the reveal.

---

## 5. The main page (`M1`): one live card plus the board

Maya, Monday October 26, 8:15 am (term week 10). Every value below is from `today` in
`sample-student.json`. Every name in `code` is a read-model field from `docs/surface/anatomy.md`.

### 5.1 Layout at 1280x800

```
+--------------------------------------------------------------------------------------------+
| Knowlu  Monday, October 26 · Week 10 of 16        In step · checked 6:02 am · VHL needs you |  topline, 40px
| Mon 26  2.2h | Tue 27 2.5h | Wed 28 2.4h | Thu 29 6.9h Exam | Fri 30 10.2h | Sat | Sun | Later  week strip, 84px
| Cell Bio · Org Chem 2h behind · Programming 1h behind · Spanish 1.5h ahead · ...    domains, 36px
+------------------------------------------------------+-------------------------------------+
| LIVE CARD (what's next, why)                         | Fixed today (commitments, free time)|
| verdict line                                         | Needs you (one card, deck)          |
| TODAY'S BOARD (rows in Knowlu's order)               | Changes (Flighty lines)             |
| Done today · Late window closed (2) · Arrived (7)    |                                     |
+------------------------------------------------------+-------------------------------------+
```

Left column 760 px (the board keeps a 72ch title measure); right column 400 px; 24 px gutters. The
right column scrolls on its own. Below 1100 px the right column folds under the board, in the same
order.

### 5.2 Regions, top to bottom

**A. Topline** (`topline`, sync/backup/scheduler line). Ambient. "Monday, October 26 · Week 10 of
16". Right side, one health phrase: "In step with your account · checked 6:02 am". When a source
needs the student, the phrase grows by one clause with the hollow ring: "VHL needs you". Calm
positive state by default, growing only when action is needed (`stripe.md §2.1`); never silent
(principle 9).

**B. Week strip** (`ahead`, first 7 of 14 days, plus a Later column for days 8-14). The same strip
that was built in onboarding, now for this week. Hours due per day as the header figure and bar
(Mon 2.2h, Tue 2.5h, Wed 2.4h, Thu 6.9h, Fri 10.2h, Sat 0.25h, Sun 0). Exams as tagged marks (Thu:
"CHEM 231 Exam 2, 9:30 am, Hallett Hall 120"). The generated takeaway (`ahead.takeaway`) sits under
the strip in small type: "Friday is the heaviest day in the next two weeks, about 10.2h due." The
strip is compact here (departure lines hidden, counts only); clicking a day opens that day's board.

**C. Domains** (`domains`, VISION's strip). One chip per domain with a small grade ring (`grades`
pct and letter) and its pace in hours: "Organic Chemistry · B- · about 2h behind". Clicking a chip
filters the board. Pace words are plain, in ink, never amber: being behind is a plan, not an alert.

**D. The live card** (`today.list[0]`, its `why`). Foreground. One card, full left-column width:

> **Lab 6: Functions is next.** CS 110 · zyBooks
> Was due Friday at 11:59 pm. **−30% now; closes Wednesday at 11:59 pm.**
> About 1h 12m left (40% done).
> Why: each day late costs 10%; finishing today keeps it at −30%.
> [Start] [Done] [Pin] [Open in zyBooks]

The card's big board figure is **the close time, "Wed 11:59 pm"**: a deadline, never a start-by.
For an item due within 24 hours the card may add "in about 15 hours", rounded down to the hour and
refreshed every 15 minutes; under one hour it says "within the hour". It never shows minutes or
seconds counting down. Flighty's 15 "smart states" change what its card shows as the departure
nears [F3]; here the card has three: **due later** (the day word and time), **due within 24 h** (adds
the rounded phrase), **past due, inside the window** (the penalty now and the close time, as above).

**E. Verdict line** (`verdict`). One generated sentence under the card, with a 30-minute-block
meter beside it (`meter`): "About 4.7h is due by tomorrow, and today has room for it. After today's
work, about 6h is yours, after 6:00 pm." Spill blocks, when any, are outlined in ink, never red.

**F. Today's board** (`must_do`, `recommended`, `today.list[1..]`). Header: "Today's board · In
order: what's next first · 6 departures · 6.5h". Rows (§1.2):

| # | Due | Course | Title | Left | Status | Source |
|---|---|---|---|---|---|---|
| 2 | 11:59 pm today | CS 110 | zyBooks 7.1-7.4 participation | 24m | On track | zyBooks |
| 3 | 11:59 pm today | SPAN 202 | VHL Lección 6: Práctica de vocabulario | 36m | On track · VHL last read Sunday 6:04 pm | VHL |
| 4 | 9:30 am tomorrow | CHEM 231 | Problem set 7 | 1h 30m | On track | Blackboard |
| 5 | 12:30 pm tomorrow | ENG 210 | Peer review: outline exchange | 1h | On track | Blackboard |
| 6 | 9:30 am Thursday | CHEM 231 | Study for Exam 2 (Ch. 5-7) | 1h 48m today, of 4h | Changed: exam room | Knowlu, from syllabus |

- Rows 2-5 are urgent (due today or tomorrow); row 6 is recommended. A hairline and the label
  "Recommended for today" separate them, with Notion's adjacency spacing: rows in a group tight, the
  gap between groups wide (`notion.md §3.1`).
- Row 6 carries **Changed** because its exam's room moved (`good_to_know` n-room). Opening it shows
  the change line from §1.4.
- Under the board, a fold: "Later: 21 more, through November 8 · Show". Anything can be checked off
  from there (PRODUCT-DECISIONS).
- **Done today** (`done_today`): "Reading: Ch. 9, Cellular respiration · Done by you, 7:55 am",
  struck through, uncheckable, until midnight.
- **Late window closed (2)** (`late_window_closed`). Appears only when non-empty (`notion.md §2.3`).
  Collapsed to one line: "2 past their late window. Archive or keep going." Opened (`M4`):
  "CHEM 231 Problem set 6. Was due last Tuesday at 9:30 am. Late work not accepted after 48 hours
  (your syllabus). [Archive] [Still doing it]" and "SPAN 202 VHL Lección 5: Vocabulario. Was due
  October 19 at 11:59 pm. Late: −10% a day, up to 3 days (your syllabus). [Archive] [Still doing
  it]". "Still doing it" asks "When will you turn it in?" with a day picker.
- **Arrived this week (7)** (`closed_this_week`, rolling 7 days). Collapsed to one line. Each entry
  says who: "Pre-lecture quiz: Ch. 7 · Turned in, seen on Blackboard Sunday 4:40 pm · Undo", or
  "Reading: Ch. 8, Metabolism · Done by you, last Monday".

**G. Right column, top: Fixed today** (`the_day`, `free_time`). Commitments as fixed rows with their
clock time and room, and no checkbox (they are not work): "9:00 BIO 201 lecture, Kessler Hall 104";
"10:00 SPAN 202 lecture, Bellamy Hall 212"; "1:00 pm CS 110 lecture, Ostrander Center 140"; "3:30 pm
CHEM 231 office hours, optional". Under them, free time as a sentence and a thin day bar: "About
5.7h free, after 6:00 pm." Knowlu doesn't place work into those hours.

**H. Right column: Needs you** (`decisions`). A deck, one card at a time; the page never grows with
the queue. Header: "4 waiting · oldest 7 days". The oldest first (`first_proposed_at`):

> **Add "SURF application" to Personal, due Friday, November 6 at 5:00 pm, about 2h?**
> From the Office of Undergraduate Research's email, October 19.
> "Applications for the Summer Undergraduate Research Fellowship are due Friday, November 6 at
> 5:00 pm..." (quoted, shown as untrusted text)
> [Add it] [Not for me] · Not now

Answers take effect at once with Undo. "Not now" sends the card to the back.

**I. Right column: Changes** (`delta` and `good_to_know`). Flighty-form lines (§1.4), newest first,
since the last run: the VHL sign-out ("VHL signed you out this morning. Sign in again to bring in new
VHL work; what Knowlu already has stays. [Sign in to VHL]"), "Lab 8: Dictionaries. Official:
Wednesday, November 4. Email says: Friday, November 6. From Priya Raman, CS 110 TA, Sunday 9:41 pm.
[Move it] [Keep Wednesday]", "BIO 201 Lab 4 report graded: 44/50. Blackboard grades, Sunday 6:00
pm.", "CHEM 231 Problem set 6 is marked missing in Blackboard. It's in Late window closed.", and the
room change. A proposed date change appears both here and in the deck; answering it in one place
answers both.

### 5.3 Named states

| State | What changes |
|---|---|
| All clear (`M7`, Sunday November 1) | The live card says "Nothing is due today. The day is yours." and, small, "Next up: Reading: Ch. 11, Cell signaling, due tomorrow at 9:00 am." The board is replaced by Fixed today at full width. |
| Source needs you (VHL) | Topline clause, the Changes line, and VHL rows show "VHL last read Sunday 6:04 pm". Rows keep their status word; the data is old, not wrong. |
| Offline | Topline: "Offline. Changes stay on this computer and sync when you're back." Nothing else on the page changes (VISION commitment 2). |
| Undo window | After Done: the checkmark, the row strikes, and "Done. Undo" sits in place for 10 s. The next row's due time moves up into the card if it was the card. |
| More due than time | Verdict: "About 6h is due by tomorrow, and today has about 5h free." No advice, no red. Spill blocks outlined. |

### 5.4 The tray flyout (`T1`)

Notion Calendar's menu bar answers "what's next" without opening a window (`notion.md §5.2`). The
flyout (360x200) carries the live card alone: the title, the due or close time in board type, the
estimate, the one-line why, and [Start] [Open Knowlu]. No list, no counts. When nothing is due:
"Nothing is due today. The day is yours."

---

## 6. Visual world: a campus shuttle departures sign

A shuttle sign at a campus stop at night: a deep ground, one line per departure, big clock figures,
a few status words in warm amber. In daylight the same sign is a bright printed board. It is a
sign, not a split-flap toy: **no flipping letters, no clacking, no scanlines, no LED dot texture.**
Each of those is decoration without information (principle 7), and the split-flap animation is the
first thing anyone would reach for with this metaphor.

### 6.1 Type

| Role | Face | Setting |
|---|---|---|
| Board figures (due times, hours-due per day, the card's close time) | **Big Shoulders Text** (Google Fonts), 600-700 | tabular figures; 22 px on board rows, 18 px in strip headers, 52 px on the live card; tracking +0.01em |
| Board labels (section names: "Today's board", "Fixed today", "Changes") | Big Shoulders Text 600 | 17 px, sentence case, never all caps |
| Everything read (titles, why lines, change lines, onboarding copy, buttons) | **Atkinson Hyperlegible Next** (Google Fonts), 400/500/700 | body 15 px / 1.45; small 13 px minimum; step benefit line 20 px / 1.35; 64-72ch measure |
| Hosts and support codes | Atkinson Hyperlegible Mono 400 | 12.5 px |

Big Shoulders is condensed, so figures stay large without widening the Due column (about 88 px).
Atkinson Hyperlegible Next was designed for low-vision readers, which suits a tired student at 11 pm
(principle 8). The prototype must check that Big Shoulders Text sets tabular figures (`tnum`) in the
build. If it doesn't, figures fall back to Atkinson Hyperlegible Mono, and the page must not mix the
two within one column.

### 6.2 Palette as tokens (OKLCH; Stripe's step-distance rule)

Stripe's rule: colours "five levels apart" pass for small text, four for icons and large text
(`stripe.md §4.1`). Every text token below is at least five lightness steps (0.40 L) from its
ground. Both modes are designed (principle 15); dark is the night sign, light is the day board.

| Token | Dark (night sign) | Light (day board) | Job |
|---|---|---|---|
| `--ground` | `oklch(0.17 0.025 258)` | `oklch(0.972 0.006 85)` | page |
| `--board` | `oklch(0.205 0.028 258)` | `oklch(0.993 0.003 85)` | strip, board, card surface |
| `--rule` | `oklch(0.95 0.01 258 / 0.09)` | `oklch(0.25 0.02 258 / 0.09)` | 1 px grid and row lines (Notion's ~9%) |
| `--ink` | `oklch(0.95 0.012 85)` | `oklch(0.23 0.025 258)` | text, figures |
| `--ink-2` | `oklch(0.79 0.015 85)` | `oklch(0.45 0.02 258)` | secondary text (never fainter) |
| `--amber` | `oklch(0.81 0.13 75)` | `oklch(0.52 0.115 65)` | Changed, Late window open: word and shape |
| `--amber-fill` | `oklch(0.81 0.13 75 / 0.14)` | `oklch(0.93 0.05 80)` | the status word's pill |
| `--needs` | `oklch(0.80 0.09 235)` | `oklch(0.48 0.11 245)` | the Needs you ring (word stays `--ink`) |
| `--brand` | `#BC4444` | `#BC4444` | wordmark and primary button only; never a state |
| `--focus` | `oklch(0.86 0.12 85)` | `oklch(0.45 0.13 258)` | 2 px focus ring, offset 2 px |

Course tints are soft fills with dark labels, identity only, never a state (`notion.md §3.3`).
Chroma stays at 0.05 in fills, and the course code is always printed on the fill. No course is red,
and none sits on the amber hue:

| Course | Hue | Fill (dark / light) |
|---|---|---|
| BIO 201 | 160 | `oklch(0.34 0.05 160)` / `oklch(0.93 0.04 160)` |
| CHEM 231 | 300 | `oklch(0.34 0.05 300)` / `oklch(0.93 0.035 300)` |
| CS 110 | 205 | `oklch(0.34 0.05 205)` / `oklch(0.93 0.035 205)` |
| SPAN 202 | 120 | `oklch(0.34 0.05 120)` / `oklch(0.94 0.04 120)` |
| ENG 210 | 330 | `oklch(0.34 0.045 330)` / `oklch(0.93 0.03 330)` |
| Clubs, job, Personal | 258 (neutral) | `oklch(0.30 0.02 258)` / `oklch(0.94 0.008 258)` |

### 6.3 Density and spacing

- Spacing scale 4 / 8 / 12 / 16 / 24 / 32 / 48. Board rows 44 px; a "Compact board" setting gives
  36 px. Strip lines 22 px.
- No card boxes inside the board: rows are separated by the 1 px rule, groups by a 24 px gap. The
  live card is the only raised surface on the page (one 1 px border plus `--board`; no shadow in
  dark mode, a 0 1px 2px shadow in light).
- Hit targets at least 32x32 px. Row actions (Done, Pin, Open) appear on hover **and on focus**, and
  are reachable by keyboard: `j`/`k` move, `x` done, `p` pin, `Enter` opens, `?` lists keys
  (`notion.md §5.1`; `log.md` D4: optional accelerators, every action has a visible control).

### 6.4 Motion

Villar's test for Stripe Checkout: if turning an animation off makes the flow feel broken, it was
doing work (`stripe.md §4.2`).

| Moment | Motion | Time | Reduced motion |
|---|---|---|---|
| A proof lands (onboarding) | Checkout's success checkmark, stroke drawn once | 240 ms, exponential ease-out | the check appears drawn |
| A task marked done | the same checkmark in the row's status cell, then the title strikes | 240 ms | instant |
| A turned-in item leaves the strip | it holds 1.5 s with "Turned in", then its line collapses into the Arrived count | 180 ms collapse | it never appears; the count changes |
| A status word changes | crossfade | 120 ms | instant |

That is the whole list. There is no error shake (`stripe.md §4.2`), no flap, no countdown animation
and no page-load sequence. The board never moves rows on its own while the student is looking. A
re-rank after a run shows as a one-line "Board updated at 12:00. Problem set 7 moved up: Blackboard
changed its due time." with the reason, and the rows settle when the student next scrolls or acts.

---

## 7. Risks, and what this brief does about each

| Risk | Answer in this brief |
|---|---|
| Countdowns create the urgency the brand forbids (BRAND-VOICE principle 7, "talk about when things are due, not when to start") | Figures are due clock times; the only relative phrase is rounded to the hour and refreshed every 15 minutes; no start-by anywhere; no ticking (§5.2 D). |
| The metaphor reads as "you missed your flight" | No "missed", "gone", "too late". Late inside the window stays on the board with the penalty in numbers. Late window closed is a calm group with two equal choices (§5.2 F). |
| A departures board sorts by time; Knowlu sorts by priority | The header says "In order: what's next first" and the Order column is printed (§1.2). To test: whether students read the Due column as the order. |
| Amber everywhere becomes the new red | Amber only on Changed and Late window open; On track is plain ink; pace words never amber (§5.2 C). If amber shows on more than two rows of a normal day, the board is mis-tuned. |
| The strip during onboarding draws the eye away from the ask | The step band is always on top and larger; the strip only changes when a proof lands, and it has no motion besides the Turned in fold (§6.4). |
| The ticket line feels like a meter running | It states a price and when it starts, nothing that counts down; it leaves after payment (§2). |
| Two places show a proposed date change (deck and Changes) | One object rendered twice (`notion.md §2.1`); answering either answers both, and the Changes line then reads "You moved Lab 8 to Friday, November 6. Undo". |

## 8. What is taken from each reference, and where it stops

### 8.1 Notion (`../research/notion.md`)

| Taken | Where it lands here | Citation |
|---|---|---|
| The menu bar: "what's next" with no window opened, because checking "may feel harmless, but this context-switching can be detrimental" | The tray flyout carries the live card alone (§5.4) | `notion.md §5.2` |
| The calendar grid recipe: 1 px lines at about 9% opacity, tabular time labels, soft course fills, no decorative containers | The week strip (§1.1) and the board's rules (§6.2 `--rule`) | `notion.md §5.4` |
| Deadlines on the time axis, links "available, not visible" | Departures inside day columns, commitments as fixed marks; detail opens on click | `notion.md §5.3` |
| The preview builds as you answer | The strip is built in front of the student, one proof at a time (§3.2) | `notion.md §1.2` |
| Adjacency spacing | Urgent rows tight, a wide gap before "Recommended for today" | `notion.md §3.1` |
| Sections appear only when they have something in them | Late window closed, Needs you, Not on the board | `notion.md §2.3` |
| The block model: one object, many renders | One task as strip line, board row, live card, tray card | `notion.md §2.1` |
| Keyboard so the UI can stay minimal | Row chrome hidden until hover or focus; `j`/`k`/`x`/`p` | `notion.md §5.1` |

**Where Notion stops.** No user-arranged layout (the board's order is the product's judgment), no
covers or emoji, no hover-only controls, no drag-to-create time blocks (`notion.md §7`, §5.3).

### 8.2 Stripe (`../research/stripe.md`)

| Taken | Where it lands here | Citation |
|---|---|---|
| Many system states collapse into a few human labels, detail one click away | Five row words (§1.3); four source words (§4.2) | `stripe.md §2.2` |
| Badges typed by what the reader must do, read-only | The precedence rule: Needs you > Late window open > Changed > On track; words never act, buttons do | `stripe.md §2.3` |
| Checkout's success checkmark as motion that does work; **not** the error shake | The only authored motion: proof landing and task done (§6.4) | `stripe.md §4.2` |
| Requirement states that keep "checking" apart from "you must act" | Reading vs Needs you on the sources line; a slow source never becomes Needs you (§3.6) | `stripe.md §1.2` |
| Pending states carry a time expectation | "Reading, usually under a minute" (§4.2) | `stripe.md §1.8` |
| Resume rendered from what is outstanding; Save for later is honest | `BOOT`, `X1`, `X2` (§3.3) | `stripe.md §1.1`, §1.4 |
| The order summary and one pay button | The board so far is the summary above $9.99 (§4.4) | `stripe.md §3.1` |
| Errors as cause plus fix, with a code | The failure panes (§3.5) | `stripe.md §1.7` |
| The notification banner exists only when action is needed | The topline's health phrase grows a clause only for Needs you (§5.2 A) | `stripe.md §2.1` |
| Summary step with per-section edit | Any reveal item opens in place and returns (§4.7) | `stripe.md §1.6` |
| Accessible colour by perceptual steps | OKLCH tokens with the step-distance rule (§6.2) | `stripe.md §4.1` |

**Where Stripe stops.** No operator density on Today; no red for negative states; no animated
gradient (`stripe.md`, "Where Stripe does not transfer").

### 8.3 Flighty (the third reference)

Sources, read 2026-10-23:
- [F1] Apple Developer, "Behind the Design: Flighty" (Ryan Jones, founder):
  https://developer.apple.com/news/?id=970ncww4 . Quotes: "We want Flighty to work so well that it
  feels almost boringly obvious." "Those airport boards have one line per flight, and that's a good
  guiding light — they've had 50 years of figuring out what's important." "There's something
  comforting about information always being there." "We really have to shine when things go awry."
- [F2] flighty.com (product page): status lines "Gate changed", "Flight changed", "Delayed 45m";
  "New Departure: 8:28pm (was 11:30pm)"; "Changed to ORD Terminal 2 • Gate 7"; "Good morning! Your
  flight today is on time."; imports flights "from calendar, TripIt, or email"; "Never stop and stare
  at a flight board again."
- [F3] Blake Crosley, "Flighty" design guide (independent analysis; its code is illustrative, not
  Flighty's): https://blakecrosley.com/guides/design/flighty . 15 context-aware states chosen by time
  to departure; status colours green, amber, red; the countdown as the heaviest type in the system.
- [F4] TechCrunch, 2024-08-06, delay predictions: notification "Delayed 45m. Due to late arriving
  aircraft from New York."
  https://techcrunch.com/2024/08/06/flightys-popular-flight-tracking-app-can-now-predict-delays-using-machine-learning/
- [F5] Flighty Help Center, delay predictions: Flighty shows its predicted times alongside the
  official ones. https://flighty.com/help/delay-predictions
- [F6] `../research/landscape.md` row 8: Flighty as a "what's next" for travel that ingests from
  email and gives an honest reason with each change.

| Taken | Where it lands here | Source |
|---|---|---|
| One line per departure, essential fields only, the board's 50 years of editing | Board rows (§1.2): seven fields, one line, one status word | [F1] |
| A change stated as new value with the old in brackets, plus the cause | The five-part change line (§1.4) | [F2], [F4] |
| Predicted shown beside official, never replacing it | "Official: ... Email says: ..." for proposals from email (§1.4, §5.2 I) | [F5] |
| Information that is always there | The ticket line in onboarding; the tray card; the strip at the top of every day | [F1] |
| The card changes what it shows as departure nears | Three card states, cut down from Flighty's 15 (§5.2 D) | [F3] |
| Shine when things go awry | Failures never empty the board; each says cause, fix and what stays (§3.5) | [F1] |
| Import from what already knows (calendar, email) | The ruled onboarding already does this; the board makes each import visible as departures | [F2], [F6] |
| A morning greeting that is a status | The verdict line, without the exclamation: "About 4.7h is due by tomorrow, and today has room for it." | [F2] |

**Where Flighty stops.**
- **No live countdown as the hero.** Flighty makes its countdown the heaviest type [F3]; a student's
  due time is not a gate closing in 32 minutes. Knowlu's hero figure is a clock time.
- **No red, no green.** Flighty's green/amber/red [F3] would turn red into an alarm (principle 9)
  and green/red into colour-only meaning (principle 10). Knowlu keeps amber plus shapes.
- **No "first to know, first to go".** Flighty sells speed of alert ([F2]'s tagline); Knowlu's
  reminders are rare and calm (BRAND-VOICE, Reminders). No alert sound.
- **No delay predictions.** Knowlu never predicts that the student will be late; it states estimates
  and due times.
- **No passport or lifetime stats.** Gamified history has no job on a study list.

---

## 9. Open questions for Quinn (one at a time, in this order, each with a recommendation)

1. **Status word for past due inside the window.** The run's thesis said "Window closing". This
   brief uses **"Late window open"**: "closing" adds urgency the numbers already carry ("closes
   Wednesday at 11:59 pm"). Recommendation: "Late window open".
2. **Arrived vs Turned in.** Onboarding stamps lines "Turned in" and counts them as "Arrived"; the
   main page lists "Arrived this week". Recommendation: keep "Turned in" on every line (plain), and
   use "Arrived" only as the collective name.
3. **The relative phrase on the live card** ("in about 15 hours"). Recommendation: keep it on the
   card and tray only; drop it entirely if a tester reads it as a timer.
4. **The ticket line.** Recommendation: keep it from the account step to payment. Its job is
   up-front price disclosure (`onboarding-evidence.md §3.5`).
5. **Big Shoulders in light mode.** A condensed display face on a bright board can read as signage
   more than software. Recommendation: judge it in the prototype, side by side with Atkinson
   Hyperlegible Next figures, in both modes.

---

## 10. Build notes for the prototype (this folder only)

- One `index.html` with a screen picker listing every id in §3.1, a light/dark toggle, a reduced
  motion toggle, a density toggle (44/36 px), and "Copy my settings" (COLLABORATION.md).
- Data from `../shared/sample-student.json` (copied in, read locally); no network except Google
  Fonts (Big Shoulders Text, Atkinson Hyperlegible Next, Atkinson Hyperlegible Mono).
- The wordmark from `assets/Knowlu_Wordmark_Red.png`.
- Strip state is computed from which sources have landed, so `O2r` -> `O7` replays the board
  growing, and `X2` shows resume with the same strip.
- Examples in §1.4 that are not in the sample data (a Blackboard due-time change, a student's
  estimate edit, the 12:00 re-rank line in §6.4) are illustrative only and must not appear in the
  prototype's Maya screens.
