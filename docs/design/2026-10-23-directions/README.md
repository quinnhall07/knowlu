# Five directions for the whole student journey (2026-10-23)

Five prototypes, each covering the ruled long onboarding (every login inside it, a benefit line
before each one, proof after it), the "Here's your week" reveal and the daily "what's next" page.
Every direction starts from Notion and Stripe and adds one third reference that no other direction
uses. All data is invented (Maya Okafor, Halverson State University, `shared/sample-student.json`).
The ruled facts come from `docs/specs/2026-10-23-onboarding-redesign-draft.md`.

## How to open them

- **Start with `index.html`** in this folder. It is the comparison page: each direction's thesis, its
  borrowings, four key screenshots (login step, proof, reveal, main page) that link into the
  prototype, its weaknesses after round 2, and a side-by-side table. Double-click it, or open
  `file:///C:/Users/danie/GitHub/knowlu/docs/design/2026-10-23-directions/index.html` in Edge.
  Append `?theme=light` or `?theme=dark` to force a theme.
- **Each prototype** is one self-contained `index.html` in its folder, built for a 1280x800 window.
  Any screen opens directly at `index.html#<screen-id>`. The ids are in that folder's
  `screens.json`, and the PNG for each is in `shots/` (re-shot after round 2).

| Folder | Direction | Third reference | Score, round 1 → 2 | Good first screens |
|---|---|---|---|---|
| `1b-concierge/` | A conversation that hands you the list | Superhuman's 1:1 onboarding | 58 → 64 | `#S-need` `#S-proof` `#R` `#T` |
| `2-one-thing-per-screen/` | Setup, one screen at a time | Apple Setup Assistant | 63 → 66 | `#S1` `#S3` `#R0` `#R1` `#T` |
| `3-week-ledger/` | The week as a balance | Shovel | 60 → 61 | `#o3a-need` `#o3c-proof` `#o12-reveal` `#m1-today` |
| `4-connector-hub/` | Connections hub, one sheet per source | Plaid Link | 61 → 66 | `#H` `#P-SCH-NEED` `#P-SCH-PROOF` `#R-REV` `#M-TODAY` |
| `5b-departures-board/` | Departures | Flighty | 65 → 66 | `#o2n-before` `#o2p-proof` `#o9-your-week` `#r1-reveal` `#m1-today` |

Extra switches: in 2, press `\` or the Screens button for the state jumper, and use `~dark` /
`~light` (`#T~dark`). In 3 and 4, add `?shot` / `?shot=1` to hide the prototype controls; in 4,
add `&theme=dark`. In 5b, add `?theme=light` and `?shot=1`. Each folder's `BRIEF.md` is its build
contract and lists what changed after each red-team round.

`1-checklist-to-today/` and `5-three-lanes/` are first passes that 1b and 5b replaced. They are kept
for the record and are not compared. `research/` holds the shared Notion, Stripe, onboarding-evidence
and landscape research. `concepts.md` has the seven candidate concepts and the ruled floor every
direction keeps.

The scores come from two critique passes, each using one rubric for all five. After round 2 they sit
within 5 points (61-66), and three directions tie at 66. That is inside critic noise, so the scores
don't decide the question. Round 2's critique was time-boxed: each critic inspected about five
screens per direction, and error, empty and skip states were mostly not re-checked.

## Recommendation

**Keep 5b, Departures, as the spine. Round 2 fixed its daily page, so the next piece of work is the
reveal, then the wait.**

The picture changed less than the scores did. 5b is still the only direction whose structure runs
through all three stages. A rail names each step's benefit before the login and its proof after. A
week board, one row per day, first appears at the school proof already holding counts, and gains
items at every later proof, in counts and course codes only. Payment waits for that proof. The daily
page is now that board too, which round 1 said it wasn't. "Lab 6 is next." and one checked sentence
("About 4.7h is due by tomorrow, and today has room for it") sit over one departures board: Due ·
Course · Assignment · Left · Status, the next row enlarged with its why and its late cost, and
"Recommended for today" as a divider row on the same board. "Free from 2:15 pm" is now computed and
agrees with the day bar.

Its weakness has moved. **The reveal (`r1-reveal`) drops the board for a 7-column calendar grid**,
on the one screen meant to pay off nine steps of watching that board fill. The reveal should be the
same rows the student watched gain counts, now with titles and hours. The wait (`w1-wait`) is also
still the round-1 version.

Why it still leads over the two it ties with: 4's round-2 board is a real structure now (ranked,
"Suggested next" on top, discovered sources indented under the school), but it ends where it started,
in a generic calendar reveal and a three-column console, and the hub is a picture of the setup, not
of the week. 2 is still the plainest and clearest, and still the closest to a conventional wizard.

### Grafts: what round 2 took in, and what is left

Round 2 already took four of round 1's grafts into 5b, each in 5b's own form: 2's headline ("Lab 6
is next."), 1b's list-first page (as the board), 3's per-day "due inside open time" bar read at the
tightest day (without the 71.5h total), and 4's update-mode repair ("VHL signed you out. Earlier VHL
work stays; new work waits. Sign in again").

Still to graft:

1. **From 2 (One thing per screen):** R0's "Building your week" list (countable lines, "12 of 12
   done", 3-minute cap) as the model for 5b's wait. Fix 2's own count clash ("11 of 12" against "7
   of 7") when copying it.
2. **From 4 (Connections hub):** "Found at your school" with its reason (CS 110's Blackboard link,
   SPAN 202's syllabus) on the rail rows for zyBooks, VHL and the syllabus, so the student sees why
   those logins are asked for. 5b's own three-row pre-login table stays; it is the clearest in the set.
3. **From 1b (Concierge):** S-proof's ruled rows, each saying what was left out and why ("2 from last
   term left out", "CS 110's isn't in Blackboard"), under 5b's proof sentence.

### Round 3 (5b only, 2026-10-23 evening)

Round 3 did most of the list below. Its details are in `5b-departures-board/BRIEF.md` under "Round 3", and screens were re-shot:
- **The reveal (`r1-reveal`) is now the board.** It uses the same per-day rows as setup, now with titles and hours, and its button reads "Start with Reading: Ch. 8".
- **The wait (`w1-wait`) is a countable "Building your week" list.** A new `w1-cap` screen shows the 3-minute cap.
- **Daily page (`m1-today`):**
  - a late item gets its own "Late · window closes" cell;
  - the column reads "Work left", and every time uses one format;
  - Done is the main action on every row;
  - the VHL alert is in the top bar, and the right column is quieter.
- **Grafts:** the proof screen lists what was left out and why, and the rail says "Found at your school."

Still open: the 11 pm ordering question (a ranking rule for Quinn); on the reveal, the weekend rows fall below the fold at 1280x800; the right column is still long. Round 3 was not re-scored. The list below is kept as it was written before round 3.

### What to fix in 5b before anyone sees it

- **Make the reveal the board.** Same rows as setup and the daily page, filled in with titles and
  hours. Keep the live card, Tonight, and the Tidied / From your email / Still coming row. Replace
  "Open Knowlu" (said inside Knowlu) with "Start with Reading: Ch. 8".
- **Late items need their own Due cell.** Lab 6 shows a large "11:59 pm" with a small "Was due Fri",
  which reads as "due in an hour" at 11 pm. Show "Late" and the window's close instead.
- **Say what "Left" means.** "24m" beside 11:59 pm reads as a countdown. Use "Work left", and pick one
  time format (the page mixes "4.7h" with "1h 12m").
- **Make Done the primary button on the lead row.** "Lock in" is filled red on a late row, which reads
  as an alarm, and the page never says what it does.
- **Give every row a visible action target.** Done, Pin and Open appear only on hover.
- **Lower the right rail.** The Needs-you card (three buttons) competes with the next row, and the day
  bar needs a four-line legend to read. "This week" and "Since last night" are below the fold.
- **Rebuild the wait (`w1-wait`)** as the countable list above.
- **Decide the 11 pm order.** At 11 pm, two items closing at 11:59 pm should probably outrank a late
  lab whose window runs to Wednesday. No direction shows the evening case; 5b, 2 and 4 all rank Lab 6
  first.
- Small: the proof headline orphans "in." (`o2p-proof`).

The round-1 list is closed per 5b's BRIEF: "Free from" computed (seen in `m1-today`), Needs-you cards
date-checked, "Your week" rebuilt questions-first, the board arriving with the first proof, and light
mode at parity. The critic checked `m1-today` in both themes; the other four fixes were not re-checked.

### Candidly, about the others

- **4 (66, up 5)** moved the most. The ranked board (`H`) fixes round 1's padlock grid, proof no
  longer truncates, and discovered sources say where they came from. It still repeats the same proof
  three times on one screen, cuts off "Your week" at the fold in `S-WEEK`, truncates the reveal's
  chips, and ends in a daily page with an unlabelled rail of dots and glyphs. Its reveal carries
  nothing of its thesis.
- **2 (66, up 3)** remains the fallback. Today is now about two scrolls, titles no longer truncate,
  and the morning gate is gone. But the reveal is still mid-animation after 4 seconds (greyed grid,
  weekend missing), the VHL warning sits apart from the VHL row it affects, diamonds still stand for
  due items, and the zyBooks step shows a password already filled in.
- **1b (64, up 6)** cut its reading load, and S-proof is now the best single proof screen in the set.
  Onboarding is still a short form floating in a mostly empty page, progress is shown three ways at
  once, and the daily margin is link soup with three different hour figures.
- **3 (61, up 1)** dropped its false 71.5h total and stopped spoiling the reveal, but every heading is
  boxed in red by focus styling (it reads as an error), the stepper overlaps its own label on
  `o10-your-week`, the daily page shows two numbers for today's work 200 px apart, and the same
  leftover-hours arithmetic survives at day scale.

## What to test with a real student first

Run one session with one student before building further. Use the prototypes and the invented Maya
data only; never put the student's own school, classes, grades or accounts into a prototype. Think
aloud, 1280x800, about 30 minutes.

1. **The school step through proof, 5b against 2.** Walk them from welcome to payment in both
   (counterbalance the order). Afterwards ask "what did Knowlu just find?" and "what will happen to
   the 4 you already turned in?" Does the growing board make the wait feel worth it, or pull their
   eyes off the ask? This decides spine vs fallback.
2. **Willingness to pay at the proof.** At 5b's payment step (`o3-payment`), ask whether $9.99 a
   month feels earned by what they've seen, and what they would need to see first.
3. **The 2-second test on the daily page.** Show each main page for 2 seconds (5b `m1-today`, 2 `T`,
   1b `T`, 4 `M-TODAY`) and ask "what do you do next, and are you okay this week?" Note which wording
   and which placement gets both answers.
4. **The 11 pm question.** Tell them it is 11 pm, show 5b `m1-today`, and ask what they would do
   first. If they pick the two items due at 11:59 pm over Lab 6, the ranking needs an evening rule.
5. **"Your week" load.** Time the rebuilt `o9-your-week` and watch for a student pressing "These are
   right" without reading.
6. **Trust lines.** Ask them to say back where their school password goes (5b `o2n-before`, 4
   `P-SCH-NEED`). If they can't, the privacy line isn't working, whatever it looks like.

Also needed, but not a student question: whether a real school sign-in returns per-item due dates
before payment (spike S-F; 5b has a totals-only variant, `o2p-totals`). The board's counts during
onboarding depend on it.
