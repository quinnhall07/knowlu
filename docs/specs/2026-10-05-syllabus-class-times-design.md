# Reading the syllabus: design (revision 2; amended 2026-10-18; signed by Quinn, 2026-10-19)

**Status: Signed by Quinn, 2026-10-19 (revision 2, this amendment included).** The signature covers
Q4-Q28 as recommended in §11 (§17 Q43), Q19's restated stage (a wave-1 lane, not a friends-release
exit condition), and the rulings of the same day on Q15, Q49 and Q50 (each marked where it is
asked). Q48 stays open and conditional: it is asked only if T4a finds a #19 test pinning a tier
number. L5's plan can start after #19 merges.
**Amendment 2026-10-18 (signed by Quinn, 2026-10-19).** Three inputs, each a ruling or a recorded fact
of 2026-10-18 (the session-5 ledger, `.superpowers/sdd/2026-10-18-session-5/progress.md`):
- **PR #19 merges before this lane** (Quinn). The registrar lands the seam, including the per-source
  table in `commitments.rs` (Quinn's ruling (4) on the close-out's §8); this lane adds the `syllabus:`
  row to it. The close-out plan's edits E1-E10
  (`docs/plans/2026-10-18-p3-registrar-close-out-plan.md`, §6 "The L5 delta") are applied below, worded
  for the ruled table and the R0 spike's facts. Quinn's ruling (9) folds them into this amendment.
- **The S2 spike's facts** (Simple Syllabus at UA, shapes and counts only), in §3 and §15.
- **Quinn's Q10 ruling:** build the Simple Syllabus read (Offer A), and widen it beyond class times:
  "It has grade information, late work policies, test and quiz dates, assignments, etc." §15 designs
  the read, §16 the widened read, and §17 holds every new choice that is Quinn's.

| Where | What changed in this amendment |
|---|---|
| Header, §2 | E1, E2; late-work policy leaves the non-goals (Q10) |
| §3 | Offer A's row and its "what it really yields" bullet carry S2's facts |
| §4.4 | E3 (the watched prefix as a table row), E4 (precedence on #19's tiers), E5 (no instances) |
| §5, §6, §7, §8 | rows for the Simple Syllabus read and the policies; the privacy lines L4's page carries |
| §9 | E6, replaced in full |
| §10, §11 | E7 (Q6); Q10, Q13, Q19, Q25 and Q28 restated by the 2026-10-18 rulings |
| §12, §13, §14 | E8, E9, E10; new risks, tasks and unverified items |
| §15, §16, §17 | new: the in-page read, the widened read, Decisions for Quinn (2026-10-18) |
| §4.4, §9, §13 *(review round 1)* | E3 corrected: the branch watches `registrar:`, and the registrar's card comes from its kept entries, not `rank`'s change path; T4t pins only the `syllabus:` row |
| §4.1, §5, §15.4, §15.5 *(review round 1)* | the extraction file records its `source`; held renderings for a two-way match and for a kept upload (Q47) |
| §4.6, §5, §6, §6.1, §16.3 *(review round 1)* | what each synced card carries, stated field by field; no policy quote on a card; the stale bump text removed (Q46) |
| §4.8, §13, §17 *(review round 1)* | the `syllabi` read model's shape and its task (T17); the console order with L1 R11; the L6 hand-offs; Q42 rewritten as a merge gate |
| §4.4, §9, §11 Q6, §12, §13, §17 *(convergence round 1)* | the tier renumbering stated as T4a's one change outside its row; `precedence` moved from T4b to T4a (`integrator`); T4t pins orders, never numbers; Q48 |
| §4.6, §12, §13, §16.5 *(convergence round 1)* | the syllabus-card withdrawal: the reason in `syllabus_cards.rs`, the move in `approvals::withdraw_syllabus` and the finishing arm (T4e); "superseded is not a decision"; the due amend card's key; T4dt's cases |
| §4.4, §12, §13, §17 *(convergence round 2)* | a syllabus series' `first` and `until`; what the ended arm stamps (`last_instance` the last meeting day before the run, `until` the series' or that day), so #19's ended path files the end or swap card unedited; T4t pins the dates; Q49 |
| §3, §4.4, §5, §8, §12, §13, §15.5, §17 *(convergence round 3)* | the ended arm and the fresh pass run only on a **usable meetings read** (status `read`, the meetings part by rules or at or above the floor, not clipped, at least one meeting); otherwise the deletion arm drops the series with no `ended` entry (Q50 (a)); the false "re-derivation is deterministic" reason replaced; §5's per-part state gains `empty`; T4t's replacement cases; Q50 |
| §4.4, §12, §13, §17 *(convergence round 4)* | Q50 (a)'s second cost stated: the drop leaves no series to age out, so a meeting a later usable meetings read leaves out (a new copy's, or a pending copy's once read) is never ended; the key paragraph's removed-lab sentence and the usable-read paragraph say so; T4t's later-copy case pins that outcome instead of an end; Q50's recommendation marked QUINN TO CONFIRM |

**Review round 1 of the amendment (2026-10-18):** ten findings (seven important, three minor), each
fixed in place; the rows marked *(review round 1)* above. Two needed Quinn and are §17's Q42
(rewritten) and Q46; a third, the held rendering, is Q47.
**Convergence round 1 (2026-10-18):** two important findings, both fixed in place; the rows marked
*(convergence round 1)*. One case, a #19 test pinning a tier's number, needs Quinn only if it
arises, and is Q48.
**Convergence round 2 (2026-10-18):** one important finding (a dropped syllabus key reached #19's
ended path with no last date, so no end or swap card could file), fixed in place; the rows marked
*(convergence round 2)*. One case, a series removed before it began, is Q49.
**Convergence round 3 (2026-10-18):** one important finding, fixed in place; the rows marked
*(convergence round 3)*. The ended arm counted a key's absence from any current extraction as an
end. So a replacement copy whose meetings part was empty or below the floor (S2's usual Simple
Syllabus shape) would have filed an end card for every class of a course still meeting. The arm now
needs a usable meetings read. What happens to the old series without one is Q50.
**Convergence round 4 (2026-10-18):** one important finding, fixed in place; the rows marked
*(convergence round 4)*. Under Q50 (a), round 3's drop takes the old series out of the file, so when
a later usable read (a new copy's, or a pending copy's once `judge` reads it) leaves a meeting out,
the ended arm has nothing to age out and nothing to stamp. T4t pinned an end that no implementation
of §4.4 could file, and Q50 (a)'s cost did not name the loss. T4t now pins what (a) does, and Q50
states the cost for Quinn to weigh against (b) and (c); the recommendation is marked QUINN TO
CONFIRM.
**Date:** 2026-10-05; revision 2 of 2026-10-06. **Asked by:** Quinn, during the email live proof.
**Revision 1 (2026-10-05):** addressed the spec review of the same day (verdict FIX, 14 findings: one
critical, nine important, three minor). Every true choice the review found became a numbered
decision. The draft had also misread `precedence` (`engine/src/commitments.rs:1416`): it puts a
`google:` calendar *first*, not last. §4.4 is corrected.
**Revision 2 (2026-10-06):** carries Quinn's two rulings of that day (§0). The scope widens from class
meetings to everything a syllabus says that "what's next?" uses, and the source order becomes: the
LMS first, then an offer of Simple Syllabus or the student's own file. §11 is renumbered: Q1-Q3 are
ruled, Q4-Q18 are revision 1's open questions under new numbers, and Q19-Q28 are new.
**Governs:** how a course's syllabus is found, read and turned into proposals: class meetings, exam and
quiz dates, assignment due dates, grade weights and scale, office hours, and (amendment 2026-10-18)
the late-work, makeup and attendance policies (§16).
**Authority:** `docs/specs/2026-09-09-knowlu-cloud-design.md` (ruling 5 at :577-580 is the one this
changes); `VISION.md` (:42-48, :88-90, :152-154, :182, :227, :236); the commitment model
(`docs/specs/2026-09-23-commitment-model-design.md`) and its phase 3
(`docs/specs/2026-09-26-commitment-model-phase3-design.md`, merged to `main` with PR #19 before this
lane, by Quinn's ruling of 2026-10-18); the events spec (`docs/specs/2026-09-29-events-design.md`, D1, D7, D10); the grades
spec (`docs/specs/2026-09-29-grades-design.md`, G2, G3); the email forwarding spec
(`docs/specs/2026-09-30-email-forwarding-design.md`, §4's resolve rule at :440-444 and the first-day
hold at :453-456) and its ruling PQ13 (b), the due-aware duplicate rule.

Quinn's words (2026-10-05): "is it possible we could skim the class for a syllabus file and use that?
And if it's gated behind Simple Syllabus, we should add a connection for that, too. We could also
allow for manual uploading of syllabi. I would rather them put in their syllabi first before having
to enter class times on their own."

## 0. Quinn's rulings of 2026-10-06, and the amendment they need

**Ruling 1, scope.** Bring forward from Beyond all of: class meeting times (classes, labs), exam and
quiz dates, assignment due dates, grading weights and scale, and office hours. A syllabus yields:
- **meetings** as commitments, through the existing commitment proposal path, confirmed by the
  student (§4.4-§4.5);
- **dated assessments and assignment dues** as proposed tasks and exams that the student approves:
  approvals, `propose`, judge once, the 15-a-day proposal cap, and a duplicate check against
  LMS-feed items with the same title, course and due day, on the email lane's PQ13 (b) precedent
  (§4.6);
- **grade weights and scale** stored on the course note (`courses/` frontmatter, through `write`),
  feeding importance and the grades lane where it has a slot (§4.7);
- **office hours** shown on the course, not proposed (`VISION.md:48`) (§4.8).

**Ruling 2, order.** Connect the LMS first and look for the syllabus there. Where none is found, offer
the student either connecting Simple Syllabus or manual entry: uploading or pasting the syllabus, or,
as the last resort, typing class times (§3).

**The amendment (T0 applies it after Quinn signs; drafted here, not yet in either file).**
- Cloud ruling 5 (`cloud-design.md:577-580`) becomes: "5. **Syllabi are read for the student**, at
  onboarding and each term: class meetings, exam and quiz dates, assignment due dates, grade weights
  and scale, and office hours. Knowlu looks for each course's syllabus in the LMS first; where it finds
  none, it offers the school's syllabus system, where one is curated, or the student's own file or
  pasted text. Meetings are proposed as commitments, dates as tasks and exams, weights and scale as a
  change to the course note; office hours are shown, not proposed; the student confirms everything
  else. This amends the commitment model's Q2 ('login-only… typing is a last resort') to 'sign-ins and
  syllabi'; typing class times stays the last resort. *(amended <signing date>, syllabus spec rulings 1
  and 2, signed by Quinn)*"
- `VISION.md:88-90` becomes: "**Syllabi**, found in the LMS or the school's syllabus system, or added
  by the student, at onboarding and whenever a term starts: class meetings, exam and quiz dates,
  assignment due dates, grade weights and the grading scale, and office hours. Uploading a file is not
  typing; everything Knowlu reads from one is shown for the student to confirm, and office hours are
  shown, not proposed."
- `VISION.md:152-153` becomes: "**Onboarding creates the vault** under `%USERPROFILE%\Knowlu\` and asks
  for sign-ins; it then looks for each course's syllabus in the LMS and, where it finds none, offers the
  school's syllabus system or the student's own file. It is judged by one thing: ..."
- `VISION.md:227` (Beyond) loses "syllabus upload with grade weights,"; GPA and exam prep stay there.
- `VISION.md:236` closes: "How a syllabus is read | Decided (syllabus spec, <signing date>): fetched on
  the device, screened by rule, extracted by one pinned model to a strict schema, every result
  confirmed by the student".
- `HANDOFF.md`'s Beyond paragraph (§3) and the stage this lane joins (Q19) follow in the same commit.
- *(amendment 2026-10-18)* `VISION.md` has moved since this was drafted: the lines quoted above as
  :152-153, :227 and :236 now read at about :154-156, :229 and :238. T0 applies each edit **by its
  quoted text, never by line number**. Ruling 5's and `VISION.md:88-90`'s new text gain, after "office
  hours": "the late-work, makeup and attendance policies (shown, and the late-work rule proposed as a
  change to the course note)"; the registrar bullet ("The registrar, where a school offers it…") is
  untouched. Both additions wait on §17's Q34 and Q35.

## 1. The problem, and the exact moment it shows

A scratch profile at The University of Alabama finished the wizard and opened the first-day confirm
screen (`#week-setup`, `app/static/index.html:92-99`). `checkWeekSetup` opens it
(`app/static/console.js:820-825`) while `your_week.setup` is true. Under "Your classes", every
course read **"no class times found; Knowlu will ask this week"** (`console.js:843`, asserted by
`app/tests/static_assets.rs:1754`). With no repeating calendar event at all, the status line reads
"Knowlu found nothing repeating on your calendar. Set your day below." (`console.js:876-877`).

Why: class times reach Knowlu from exactly one place, repeating events on a connected calendar.
Google and ICS events are normalised in `engine/src/cli.rs:779-812`, merged by
`commitments::refresh_series` (`commitments.rs:2079`), then classified and proposed. A course
the LMS lists (`app/src/lms_link.rs:481`) with no class proposal and no confirmed class note is
"uncovered" (`commitments::uncovered_courses`, `commitments.rs:890`). Today an uncovered course has
exactly one way forward. From vault day `ASK_FROM_DAY` = 3 (`commitments.rs:3066`), a
`commitment-ask` card asks the student to **type** the days and times (`emit_asks`, :3120, at most
`ASKS_PER_DAY` = 2 a day, :3063). Approving settles a hard class note; rejecting writes the
`card:<slug>` marker. A student who has never put classes on a calendar types every course. That is
most first-years, and every student at a school with no class-time feed.

The same gap runs wider than class times. The LMS feed carries only what an instructor has already
posted, often two or three weeks ahead. The syllabus usually names every exam date of the term on day
one, and the course's weights, which `judge` needs to ground importance (`judge.rs:409`, :591) and
which today sit as an empty `## Grade weights` seed (`app/src/scaffold.rs:785`). A midterm five weeks
out is "a test a week out they should start studying for" (`VISION.md:12`) long before the feed shows
it, if it ever does.

That breaks things the product has already said:
- "Uploading a file is not typing; everything Knowlu reads from one is shown for the student to
  confirm" (`VISION.md:88-90`), and cloud ruling 5's "typing stays the last resort" (:579).
- The first-session test: "a new student reaches 'it already knows what's next'" (`VISION.md:153-154`,
  :192). A day with no classes on it, or a term with no exams in it, is not "what's next".
- "Nothing missed that Knowlu had the information to catch" (`VISION.md:189`).

A syllabus nearly always names the meeting pattern, the graded work and the weights. At UA, every
section must publish one in Simple Syllabus by the first class day (cit.ua.edu; bama.atlassian.net CIT
pages, read 2026-10-05). Nothing syllabus-shaped exists in code today. "Simple Syllabus" has no hit in
the repo, and the only `syllabus` string in `app/src` is the `## Grade weights` seed.

## 2. Goals and non-goals

**Goals**
- G1. For every course, Knowlu looks for the syllabus in the LMS first. Where it finds none, it offers
  Simple Syllabus (where the school's row names a host) or the student's own file or pasted text, side
  by side. Typing class times comes last (ruling 2).
- G2. Whatever a syllabus yields arrives as **ordinary proposals on existing paths**: class and lab
  rows on the first-day confirm screen, then `commitment-check` cards; task and exam cards in the deck;
  an `amend` card for the course's weights and scale. Nothing is written as the student without a tap,
  and nothing is auto-confirmed. Office hours are the one exception, shown and never proposed.
- G3. Fetch on device, think in the cloud. The file is fetched and its text extracted on the device.
  Only a screened, bounded excerpt goes to a new pinned judgment kind, and the result enters the
  engine as data. `rank` never calls a model.
- G4. Every source says, per course, what it found or why it found nothing (§8). No row is ever left
  blank, and no wait has an open end.
- G5. School-agnostic. The only school-specific facts are curated data rows (`scaffold.rs:26-62`).
- G6. The LMS feed stays the authority on what it carries. A syllabus date never overrides a feed
  item, never duplicates one, and yields to one that arrives later (§4.6).

**Non-goals (this spec)**
- GPA, the uncertainty band, exam prep (study sessions placed before a test) and grade-based priority
  weighting. They stay in Beyond (`VISION.md:67-70`, :133-136). A syllabus here feeds the slots that
  exist: importance in `judge`, and the letter scale in the grades read model (Q26).
- A weighted ring (grades G2's "syllabus weights replace the second basis", `grades-design.md:18`).
  It needs Blackboard categories mapped to syllabus categories, which is its own design (Q26).
- Readings, lecture topics and class cancellations ("No class: Fall Break"). Q21 holds the line on
  what the schedule yields. *(amendment 2026-10-18: late policy left this list by Quinn's Q10 ruling;
  §16 reads it.)*
- Textbooks and other required materials (Simple Syllabus's `material` component), credit hours,
  extra credit and per-item points (§16.6, Q36).
- OCR of scanned PDFs. A file with no text layer gets a named outcome (§8); OCR is not a feature here.
- Syllabi by email (forwarding one to the student's Knowlu address). The mail pipeline keeps what it
  found, never the message (`VISION.md:182`). An attachment path would be a later, separate spec.
- A new local card kind. `commitments::LOCAL_CARD_KINDS` (`commitments.rs:27`) stays
  `["commitment-ask", "commitment-check"]`. `sync.rs` and the tripwires are not touched. The one new
  card kind, `syllabus-exam` (Q20), is synced like every other deck card.
- The registrar. It reaches `main` with PR #19 before this lane is built; this spec changes nothing in
  it (§9). *(E2, amendment 2026-10-18)*
- A cross-student syllabus cache (market note §7.6). It would need its own ruling.

## 3. The source order, and what each step yields

Ruling 2 sets the order. It runs for **every** course on the student's list, not only uncovered ones:
a course whose classes are already on a calendar still has exam dates and weights to read. Each step
records its outcome (§8), whether or not it yields a syllabus.

| Step | Source | Reach | Yields | Needs |
|---|---|---|---|---|
| 1 | **The LMS's own syllabus** | Canvas: the course's `syllabus_body` (one GET with `include[]=syllabus_body`). Blackboard: the course's content tree, depth-limited. It looks for items whose title or file name matches `syllabus` (case-insensitive), with handlers `resource/x-bb-file` and `resource/x-bb-document`, and downloads PDF, DOCX or HTML. **Runs only inside the wizard's LMS sign-in** (§3.1) | A syllabus text, or a **gated link**: an LTI or external link whose title or host says "syllabus", recorded as an outcome; *(amended 2026-10-18, Q44)* it never supplies a host, which only a curated row does | The wizard's live LMS window; spike S1 |
| Offer A | **Connect Simple Syllabus** | Shown only where the school's curated row names a host. The host in its own sign-in window, cookies never kept, the window wiped after the read. *(amended 2026-10-18)* S2 found that a plain cookie GET of the document call is refused (403), so the read happens **in the page**, by a compiled-in script the app evaluates by name, the way the mail window already reads its pages (§15). Never CDP, never OS input. The host comes from a curated row field, never from code; a gated link step 1 finds is only evidence for curating a row (§15.1) | Every section the student holds on the host's "my courses" page, as the host's own document JSON: structured term dates and section attributes, and the syllabus as HTML components (§15.3) | S2 (done), S3 (the in-page read), a first-pass terms read (Q39), a curated field |
| Offer B | **Add the syllabus** | A file picker (PDF, DOCX, HTML or TXT), or a box to paste the text, per course | A syllabus text | Nothing external |
| Offer C | **Type the class times** (last resort, uncovered courses only) | Inline on the first-day confirm screen (Q27); after day 1, the existing `commitment-ask` card (`commitments.rs:3060-3167`) | A confirmed class note | Unchanged after day 1 |

**The offer is one choice, not a ladder.** Ruling 2 says "either ... or". Offers A and B show side by
side on a course whose step 1 found nothing; the student picks. Offer C shows below them, smaller,
only on a course with no class times. Offer A shows only where it can work: no host, no button. A
student who takes neither offer is never blocked: the course keeps its LMS-feed items, and an
uncovered course is asked for its times on today's schedule (§8, Q16).

**What each step really yields.**
- Step 1, Canvas: one call in the common layout. But `syllabus_body` is often a stub ("see Simple
  Syllabus") where the school uses an external tool, and the Files tab is often turned off for
  students. Blackboard: only where the instructor posted a file and the student's cookie session can
  read `GET /learn/api/public/v1/courses/{id}/contents`. That is unverified: `grades.rs:550-571`
  proves a cookie session reaches `users/me`, the course list and the gradebook, but not contents. At
  UA, where Simple Syllabus is mandated, most sections likely resolve to a gated link. No published
  figure says what fraction.
- Offer A *(S2, 2026-10-18, one student's eight Fall sections, counts and shapes only)*: per-section
  syllabi, all eight listed on the host's "my courses" page. The institution's structured section
  attributes carry the term, the CRN, the schedule type, the section title, credit hours, campus,
  instructional method and part of term, and **no meeting time, day or room**. The term's start and
  end dates are structured (`YYYY-MM-DD`, with a timezone). The syllabus itself is HTML in
  `components[]`, typed `content` (158 across the eight), `instructor` (8) and `material` (8). Clock
  times appear in free text in 2 of 8 documents, one of them inside the instructor component (likely
  office hours). Dated, schedule-like content appears in 2 of 8 (37 and 17 dates). Percent signs are
  inflated by inline styles, so a raw percent count is no weights signal (§15.3). No uploaded files;
  one content component links a PDF. **So at UA, Offer A adds little for class times** (the registrar
  has them, §9) and its value is the dated items, the grading and the policies (§16).
  *(convergence round 3)* It also means a Simple Syllabus copy usually has no usable meetings read
  (§4.4). When one replaces an LMS copy (§15.5), the course's syllabus series are dropped with
  nothing ended, so no end card is filed for a class that is still meeting (Q50).
- Offer B: works at every school, today, with no session, no terms question and no spike.
- Every source: a syllabus often lists several sections, puts times in free text, or dates its schedule
  by week number. The extractor returns every meeting pattern with its section label and every dated
  item as written. The engine matches the student's section (§4.4), resolves week numbers only from a
  date the syllabus itself gives (§4.6), and never guesses.

### 3.1 Where each step runs, and where the order shows

**Step 1 runs in the wizard, and only there.** The LMS sign-in window keeps no cookies, session or
profile (`lms_link.rs:1-6`). Its commands (`open_lms_window`, `capture_courses`, ...) are only in the
onboarding `generate_handler!` list (`app/src/main.rs:107`), not in the console list (:179) that
`#week-setup` runs under. So the skim runs where the session exists: straight after
`capture_courses` in the wizard, in the same sitting and the same window (`console.js:3180`). It is a
new onboarding command, `lms_link::capture_syllabi(unitid, courses)`, added to list :107 only. Each
course gets its own outcome, and a failure says nothing about the course list that already worked.
The console gets no LMS command. Later terms are covered by Offer B, and later by the kept grades
profile if Q11 (b) is ever ruled.

**Offer A runs from the console**, behind its own Connect button, in its own window
(`syllabus_link.rs`, T10). Its commands go in the console list (:179). It never runs in the
background: Simple Syllabus may sit behind SSO and MFA, so it needs the student present.
*(amendment 2026-10-18)* While lane L6 owns `app/src/main.rs`, the handler lines are handed to L6's
owner or added after L6 merges (session-5 conflict map). The window, the script and the read are
§15's.

**Offer B runs from the console.** The commands `syllabus_add` and `syllabus_status` go in the
console list (:179). The wizard offers no upload: the confirm screen opens straight after Finish and
carries the offer there, so list :107 gains only `capture_syllabi`.

**On the first-day confirm screen**, a new section, "Your syllabi", lists every course with its step-1
outcome (§8). A course whose syllabus was found reads "Found in Blackboard" and its read state. A
course with none shows the offer: **Connect Simple Syllabus** (where available) and **Add syllabus**
(file or paste). Under "Your classes", each uncovered row (`console.js:842-844`) stops saying
"Knowlu will ask this week"; it points at its course's offer and carries Offer C's "Type the times"
link. After day 1, the `commitment-ask` card for a course shows the offer above the typed fields, so
the card's first action is a syllabus, not typing. The Schedule view carries the same offer for later
terms.

## 4. Device and cloud: the pipeline

### 4.1 Fetch (device), and where fetched bytes wait
Step 1 and Offer A run in the app, like every portal fetch. Step 1 uses the school's own sign-in
window, hands its cookies to `ureq` GETs, navigates nothing and keeps nothing (`lms_link.rs:1-6`,
:395-433, :570-596). *(amended 2026-10-18)* Offer A cannot: its host refuses the plain GET, so it
reads in the page by a compiled-in script and keeps nothing either (§15.2). Bounds: at most 3 candidate files per course, each at most 10 MB, with a per-course
timeout that yields "not tried" (§8). Whatever runs, the student is never kept waiting on Finish.

**Fetched bytes never go to the vault from the app.** In the wizard the vault does not exist yet, and
`app/src/commands.rs` writes no vault file in any case. Step 1 writes each file to a staging folder
under `state::app_data_root()`: `pending/syllabi/<random>/<lms-course-id>.<ext>`. Beside it goes a
small outcome list. At Finish, `onboarding::create_vault_in` hands each course that is still on the
student's list to the engine as `syllabus-add` (below), then deletes the folder. Courses the student
removed are deleted unread. A leftover folder more than an hour old is swept at the next launch, the
way `sweep_stale_sessions` sweeps session folders (`lms_link.rs:229`). Offer A stages the same way
under the open profile's own folder (`profiles::profile_dir`).

**Offer B** uses the native dialog `onboarding::pick_file` (`app/src/onboarding.rs:272`, already in
both handler lists, `main.rs:107` and :179). It gains an optional list of extensions, and its existing
callers stay unchanged. The picked path goes straight to `syllabus-add`, with no staging step. Pasted
text is written by the app to the profile's staging folder as a `.txt` and handed over the same way,
then deleted.

**`syllabus-add`, a new engine command, ungated and offline.** It never calls the network. The two
forms are:
`syllabus-add --vault <v> --course <slug> --file <path> --source lms|simple-syllabus|upload|paste`
and `syllabus-add --vault <v> --course <slug> --outcome <code> --source <s>`. The second form records
a step that found nothing (§8's codes, a closed list). *(amendment 2026-10-18)* A third form,
`syllabus-add --vault <v> --file <path> --source simple-syllabus`, takes one host document JSON with
no `--course`: the engine matches it to a course itself (§15.4) and renders the copy it keeps
(§15.3, Q31). *(review round 1, Q47)* A fourth form,
`syllabus-add --vault <v> --held <id> --course <slug>`, settles a **held rendering**: a Simple
Syllabus document the third form could not place on its own, either because it matched two courses
(§15.4) or because the course already has the student's own upload or paste (§15.5). The held form
needs no staged file, so the student's pick or "Use Simple Syllabus's instead" works after the
Connect window is gone.

**Held renderings.** The third form writes what it cannot place to
`<vault>/state/syllabi/held/<id>.html` (the same rendering §15.3 keeps) and `<id>.json` (`version`,
`reason`: `two-matches` or `kept-own`, the candidate slugs, `added`). `<id>` is the first 12 hex of
the SHA-256 of the rendering, so a second Connect of the same document reuses it. Nothing is read
from a held file: no screen, no `judge`, no card. The held form checks that `--course` is one of the
candidates (exit 2, nothing written, otherwise), moves the rendering to `syllabi/<slug>.html` and
continues as the file form. Every run of `syllabus-add` deletes held files more than 30 days old.
`state/syllabi/` never syncs (§5), so a held rendering stays on this desktop.

**One copy per course, and who may replace it (Q33).** A course has at most one file in `syllabi/`,
whatever its extension; a replacement deletes the earlier copy, `.pdf` or `.html` alike. The
extraction file records the copy's `source` (§5). An `upload` or `paste` replaces any copy (the
student asked). `simple-syllabus` replaces a copy whose `source` is `lms`, or a course with none, and
otherwise is held as `kept-own`. `lms` comes only from the wizard (§3.1), before any other copy can
exist.

The file form does the following:
1. Copies the file to `<vault>/syllabi/<slug>.<ext>` (Q8), replacing an earlier copy for that course
   by the rule above.
2. Extracts and screens its text (§4.2).
3. Settles by rules each part it can (§4.2).
4. Marks the course `pending` with today's date in `state/syllabi/<slug>.json` (§5) if any part is
   left for the model, and `read` otherwise.

It exits 0 on every named outcome and 2 on bad input (an unknown course, an unreadable path, and
*(review round 1)* an unknown held id or a course that is not one of its candidates), having
written nothing. Like `commitments` without `--confirm`, it writes no note and no journal record. It
joins `docs/reference/engine-commands.md` (T13).

### 4.2 Text and screen (device, engine)
A new engine module, `engine/src/syllabus.rs`, turns the file into plain text:
- PDF: the text layer (one new crate, Q7).
- DOCX: `word/document.xml`, through the `zip` crate the engine already depends on
  (`engine/Cargo.toml:54`). Each table row becomes one line, its cells joined by `" | "`, so a
  schedule table survives as rows.
- HTML: tags stripped; `<tr>` rows joined the same way.
- TXT and pasted text: as is.

Then comes a **deterministic screen**, per `VISION.md:182` ("the cheapest deterministic screen
discards what cannot answer it before any model reads it"). It builds **four parts**, each with its own
bound. Together they are the excerpt, clipped at **`MAX_SYLLABUS_CHARS` = 12,000 characters**
*(amendment 2026-10-18: a fifth part, Policies, at 1,500, and a total of 13,500 if Q38 is ruled as
recommended; §16.2)*:

| Part | Keeps | Bound |
|---|---|---|
| Identity | the line or lines carrying the course code, the section label and the term | 400 |
| Meetings | every line carrying a day token (`M`, `TR`, `MWF`, `Monday`, `Tues`...) **and** a clock time (`9:30`, `2 pm`, `14:00`), with one line of context either side; any line naming "meets", "class time", "lecture", "lab", "recitation" or "location" next to a time; any line naming "office hours", with every e-mail address, phone number and URL in it replaced by `[removed]` | 1,600 |
| Schedule | every line carrying a calendar date (`Oct 14`, `10/14`, `2026-10-14`, `Tuesday, October 14`) or a week token (`Week 7`, `Wk 7`), each clipped to 240 characters, in file order; the header line of each run of such lines (the nearest line above it with no date) | 7,000 |
| Grading | lines carrying a percent sign or "points"/"pts" beside a word; every line of a block whose heading matches `grad(e|ing)`, `evaluation`, `assessment`, `weight` or `breakdown`, up to the next heading; letter-scale lines (a letter grade beside a number or a range) | 3,000 |

**Dropped, always:** the instructor block. Any line carrying an e-mail address or a phone number is
dropped whole, unless it is an office-hours line (sent with those removed). So is any line opening with
"instructor", "professor", "TA", "teaching assistant", "e-mail", "phone" or "office:". These are a third
party's personal data, and nothing in the schema needs them. Everything a part does not keep stays on
the device: descriptions, policies, accommodations text, learning outcomes and the student's own marks.
*(amendment 2026-10-18: except the late-work, makeup and attendance paragraphs the Policies part
keeps, §16.2.)*

**When a part overflows its bound** it is clipped at a line boundary, and the extraction records it.
A clipped schedule keeps its earliest lines, and the course reads "Read the schedule up to Nov 20; the
rest is past what Knowlu reads at once" (§8). Unused bound in one part is not lent to another, so the
excerpt's shape, and the privacy page's sentence, stay fixed.

**Rules settle what they can, per part**, the way the judge pipeline runs rules before any model
(`judge_pipeline.ts:1-6`):
- Meetings: a single unambiguous pattern line, such as "MWF 10:00-10:50 AM, Example Hall 101" (an
  invented room; examples, plans and tests use invented rooms only), for one section.
- Grading: a block whose every line is `<category> <number>%`, summing to 100 (±1); a scale block whose
  every line is a letter and a minimum.
- Identity is never sent alone, and the Schedule part is always the model's: its titles and kinds are
  free text.

A part rules settled is not sent. A part with nothing kept is not sent. **No part left: no call.** A
syllabus with no day-and-time pair, no dated line and no grading line reads "This syllabus names no
class times, dates or weights" (§8).

### 4.3 Extraction (cloud, a new pinned kind)
The residue goes to a new judgment kind, `syllabus`, through the existing transport
(`engine/src/cloudmodel.rs`, `CloudClient`, beside `/judge-task`, `/judge-event` and `/judge-email`,
:514 and :602). As for `task`, `event` and `email` (`judge_validate.ts:9`), the prompt, the strict
output schema and the pinned model live server-side. The request carries the parts that are left and
the vault-local date (so a schedule's year can be read). The draft schema:

```
{ "version": 1, "term": string|null, "week1_start": "YYYY-MM-DD"|null,
  "sections": [ { "label": "001"|null,
      "meets": [ { "kind": "class|lab", "days": ["mon", ...], "start": "HH:MM", "end": "HH:MM",
                   "where": string|null, "first": "YYYY-MM-DD"|null, "until": "YYYY-MM-DD"|null } ] } ],
  "office_hours": [ { "days": [...], "start": "HH:MM", "end": "HH:MM", "where": string|null } ],
  "items": [ { "kind": "exam|quiz|assignment", "title": string, "section": string|null,
               "date": "YYYY-MM-DD"|null, "week": int|null, "day": "mon"|...|null,
               "start": "HH:MM"|null, "end": "HH:MM"|null, "in_class": bool } ],
  "weights": [ { "category": string, "percent": number } ],
  "scale": [ { "letter": "A|A-|B+|...|F", "min": number } ],
  "confidence": { "meets": 0..1, "items": 0..1, "grading": 0..1 } }
```

- Meeting `kind` is `class` or `lab` only: a recitation or discussion section is a `lab`, as the
  classifier's designators treat them (`commitments.rs:569-571`, :1206-1207).
- Item `kind` is `exam` (exam, test, midterm, final), `quiz`, or `assignment` (homework, problem set,
  paper, project, lab report, presentation). Readings and topics are not items (Q21).
- An item gives either a `date` or a `week` and `day` as written, never a date the model computed from
  a week; the engine resolves weeks (§4.6). `in_class` is true only where the syllabus says the item
  happens in class.
- **Confidence is per part.** A part below the floor is "found a syllabus, could not read <part> from
  it" (§8), never a guess, and does not sink the other parts.

The request's `item.id` is the **course note's opaque `id:`** (`course_...`, `ids.rs:27`). It is never
the slug and never the file's hash, which would be the same for every student holding that syllabus.
Only the clipped excerpt, the vault-local date and that id travel.

**Where it runs: inside `judge`, as a scoped pass (Q5).** `judge` already always exits 0, is already
gated by `entitle.rs`, and is already where judgment lives. A plain `judge` run in the slot runs the
syllabus pass after the enrichment batch and before the events pass (`enrich.rs:412-439`), at most 8
courses a run, on the same budget. **A new flag, `judge --only syllabus`,** runs the probe and the
syllabus pass and nothing else: no task enrichment, no events, no Gmail, no mail, no rules, no labels.
So an upload never spends the student's task, event or email caps, and never makes them wait on
unrelated judging. This changes the CLI contract (`main.rs:155`), and
`docs/reference/engine-commands.md:61-65` gains the flag (T13). No new gated command means no
`entitle.rs` edit. **The pass writes only `state/syllabi/<slug>.json`**: no note, no card. Cards are
filed by `rank` from that file (§4.6, §4.7), so filing is deterministic, never gated, and runs every
slot as items come into range.

**Running it next to the scheduler (Q17).** After an add, or when the confirm screen opens with a
`pending` course, the app runs `judge --only syllabus`, then `commitments` and `rank`, as child
processes. The chain takes the scheduler's `running` flag (`app/src/scheduler.rs:58`) the way a slot
does. If a slot already holds it, the app starts nothing and the row reads "Reading with this run".
The slot's own `judge` step picks the course up, and if that step has already passed, the app runs the
scoped chain once the slot releases the flag. The scoped run uses an existing `journal::VIAS` value,
so `VIAS` is not touched.

### 4.4 Meetings into the engine (device, deterministic)
The extraction comes from rules or from the cloud and is stored in `state/syllabi/<slug>.json` (§5).
`commitments` reads its meetings as a **new series source**, beside Google and ICS. It stays read-only:
`commitments` without `--confirm` still writes only `state/calendar-series.json`.

**The key comes from the content, never from position.** There is one `Series`
(`commitments.rs:946`) per course, section and kind:
- `calendar`: `syllabus:<course-slug>`
- `source_uid`: `syllabus:<course-slug>:<section>:<kind>`

`<section>` is the extracted label, normalised: trimmed, upper-cased, with a leading "section" or
"sec" removed. It is `-` when the syllabus has one unlabelled section. `<kind>` is `class` or `lab`.
The series' `meets` holds every meeting of that kind in that section, the way a confirmed note's
`meets` list does. *(convergence round 2)* Its `first` is the earliest `first` those meetings give and
its `until` the latest `until` they give, each null when the syllabus gives none; it carries no
instances (E5).

Why this key and not one carrying the times: the extractor's output order and a re-added file's
section order cannot move it. A corrected time or room keeps the key, so §5.4's **changed** path
compares `meets` under one key and files one change card. A key that carried the meeting signature
would instead turn every correction into an end card for the old key plus a check card for a new one.
A lab that a corrected syllabus adds is a new key (a check card). A lab it removes leaves the
extraction (the ended path, below), but only when the corrected copy's meetings read is usable
*(convergence round 3)*, and only while the series file still holds the lab's series *(convergence
round 4)*. Under Q50 (a), a run that found no usable read has already dropped that series (the
corrected copy itself, while still pending, is one such run), and the removal is then never ended
(below).

**`SERIES_KEY_PREFIXES` gains `"syllabus:"`** (`commitments.rs:3364`, read at :3493). *(E3, amendment
2026-10-18; corrected in review round 1)* What PR #19 leaves, stated as facts:
- **On the branch at `2ad1e3d`**, `SERIES_KEY_PREFIXES` is
  `["gcal-series:", "ics-series:", "registrar:"]` (`commitments.rs:3483` there): `rank`'s §5.4
  change path still watches registrar notes. The close-out plan's §6 sentence that the registrar
  "leaves the card path" was written before Quinn's ruling O1 and is not what the branch says.
- **After #19's close-out** (its spec's "Amendments (2026-10-18, close-out)", items 8, 12 and 16, for
  review; built by task B3), the registrar **applies and lists** a changed `meets`, `where` or
  `until` at the fetch, and `rank`'s `detect_changes` files no change, end or swap card for a
  `registrar:` key. A field the student set by hand is never written; by O1 its change gets the
  **existing `commitment-check` change card**, filed from the registrar's *kept entry* in the series
  file, at the fetch and at every `rank` through `emit_checks`. So a registrar note does get a card
  for a hand-set field, by its own route, not by the §5.4 change path.
- **The per-source table** (Quinn's ruling (4), B3) names, per calendar prefix, the precedence tier,
  the keep-alive rule, whether series carry instances, whether the source holds other sources'
  proposals per kind, and how a change lands: a `commitment-check` card through `detect_changes`, or
  applied and listed (with the kept-entry card for a hand-set field).

`syllabus:` joins as **one row of that table**: tier 1, its two keep-alive arms (below), no instances
(E5), no per-kind hold, and changes that land only as a card through `detect_changes`, the §5.4
change path, under that path's existing hand-set rule (§4.5); nothing is ever applied. **T4a adds
that row, and its one change outside that row is the tier renumbering** *(convergence round 1)*:
inserting `syllabus:` between the registrar and Google moves `google:` from tier 1 to 2 and the
default row from 2 to 3 (E4, below). That is a renumbering, not a reordering: every pair of existing
sources keeps its order. If B3 expresses tiers by row order rather than by a number column, inserting
the row is the whole change and nothing is renumbered. T4a edits **no other column of any other row**:
the registrar's row, its landing and its kept-entry cards are #19's, pinned by #19's tests, which T4a
leaves green and unedited. *Stop rule:* if a #19 test asserts a tier's number (for example, that
`google:` is tier 1) rather than an order between two sources, it cannot pass after the renumbering;
T4a stops before renumbering, edits nothing of #19's, and the controller puts it to Quinn (Q48).
T4a reads what is on `main` before it starts (§14): whether `registrar:` is still in the
watched set or B3 moved it to its own landing column, the syllabus row's behaviour is the same. Where
this paragraph and the ones below say `SERIES_KEY_PREFIXES` or "an arm in `refresh_series`", read
"the `syllabus:` row's column". Without it a confirmed syllabus class is never watched, and the
change card above never fires. Each of the three §5.4 paths (:3420-3443) then applies as follows:
- **changed:** needs the note's record live on a calendar in `fresh`. So every run in which
  `state/syllabi/<slug>.json` holds a **usable meetings read** (below) and its file is present passes
  `syllabus:<slug>` to `refresh_series` (:2079) as a fresh calendar with that run's series.
- **ended:** needs the key in `file.ended`. A syllabus key enters `ended` only when a usable meetings
  read of a present file no longer returns it. That happens on the same run, not after `UNSEEN_DAYS`
  = 14 (:1363). *(convergence round 3: revision 2 said this was because "re-derivation is
  deterministic". That holds only for `rank` re-deriving series from the stored extraction; reading a
  new copy is a fresh rules-or-model read, §4.3.)* The reason is that waiting would not help. An
  unchanged file is never re-sent (§5), so the read that dropped the key is the only read there will
  be until the student adds another copy. The guard is the usable-read test below, and the end card
  is still a proposal the student decides. This is one syllabus-only arm in `refresh_series`'s
  aged-out loop (:2121-2135), beside the registrar's at-once arm (#19's Plan ruling R2-c).

  **A usable meetings read** *(convergence round 3)*. A meeting missing from a read is evidence of an
  end only when the read could have seen it. Confidence is per part (§4.2, §4.3), so a copy's items
  and grading can stand while its meetings part fails. "The extraction is current" is therefore not
  the test. An extraction holds a usable meetings read only when all four of these hold, each read
  from §5's per-part state:
  1. its `status` is `read`, not `pending` and not `failed:<code>`;
  2. its meetings part was settled by rules, or read by the model at or above the floor;
  3. that part was not clipped (§4.2): a clipped part may have lost the very line that names a lab;
  4. it returned at least one meeting, in any section.

  A copy with no usable meetings read is not evidence about any meeting. That covers a meetings part
  that kept nothing (so was never sent), one below the floor, one clipped, one read with no meeting,
  and a copy still pending or failed. **As recommended (Q50 (a))**, the run treats that course's
  `syllabus:` calendar the way the deletion arm (below) treats a deleted file. It drops the course's
  `syllabus:` series from the series file outright, with **no `ended` entry**. So neither the ended
  arm nor #19's aged-out path files an end card, and a confirmed note keyed `syllabus:` stays as it
  is, unwatched, until a usable meetings read arrives. *(convergence round 4)* That read, a later
  copy's or this copy's once `judge` reads it, watches again only the keys it returns: their series
  come back as fresh, so a moved time gets its change card. A meeting it leaves out has no series
  left in the file to age out. It gets no `ended` entry, no end card and no swap card, and its
  confirmed note keeps blocking its old times. That is (a)'s second cost (Q50); (b) and (c) do not
  have it, because under either the old series is still in the file when the usable read arrives.
  The run's drop changes neither the course's outcome nor the ask gate (§8). A copy that has been read and has no usable meetings part
  reads "could not read class times from it", and the ask fires, as it already does for a meetings
  part below the floor. A pending copy holds the ask for at most `SYLLABUS_WAIT_DAYS`, and a failed
  one releases it, both unchanged. An S2-like Simple Syllabus copy (no meeting times) that
  replaces an LMS copy (§15.5) therefore ends nothing.

  **What the arm stamps** *(convergence round 2)*. A syllabus series has no instances (E5), and
  #19's ended path in `detect_changes` needs a last date: its end is `last_instance.or(until)`, and
  its `successor` needs `Some(old_last)`. Left at `last_instance: None`, a dropped key whose `until`
  is the term end is skipped (the note's `until` is no earlier), and one whose `until` is null gets
  only the "ended with no last instance and no until; no end card" warning on a hard note and nothing
  on a soft one. No end card and no swap card is filed, and the removed and the added lab both block
  time all term. So the arm stamps the `Ended` entry with a date of its own, as the registrar's arm
  does (`today − 1` there). T4a computes these values; it never chooses them:
  - `last_instance` is **the last meeting day before the run**: the latest date `d` with
    `run day − 7 ≤ d < run day` whose weekday is a day of any of the dropped series' `meets`. One
    always exists, because a series has at least one meeting. Two clamps follow, in this order. When
    the series' `until` is earlier, `last_instance` is that `until`: a series already past its own end
    keeps that end, and no card is filed, because the note's `until` is no later. When the day before
    the series' `first` is later, `last_instance` is that day: the series had not begun (Q49).
  - `until` is the dropped series' `until` when it has one, else the `last_instance` just stamped. A
    syllabus key reaches this arm only on a usable meetings read that no longer returns it (above),
    which is evidence of an end, never dormancy. #19's open-ended rule (a soft or optional note whose series had no `until` is never ended by itself)
    must not swallow it, and stamping `until` here leaves that rule unedited for every other source.
  - `calendar` and `dropped` are set as #19's loop sets them.

  With these values #19's ended path files the end card unedited, with `until: <last_instance>`. A
  term-end `until` lies more than `TRUNCATION_DAYS` (7) past the last meeting day, so `last.or(until)`
  picks the meeting day. An `until` within that week and before the note's own is the series' own
  truncation, and it wins, as it does for any source. The hand-set rule on `until` still holds.
  `successor` gets `Some(last_instance)`. It files one swap card instead of the end card when a live
  series of the same course and kind (from any source, under #19's own filters), with a different
  meet set, has a `first` on or after that day. A new key whose `first` is null or earlier (a syllabus dated from week 1) gets no swap
  card. The student then gets the end card for the old key and a check card for the new one, the
  two-card outcome the key paragraph above names for a removed lab and an added one. A
  re-extraction that only relabels a section keeps the meet set under a new key. A live series then
  still carries the note's signature, so #19's path files nothing, as it does for any source.
- **succeeded:** unchanged. It applies to `class` and `lab` across keys of one course.

**Deleting the file (Q15, recommended (a)).** Deleting `syllabi/<slug>.<ext>` is not evidence that a
class ended. The next run drops that course's `syllabus:` series from the series file outright, with
**no `ended` entry**, so no end card is filed. A confirmed note keyed `syllabus:` stays as it is,
unwatched, until a syllabus for that course is added again. The same rule covers a second desktop
(§5), which never has the file. This is a second syllabus-only arm in `refresh_series`: a `syllabus:`
calendar whose file is absent is neither `configured` (:2100-2103) nor kept while recent.
*(convergence round 3)* The same arm takes a `syllabus:` calendar whose present copy has no usable
meetings read (above, Q50 (a)). It is the arm's second trigger, not a third arm.
*(convergence round 4)* After either trigger, the next usable meetings read (a re-added copy's, or a
pending copy's once `judge` reads it) watches again only the keys it returns; a meeting it leaves out
is never ended (the usable-read paragraph above, Q50).

**`create_confirmed_as`** (:3883) gains a body line for the new prefix, beside the `gcal-series:`
branch at :3929: "Found in your syllabus.\n".

**Section match.** If the extraction has one section, it is used. If it has several, the label is
matched against the course note's code or LMS section. With no unique match, each section's rows are
proposed, labelled "CS 100 section 002", and the engine never picks one. The same match chooses which
items apply when items carry a `section` (§4.6).

**Precedence uses the tiers PR #19 lands.** *(E4, amendment 2026-10-18; replaces revision 2's
`source_rank` paragraph in full.)* `precedence` is `(tier, calendar)`: tier 0 for a `registrar:`
calendar, 1 for `google:`, 2 for anything else, and the collapse's `rank` (one series per meeting
signature) orders by `(precedence, source_uid)`. This lane inserts `syllabus:` as tier 1, in its
per-source table row, and **renumbers** `google:` to 2 and the rest to 3 (Q6; the one edit outside
the syllabus row, stated above). *(convergence round 1)* `syllabus:` must hold a tier of its own:
sharing tier 1 with `google:` would break the tie on the calendar string, `"google:…"` sorts before
`"syllabus:…"`, and Google would win the collapse, reversing Q6. `precedence` has four readers on
#19's branch, and each keeps every existing pair's order after the renumbering: `by_key` (one
`source_uid` under two calendars), the collapse's `rank`, the aged-out loop's order in
`refresh_series`, and the swap pick in `detect_changes`. So no existing behaviour moves; only
`syllabus:`'s place among them is new. Because a wrong tier silently reverses which source wins in
all four, the precedence change is T4a's (`integrator`), not a cheaper agent's (§13). A
syllabus row and a calendar row that agree (same kind, course and meet set) collapse to the syllabus
row. A course the registrar holds, by a confirmed in-play `registrar:` class or lab note or a current
registrar series of that kind, gets no syllabus row of that kind at all (PR #19's per-kind hold, its
spec's Amendment 6); a held row is never `contested`.

**Syllabus series stay out of the instance map.** *(E5, amendment 2026-10-18)* A syllabus series
carries no instances. `SeriesFile::instances_map` treats a series' instances as the only busy time
inside its 28-day horizon, so a confirmed syllabus class would block nothing there. Syllabus series
are left out of the map, as PR #19 leaves registrar series out (its plan ruling R2-d), and their
notes block time by `meets`. In the table this is the row's "no instances" column.

**Contested rows.** Each proposal gains a field `contested: bool`, carried through `--json` and
`week::commitment_proposals` to the console. It is true when another live proposal has the same
course and kind but a different meet set, from any source. Two syllabus sections the engine could not
choose between are contested. So are a syllabus row and a Google row that disagree. **A contested row
is never preset "Mine"** (Q12). `console.js:830` presets by kind today, and T11a makes it skip a
contested row. Both rows show, and the student picks.

### 4.5 The student confirms meetings (existing path, unchanged)
On day 1, the rows join "Your classes" on the confirm screen (`paintSetupRows`, `console.js:838-850`)
with Mine / Not mine. They go through `commitments_confirm` (`app/src/week.rs:71`) to
`commitments::confirm` (`commitments.rs:4476`) and `create_confirmed_as` (:3883). They are written as
the student through `write`, journal first. After day 1, a syllabus series files a `commitment-check`
card like any new calendar series (`emit_checks`, `commitments.rs:2973`). Either way the course is no
longer uncovered, so its `commitment-ask` card withdraws itself (`commitments.rs:3248-3258`;
`withdraw_asks`, `cli.rs:867`).

A syllabus is "Knowlu noticed", not an authoritative source (`VISION.md:42-46`): it proposes and
waits. A re-added syllabus that moves a confirmed class's times or room files a change card through
the **changed** path (§4.4). A field the journal shows the student set by hand
(`journal::human_edited`, `journal.rs:416`; `is_human_set`, `commitments.rs:3412`) is never proposed
back over.

### 4.6 Dated items into proposals (device, deterministic, in `rank`)
`rank` reads each `state/syllabi/<slug>.json` with a current extraction and files cards from its
`items`, as the agent actor **`agent:knowlu.syllabus`** (it starts `agent:`, so `provenance::is_agent`
reads it as an agent and judge once holds). The file is local and deterministic, so the same vault,
file and day give the same cards. A vault with no `state/syllabi/` files nothing, so every frozen and
surface reference stays byte-identical.

**Resolving the date.** An item with a `date` uses it. An item with a `week` and `day` resolves to
`week1_start + 7 × (week − 1)` plus the day's offset, only where the extraction carries a
`week1_start` the syllabus itself gave. Otherwise, and for any date more than 9 months from the add
date, the item **needs a date**: it is shown on the course and never proposed (`VISION.md:51`, Knowlu
never invents a due date). It is proposed once a date appears, from a re-added syllabus or the feed.

**Resolving the time.** An item's `end` (else `start`) is its due time. An `in_class` item with no
time takes the start and end of the course's confirmed `class` note that meets on that weekday, read
from `commitments/`, deterministically; with none, it has no time. An assignment with no time is due
on a bare date, which the engine already reads as 23:59 (`approvals::as_note_datetime`,
`approvals.rs:146`); its card says "no time given".

**The key.** `syllabus:<course-slug>:<kind>:<normalised title>`, with `@<YYYY-MM-DD>` appended only
when the same normalised title repeats in one extraction (a weekly "Quiz"). The normalisation is the
email lane's: case, punctuation and whitespace folded (`email-forwarding-design.md:440-444`). Items
whose `section` names a section the engine did not match (§4.4) are skipped.

**Which items are filed, and when (Q21, Q24).** An item is filed once it is due within **14 days**
(an assignment or quiz) or **21 days** (an exam) of today, and never once past. At most **8** syllabus
cards are first proposed a day, soonest first; each is charged 1 to the 15-a-day budget
(`proposal_weight`, `approvals.rs:855`), and anything over is snoozed by `defer_over_budget` (:921),
never dropped. On the vault's first day (`commitments::vault_day` is 1) a card is written with
`proposed_at` and `snooze_until` both day 2, as mail cards are (`email-forwarding-design.md:453-456`).
Items further out are listed on the course ("9 more dates from your syllabus; each is proposed two
weeks ahead").

**The duplicate check (Q22), PQ13 (b)'s rule.** Before filing, the filer looks in `tasks/`, `archive/`
and the pending task and exam cards in `approvals/`, in the item's course, for a match:
- **the title matches** when the normalised titles are equal, or every token of one is in the other,
  or both reduce to the same **kind-and-number key** (a fixed synonym table: `hw`, `homework`,
  `assignment`, `problem set`, `ps` → assignment; `quiz`; `exam`, `test`, `midterm` → exam; then the
  first integer: "HW 3" and "Homework 3: Chapter 4" are both `assignment-3`);
- a title match with **the same due day, or an existing note with no due**, is a duplicate: nothing
  is filed, and the item is recorded `duplicate` with the note it matched;
- a title match with **another due day**: where the existing note came from the LMS feed (its
  `source_uid` is not `syllabus:`), the feed is the authority (`VISION.md:46`). Nothing is filed, and
  the course shows a notice, not a proposal: "Your syllabus says Oct 12; Blackboard says Oct 14.
  Knowlu uses Blackboard's date." Where it is a pending card from mail, both stay, as PQ13 (b) rules
  for two noticed sources.

A key already in `approvals::existing_source_uids` (`approvals.rs:1080`; `approvals/` and `archive/`)
is never filed again: a rejected card stays rejected, as an archived event card does (events D9).
*(convergence round 1)* The one exception is a key whose only history is cards this lane withdrew as
`superseded`; it files again (see "How a syllabus card is withdrawn", below).

**The cards.**
- **An assignment or a quiz** is a `kind: task` card whose fenced payload is the task note: `title`,
  `course`, `due`, `source_uid` (the key). Approve runs the existing `materialize`
  (`approvals.rs:1836`), with no change to `approvals.rs`. The card's context line quotes the one
  schedule row it came from.
- **An exam** is a `syllabus-exam` card (Q20). Like `event-accept` (events D1, D10), it carries its
  own payload: one instance (date, start, end, where, the key). Approve writes a one-off `exam`
  commitment, hard (`commitments.rs:66`), through `create_confirmed`, and a task "Study for <title>"
  due at the exam's start, as `event-accept` writes its "Register" task (`approvals.rs:1784-1825`). An
  exam with no time writes only the study task, due on a bare date, and the card says "No time in the
  syllabus; add it on the schedule".
- *(review round 1)* **What each card carries, since every one syncs** (`approvals/`, then
  `archive/`). A task card: `title`, `course`, `due` and `source_uid`, and a context line quoting the
  one schedule row it came from (at most 240 characters, §4.2's clip). A `syllabus-exam` card: the
  title, the course, the date, `start` and `end` where given, `where` as the syllabus writes it (one
  line, at most 80 characters, `eventaccept.rs`'s `WHERE_MAX`; longer is left out) and the key, plus
  the same one-row quote. The grading amend card: the proposed `grade_weights`, `grade_scale` and
  `late_policy` values and the `source_uid`, and a context line in Knowlu's own fixed words built
  from those values ("Grade weights, a grading scale and a late-work rule from your syllabus"). **No
  card carries a policy quote**: the console shows the quote beside the amend card from the local
  extraction file, on the desktop that has it (§16.4). §6.1 item 4 states exactly this list. Whether a
  pending exam card may carry its `where` is Q46.

**When the syllabus changes.** A re-extraction that moves an item's date withdraws its pending card as
`superseded` and files a fresh one. For a task already approved and still keyed `syllabus:`, it files
a `kind: amend` card for `due` (`AMENDABLE_FIELDS`, `approvals.rs:43`), whoever set the field, since
an amend card is itself the proposal judge once allows. An item that a re-extraction drops changes
nothing: a syllabus edit is not evidence that work was cancelled (Q15's reasoning).

**When the feed brings the item later (Q23).** Syllabi name the term's work on day one; the feed
often shows it weeks later. Each `rank` re-runs the duplicate check on pending syllabus cards and
withdraws, as `superseded`, any card a feed item now matches; the run line names it. For an approved
task still keyed `syllabus:`, `ingest` adopts it: before creating a task for a new feed item, it looks
for an active `syllabus:` task in the same course by the same title rule and, on a match, sets that
note's `source_uid` to the feed's uid (an agent write through `write`, journal first) instead of
creating a second task. From then the feed governs it: a different due is applied and listed as any
feed move is, and a due the student set by hand gets an amend card instead (cloud ruling 3's
consequence). Only `ingest`'s new-item branch changes, and only when a `syllabus:` task exists.

**How a syllabus card is withdrawn** *(convergence round 1)*. Moving an approval card is
`approvals.rs`'s job, as it is for commitment cards (`withdraw_stale` and `withdraw_asks` each walk
one kind; `process_approvals` finishes a card whose `superseded` stamp landed but whose move did not).
So the two withdrawals above are split the same way:
- **The reason is the filer's, and pure.** `syllabus_cards::withdrawal_reason` (T4d) takes a card's
  frontmatter, the current extraction and the vault's tasks, and returns a reason or none: *date
  moved* (the item's resolved due day is no longer the card's), *authoritative match* (§4.6's
  duplicate check now finds a feed or `coursework` note, §16.5), or *set changed* (a pending grading
  amend card whose `source_uid` hash is not the current extraction's, so one course never holds two
  pending grading cards). It reads and writes nothing else. An item a re-extraction drops is not a
  reason (Q15's reasoning, above).
- **The move is `approvals.rs`'s.** `approvals::withdraw_syllabus` (T4e, contract list) walks
  `approvals/` for `pending` or `snoozed` cards that are `kind: task` with a `syllabus:` `source_uid`,
  `kind: syllabus-exam`, or `kind: amend` with a `syllabus:` `source_uid`, asks the reason, and on
  one stamps `status: superseded` and archives the card through `write` and `delete`, journal first,
  exactly as `withdraw_asks` does: never raising into the run, one `transition failed:` warning per
  card it could not move, and a `Withdrawal` whose `pending` count `rank` takes back out of its
  `Approvals: N pending` line. `process_approvals`'s finishing arm (today `commitment-check` and
  `commitment-ask` only) gains the same three shapes, so a half-moved card is archived quietly on the
  next run and never sits in `approvals/` holding its key.
- **`rank`'s order:** `withdraw_syllabus` runs before the filer, so a fresh card never meets the
  card it replaces.
- **Superseded is not a decision.** `existing_source_uids` counts `archive/`, so the archived card
  would block its own replacement (a moved date keeps the item's key). The filer, read-only in
  `syllabus_cards.rs`, therefore blocks a key only when some note holding it is *not* an archived
  `superseded` card: a key whose only history is withdrawals files again, while an approved or
  rejected card still blocks for good. An authoritative-match withdrawal does not refile, because the
  duplicate check stops it first.
- **An amend card for a moved due** (an approved task still keyed `syllabus:`) is keyed
  `<item key>:due:<YYYY-MM-DD>`, the new day, so a rejected move is never proposed again and a later
  move is a new card; a pending one whose day moved again is withdrawn as *date moved*.

`syllabus_cards.rs` never stamps, moves or deletes an approval card itself: it writes only new card
notes through `write`.

### 4.7 Grade weights and scale (device, deterministic)
**Valid only when whole.** Weights are proposed only when they sum to 100 (±1); a scale only when its
letters come from the fixed set and its minimums fall strictly. Otherwise the course shows what was
read, with the reason ("These add to 110%; check your syllabus"), and nothing is proposed.

**Stored on the course note**, as two single-line flow mappings through `write`'s frontmatter surgery,
emitted by `yamlemit` as `meets` and `judgment:` are: `grade_weights: {Exams: 45, Homework: 20, ...}`
(the syllabus's own category labels) and `grade_scale: {A: 93, A-: 90, ...}` (minimum percent).

**How they land (Q25): one `kind: amend` card per course**, filed by the same `rank` pass, target
`courses/<slug>.md` (`AMENDABLE_FOLDERS` already holds `courses`, `approvals.rs:55`), changes
carrying whichever of the two is valid. Its `source_uid` is `syllabus:<slug>:grading:<first 12 hex of
the SHA-256 of the canonical values>`, so a rejected set is never proposed again and a changed set is.
It is charged 1 to the budget and held to day 2 on the first day, as items are. `AMENDABLE_FIELDS`
(:43) gains `grade_weights` and `grade_scale`, and `apply_amendment` (:697) validates a flow mapping
for them: contract-list work (T4e). *(amendment 2026-10-18)* The same card carries a valid
`late_policy` (§16.3, Q34), and its `source_uid` hash runs over the canonical values of all three
fields, so a changed late rule is a changed set.

**What reads them.**
- **Importance, now.** `judge::Heuristics::load` (`judge.rs:389-418`) renders `grade_weights` as
  `- <category>: <n>%` lines ahead of the `## Grade weights` section's own text, both within
  `MAX_WEIGHTS_CHARS`. Tier 3 then grounds importance in them, the slot that exists (`judge.rs:591`).
  The body section stays the student's own and is never rewritten.
- **Letters, now.** The grades read model's `letter` (`grades-design.md:224`) uses the course's
  `grade_scale` where it has one, G3's default otherwise: the slot G3 names
  (`grades-design.md:19`). The surface references have no `grades/` folder, so they do not move.
- **The ring's basis, later.** G2 keeps Overall Grade, then points so far (Q26).

### 4.8 Office hours (shown, never proposed: ruling 1)
Office hours come from the extraction's `office_hours` and are **shown on the course**: on its row in
"Your syllabi" and on the course's entry in the Schedule view, as "Office hours: Tue, Thu 2:00-3:30,
Sample Building 204 (from your syllabus)" (an invented room). They are never a commitment, a card or
busy time, and are not drawn on Today. That matches `VISION.md:48` ("office hours ... shown, not
proposed"), the level they would carry (optional, `commitments.rs:68`) and the card filter that
already drops them (`commitments.rs:2369`). They are read from the local extraction file, so no note
is written for them.

**The `syllabi` read model** *(shape stated in review round 1; built by T17)*. `surface` gains a
top-level `syllabi` list, serialized only when non-empty, so `surface-today-*.json` stay
byte-identical. It is read from `state/syllabi/` only (each `<slug>.json`, and `held/`), never from
`syllabi/` files, and carries **codes and values, never row text**: the console words every code
(§8's table is the draft wording). One entry per course with a state file or a held rendering
naming it, in course-slug order:
- `course` (the slug) and `source` (`lms`, `simple-syllabus`, `upload`, `paste`, or absent);
- `steps`: per step (`lms`, `simple-syllabus`, `add`), its §8 outcome code;
- `status` (`pending`, `read`, `failed:<code>`) and `parts`: per part (meetings, items, grading,
  policies), `rules`, `model`, `below-floor`, `clipped` (with the last date read) or `none`;
- `office_hours`: the extraction's list (days, start, end, `where`);
- `items`: every item not filed, in date order, each with `kind`, `title`, `date` or null, and
  `state` (`later`, `duplicate`, `needs-date`, `feed-differs`, `past`), plus the matched note's id for
  `duplicate` and both dates for `feed-differs`; and `counts` by state (§16.5's one line);
- `grading`: the values read and, when not proposed, the reason code (`sum`, `scale-order`);
- `policies`: the late rule's values or null, and the `late`, `makeup` and `attendance` quotes;
- `held`: per held rendering naming this course, its `id`, `reason` and candidate slugs (§4.1).
T16 then adds the late note to a past-due task's own row (§16.3); T17 does not touch task rows.

## 5. Data: what is stored where

| Thing | Where | Leaves the device? |
|---|---|---|
| Fetched bytes before Finish, and pasted text | `state::app_data_root()`/`pending/syllabi/<random>/` (§4.1), deleted at Finish or once handed over, swept after an hour | No. Never in the vault |
| The original file | `<vault>/syllabi/<course-slug>.<ext>` (Q8): the first non-markdown, non-generated files in a vault | No. `syllabi/` is not in `ids::NOTE_FOLDERS` (`engine/src/ids.rs:23-24`), and sync pushes only `.md` notes under those folders (`sync::is_note_path`, `sync.rs:365-376`; the server's `NOTE_PATH_RE`, quoted at :359). The backup mirror copies it to the student's own backup folder (`backup.rs:37-39`) |
| Extracted text and excerpt | Memory only. `syllabus-add` and `judge` each re-derive them from the file; the screen is deterministic, so both get the same bytes | Only the clipped excerpt, for one call |
| The extraction, office hours, item outcomes and step outcomes | `<vault>/state/syllabi/<course-slug>.json`, written whole through `ledger::dumps_value` | No |
| Series | `state/calendar-series.json`, as today | No (P21 keeps it off the wire) |
| Class proposals and cards | `commitment-ask` and `commitment-check` cards | No (`LOCAL_CARD_KINDS`) |
| Task, exam and grading cards | `approvals/`, then `archive/` | Yes, as every deck card does. *(review round 1)* Field by field (§4.6): a task card's title, course, due and one quoted schedule row; an exam card's title, course, date, times, `where` (Q46) and one quoted row; the grading card's proposed weights, scale and late rule. No policy quote |
| Confirmed class and exam notes; approved tasks | `commitments/`, `tasks/` | Yes, as every note does today |
| Grade weights and scale | the course note's frontmatter | Yes, with the course note |
| *(2026-10-18)* The late-work rule (`late_policy`) | the course note's frontmatter, once the student approves the course's amend card (§16.3) | Yes, with the course note |
| *(2026-10-18)* Policy quotes (late work, makeup, attendance) | `state/syllabi/<slug>.json`, inside the extraction (§16.4) | No. Shown on the course and beside the amend card, on this desktop only, like office hours; never on a card (§4.6) |
| *(review round 1)* Held renderings (§4.1, Q47) | `state/syllabi/held/<id>.html` and `<id>.json`, deleted when settled or after 30 days | No |
| *(2026-10-18)* A Simple Syllabus document as the host sent it | staging under the open profile's folder (§4.1), for one `syllabus-add` run, then deleted | No. Never in the vault |
| *(2026-10-18)* The engine's rendering of that document | `<vault>/syllabi/<slug>.html` (§15.3, Q31): the term dates, the section attributes and the `content` and `instructor` components, in order | No, as every file in `syllabi/` |

**The extraction file is a byte contract from its first release.** It carries `"version": 1` and
holds:
- the file's name and SHA-256 (`ring`, already an engine dependency), so an unchanged file is never
  re-sent;
- *(review round 1)* `source`: `lms`, `simple-syllabus`, `upload` or `paste`, the `--source` the copy
  came in by. §4.1's replacement rule (Q33) reads it, so it is in `version: 1` from the first
  release rather than a later bump;
- `added` (the vault-local date of the add);
- `status`: `pending`, `read` or `failed:<code>`, and per part (meetings, items, grading) whether it
  was settled by rules, read by the model, below the floor or clipped, *(convergence round 3)* or
  empty (the screen kept nothing, so it was not sent). `rank` reads the meetings part's state for
  §4.4's usable meetings read, so the states are part of `version: 1`;
- the outcome per step (§8) and per item (`filed`, `duplicate`, `needs-date`, `later`, `feed-differs`);
- the extraction itself.

A reader that meets an unknown `version` treats the course as needing a re-read from its file and
never guesses. Like `state/grades.json` (`grades.rs:322`), it is generated, written whole by its one
writer, and holds no field a human edits. The student's edits live on the confirmed notes, tasks and
course note, where "judge once" already protects them. `scaffold.rs` creates `syllabi/` with the
vault. `ids.rs` and `sync.rs` are not edited, and the tripwire
`the_servers_note_path_rules_name_every_note_folder` keeps holding because the folder list does not
change.

**A second desktop (Q18).** `syllabi/` and `state/syllabi/` never sync, so a second desktop
(`VISION.md:148`) has neither. What the student already decided does sync: confirmed class and exam
notes, approved tasks, the course note's weights and scale, and every deck card in `approvals/` and
`archive/`. A second desktop files no syllabus card, because it has no extraction, and could not
double one anyway, because `existing_source_uids` sees the synced cards. Under Q15 (a), confirmed
syllabus classes are unwatched there and never ended. It shows "Add syllabus" for a course it has no
file for, and no office hours. The relay and two-desktop work (Launch, `HANDOFF.md:366-369`) is where
syllabus files could ride the account, if Quinn wants them to.

## 6. Privacy

- **A new content class reaches a model.** The privacy page bounds what travels: "the note's title
  and up to the first 1,200 characters of its text... for an email... up to the first 1,200
  characters" (`site/privacy.html:56`). That bound is `MAX_BODY_CHARS`
  (`cloud/supabase/functions/_shared/judge_prompts.ts:15`), and its price is worked out in
  `judge_caps.ts:17-23`. A syllabus excerpt is neither a note nor an email. The page's change clause
  (:119-121) says a materially different use needs an email and an in-app yes first.
- **Its own input bound.** `judge_prompts.ts` gains `MAX_SYLLABUS_CHARS`, beside `MAX_BODY_CHARS`,
  at 13,500 with the policies part if Q38 is ruled as recommended (12,000 otherwise), and the part
  bounds (§4.2, §16.2). It is clipped with the same marker and pinned by a test the way
  `judge_prompts_test.ts:14-42` pins the body. The engine clips to the same numbers before sending
  (§4.2). The page states the total.
- **Which page carries it (Q13, restated 2026-10-18).** *(review round 1: revision 2's choice between
  privacy bump #1 and a bump #2, and its draft page paragraph, are withdrawn; neither is to be
  lifted.)* Lane L4's honest page carries the syllabus lines now, with its `PRIVACY_VERSION` move;
  the lawyer reads the whole page before the Pilot. The lines are §6.1's six statements, and nothing
  else in this section is page text. The release guard is §13 T12's, and what "off" means in a
  release is Q42's merge gate.
- **The screen is the minimization.** The model sees four bounded parts, not the syllabus (§4.2), and
  never the instructor block. A student's own notes on a file (accommodations, marks) usually fall
  outside those lines. But the screen is a reduction, not a scrub: a schedule row can carry a name
  ("Guest lecture: Dr. Lee"), so the page says "lines", not "nothing personal".
- **What syncs that did not before.** *(review round 1)* Each deck card's fields, as §4.6 lists them:
  a task card's title, course, due and one quoted schedule row; an exam card's title, course, date,
  times, `where` (Q46) and one quoted row; the grading card's proposed weights, scale and late rule,
  as values. Once approved, the same values sit on the course note, the task and the exam note. These
  are coursework of the same class as the LMS feed's titles, which already sync. **Policy quotes never
  sync**: they live in the local extraction file and no card carries one. §6.1 item 4 says this to the
  student, and the lawyer's read before the Pilot confirms it.
- **What the service logs.** `judgments.item_id` is the course note's opaque id (§4.3), never the
  slug or the file's hash. A hash would let the service see that two students hold the same syllabus.
- **Which host may read it.** Today only the pinned hosts named on the page read content: CoreWeave
  for assignments and events, DeepInfra for email (`privacy.html` ~:67-71). The `syllabus` kind is
  pinned to one of those (Q9), so the page names no new processor.
- **Who owns the text.** The instructor owns a syllabus; the student holds a copy. Knowlu reads the
  student's copy for the student, keeps only what it extracted, and never shares or aggregates syllabi
  across students.
- **Step 1 and Offer A read a portal as the student.** They use the same sign-in pattern as the course
  list, so the university-policy read (cloud ruling 12) applies as it does there. It is suspended until
  20 paying users (Amendment 2026-09-30, A13). Offer A adds a third party's terms (Q10).

### 6.1 What lane L4's page must add (amendment 2026-10-18)

Quinn's ruling 5 of 2026-10-18 makes the privacy page honest now: `site/privacy.html` says what the
app does today, syllabus reading included, and `PRIVACY_VERSION` moves so testers accept it. **Lane L4
owns the page**; this lane writes no line of it. T12 hands L4 these statements, worded for the
student, and the release guard (§13 T12) holds until the page carries every one that applies to what
the release turns on. The lawyer reads the whole page before the Pilot (ruling 3, "lawyer later"), and
the re-consent screen L4 builds re-prompts everyone then. Q13 is restated to match (§11).

1. **Where a syllabus comes from:** the student's course in the LMS, a file or text they add, and, at
   schools Knowlu supports, the school's syllabus system (Simple Syllabus), read in its own sign-in
   window on the computer, as the student, with nothing of that sign-in kept after the read.
2. **What stays on the computer:** every syllabus file, Knowlu's copy of a Simple Syllabus document,
   and everything read from them that is not in a suggestion (item 4) or approved: office hours, the
   policies in the syllabus's own words, and the dates not yet suggested (`syllabi/`,
   `state/syllabi/`). *(review round 1: revision 2's "not approved" was wrong, since a pending
   suggestion syncs.)*
3. **What one model call sees,** with the total bound stated as a number (12,000 or 13,500
   characters, Q38): the course's code, section and term; the lines that say when and where the class
   meets and when office hours are, with e-mail addresses, phone numbers and links removed; the lines
   of the schedule that carry a date; the lines that say how the course is graded; and the paragraphs
   that state the late-work, makeup and attendance policies. The instructor's contact details are
   never sent. The model and host are the ones the page already names (Q9); neither keeps the text.
4. **What is kept with the account** once the student approves it: classes and exams, assignments
   and quizzes as tasks, and on the course, its grade weights, grading scale and late-work rule.
   *(review round 1)* **What a suggestion carries before the student decides**, since suggestions are
   kept with the account too: for an assignment or quiz, its title, the course, the due date and the
   one schedule line it came from; for an exam, the same plus its times and the room the syllabus
   names (Q46); for the course's grading, the weights, scale and late-work rule as Knowlu read them.
   The syllabus's own wording of a policy is never in a suggestion and never leaves the computer.
5. **What Knowlu never does with a syllabus:** share it, pool it across students, or keep a copy on
   the service.
6. **The third party:** Simple Syllabus is the school's vendor. Reading it adds no data flow to the
   vendor beyond the student's own signed-in page; the page names it and links its privacy policy.

Items 1 and 6 apply only to a release whose curated rows carry a syllabus host (Q39); items 2 to 5 to
any release from a `main` carrying this lane's app surfaces (T8a, T9, T11a onward), which by Q42's
merge gate cannot reach `main` before L4's page carries them. The registrar's own sentence ("…at schools Knowlu supports,
your class schedule…") rides the same page by Quinn's ruling (5) on #19's close-out.

## 7. Cost per student

This is arithmetic, not measurement (the eval is T7). Prices are the pinned rows in
`cloud/supabase/migrations/20260916000100_provider_swap.sql:34-47`: Granite 4.2 8B at $0.10 in and
$0.15 out per million tokens; Qwen3.5-35B-A3B at $0.14 in and $1.00 out. Tokens are about 4
characters each (`judge_caps.ts:15-23`). The wider schema needs a longer prompt (~2.4k characters,
~600 tokens) and a longer answer: about 25 tokens an item, so ~1.3k for a 40-item term plus ~300 for
meetings, weights and scale; the output cap is 2k.

| Case | Tokens in / out | Granite | Qwen |
|---|---|---|---|
| Rules settle every part, or nothing kept | 0 | $0 | $0 |
| Typical excerpt (~6,000 characters) | ~2.1k / ~1.2k | $0.0004 | $0.0015 |
| Full excerpt (12,000 characters) | ~3.6k / ~2k | $0.0007 | $0.0025 |
| Whole syllabus, no screen (rejected, Q4), 5-20 pages | ~4.4-15.6k / ~2k | $0.0007-0.0019 | $0.0026-0.0042 |

A student with 6 courses, two re-adds and a retry or two makes about 10 calls a term: about $0.007 a
term on Granite and $0.025 on Qwen, or at most about $0.007 a month. The scoping rule is $0.25 a
student a month (`docs/notes/2026-09-16-inference-provider-and-model-scoping.md` §1 item 2). **The
screen no longer buys a 10x saving**: output now dominates, so it saves about 1.5-2x. Its reason is
minimization (§6), not cost. The runaway guard is the $2 monthly ceiling (`judge_caps.ts:66`). The kind
needs its own row in `DAILY_CAP` (`judge_caps.ts:45`, today `{ task: 60, event: 80, email: 120 }`,
pinned by `judge_caps_test.ts:11`). **20 a day** is proposed: one call a syllabus, so enough for a full
term's courses in one sitting, with retries. The scoped flag (§4.3) means an upload charges only this
row. *(review round 1)* `DAILY_CAP` is one code constant that staging and prod share (one function
deploy each); it is a volume guard and is **never** an off switch for an environment. Q42 is how a
release is kept from reading syllabi, and it needs no per-environment cap.

*(amendment 2026-10-18)* The policies part (§16.2) adds at most 1,500 characters in (~375 tokens) and
about 150 tokens out a call: at most about $0.0003 a call on Qwen, so the term figures above hold to
the second decimal. A Simple Syllabus Connect reads every section at once (eight at UA in S2), one
call each, inside the 20. The existing first-days rule doubles every kind's daily cap on an account's
first two judging days (`20260923000100_first_day_cap.sql`), so a new student's Connect has 40. The
first-day **event** allowance (`docs/specs/2026-10-06-first-day-event-allowance-design.md`, lane L3)
is for `event` only, and `syllabus` gets none (Q41): one call a syllabus never builds a backlog.

## 8. Failure and silence rules

Every step, for every course, ends in exactly one named outcome, and every part of a read syllabus
(meetings, items, grading) ends in one too. The outcome is stored (§5), shown on the course's row or
card, and named in the run log. No outcome is ever an empty row. No step's failure stops the next:
this is the slot's rule that a step which cannot run is left out and named, never run and failed.

| Where | Outcome (row text, draft) | Next |
|---|---|---|
| 1 | "Found the syllabus in Blackboard" / "in Canvas" | extract |
| 1 | "No syllabus in this course on Blackboard" | the offer (A where available, B) |
| 1 | "The syllabus is in Simple Syllabus" (a gated link) | the offer, A first |
| 1 | "Blackboard would not show this course's content (403)" | the offer |
| 1 | "Not tried: the sign-in window closed first" (or the per-course timeout) | the offer |
| A | "Simple Syllabus: not connected" (with a Connect button) | the student |
| A | "Simple Syllabus: no syllabus published for this section yet" | B; retried at the next Connect |
| A | "Simple Syllabus: signed out" | Reconnect |
| A *(2026-10-18)* | "Simple Syllabus: read 8 syllabi" (a count, never a course name in a log) | extract |
| A | "Simple Syllabus would not hand over this syllabus" (the in-page read refused, §15.2) | B |
| A | "Simple Syllabus lists a course Knowlu does not have" / "matches two of your courses; pick one" (§15.4; the pick settles the held rendering, §4.1) | the student |
| A *(review round 1)* | "You added your own syllabus for this course; Knowlu kept it" with "Use Simple Syllabus's instead" (§15.5; settles the held rendering) | the student |
| A | "This syllabus links a PDF; add it with Add syllabus to read it too" (§15.3) | B, optional |
| A | "Simple Syllabus: not tried, the window closed first" (or the per-section timeout) | Connect again |
| policies | "Late work: −10% a day, up to 3 days (from your syllabus)" / "Read a late-work policy; could not turn it into a rule" (§16.3) | the deck / shown |
| B | "Add syllabus" (no file yet) | the student |
| B | "This file has no text Knowlu can read (a scanned page?). Paste its text instead." | paste, or C for class times |
| B | "Knowlu reads PDF, Word, web pages and text files" | the student |
| any | "This syllabus names no class times, dates or weights" | the offer |
| any | "Found times for sections 001, 002 and 003. Pick yours below." | the student |
| any | "Found a syllabus; could not read <class times / dates / weights> from it" (that part below the floor) | the other parts stand; the offer stays for class times; *(convergence round 3)* a class-times miss ends no class (§4.4's usable meetings read) |
| any | "Read the schedule up to Nov 20; the rest is past what Knowlu reads at once" | the student may paste the rest |
| items | "14 dates found: 5 to review now, 9 proposed two weeks ahead" | the deck |
| items | "Blackboard already has 6 of these" (duplicates, §4.6) | nothing |
| items | "Your syllabus says Oct 12; Blackboard says Oct 14. Knowlu uses Blackboard's date." | nothing |
| items | "2 dates need a date: the syllabus gives week numbers but not when week 1 starts" | a re-added syllabus or the feed |
| grading | "Grade weights to review" / "These add to 110%; check your syllabus" | the deck / nothing |
| extract | "Reading with this run" (a slot holds the flag, §4.3) | the slot |
| extract | "Waiting to read: no network. Knowlu will try at the next run." | slot retries, bounded below |
| extract | "Not read: no active subscription" (the entitlement line, exit 0) | after renewal, bounded below |
| extract | "Not read: the daily limit was reached; tomorrow" | next day, bounded below |
| extract *(review round 1)* | "Not read: Knowlu's service cannot read syllabi yet" (the route is missing, a 404, or the service does not know the kind: a build ahead of its server) | next run, bounded below |
| C | "No class times found; Knowlu will ask this week" (today's text, kept only here) | `commitment-ask` |

Two existing strings change. `console.js:843` ("no class times found; Knowlu will ask this week") is
shown only once the ask gate below lets the ask fire, and `app/tests/static_assets.rs:1754` is updated
in the same commit, with the reason. The whole-screen status at `console.js:876` gains a second clause
when any course has no syllabus: "...Add a syllabus for each class below."

**The ask card waits for a syllabus being read, and only for a bounded time (Q16).** The ask is held
back only by a syllabus that is actually in progress. A missing syllabus never holds it, so a student
who never adds one gets exactly today's behaviour. `emit_asks` (`commitments.rs:3120`), after its
`ASK_FROM_DAY` check (:3130), skips a course on a given day only when both of these hold:
1. its `state/syllabi/<slug>.json` has `status: pending` (a file is present and is waiting on
   network, entitlement, the daily cap or *(review round 1)* a service that cannot read syllabi yet);
   and
2. fewer than **`SYLLABUS_WAIT_DAYS` = 2** vault days have passed since that file's `added` date.

In every other case the ask fires as it does today. That includes:
- no syllabus file at all ("Add syllabus", no file yet), which counts as exhausted;
- `failed:<code>`, or a meetings part below the floor;
- every step-1 and Offer A "nothing found" outcome;
- a pending file older than the bound.

The card then carries the offer above the typed fields, plus the pending note where there is one
("Your syllabus is still waiting to be read"). The student can always skip straight to typing: the
order is a default, not a gate. p3's D7 (asks wait for term start) composes with this; the later of
the two holds (§9).

**Silence the wider scope must not create.** An item that is not proposed is always listed on its
course with its reason (later, duplicate, feed differs, needs a date); a grading read that is not
proposed shows what was read and why. A card withdrawn because the feed now has the item is named in
`rank`'s run line. Nothing a syllabus said disappears without a line saying where it went.

## 9. The registrar (PR #19), which lands first

*(E6, amendment 2026-10-18; replaces revision 2's §9 in full.)* The registrar (commitment phase 3)
reaches `main` before this lane, by Quinn's ruling of 2026-10-18. It is the other planned class-time
source, and the two share one shape: a series source with its own calendar key
(`registrar:<school>:<term>`; `syllabus:<slug>` here), a keep-alive rule, a button on the confirm
screen and the Schedule view, and a per-kind hold over Google and ICS.

How they fit:
- **The seam is the registrar's.** PR #19 lands tiered `precedence`, the registrar arms in
  `refresh_series`, the instance-map exclusion and the per-kind hold, as **one per-source table** in
  `commitments.rs` (Quinn's ruling (4) on the close-out, task B3). This lane adds one `syllabus:` row:
  tier 1, its two keep-alive arms (§4.4), no instances (E5), and a change that lands as a card. The
  tiers below it are renumbered one down (`google:` 2, the rest 3), keeping every existing pair's
  order; no other column of another row changes *(convergence round 1, §4.4)*.
- **Precedence: registrar above syllabus above Google and ICS** for the same course (Q6). The
  registrar is authoritative; a syllabus can be last term's copy; a calendar event is the student's
  own entry and may be stale.
- **Different landing rules, on purpose.** The registrar confirms matched courses as `hard` (R24,
  ruled yes by Quinn, written as the student via `dashboard`) and applies later changes and lists them
  (cloud ruling 3); a field the student set by hand is never overwritten and gets the existing
  `commitment-check` change card instead (Quinn's ruling O1/Q2 (a)), filed from the registrar's kept
  entry at the fetch and at each `rank` (#19's close-out amendment, item 12), not by `rank`'s §5.4
  change path. A syllabus never does either: it proposes and waits (§4.5); a change to a confirmed
  syllabus note can only ever be a card through the §5.4 change path, under that path's existing rule
  for a hand-set field (§4.4, §4.5).
  *(review round 1: §4.4 now states the same.)* When the registrar covers a course, that course's
  syllabus rows of the same kind are not proposed.
- **The ask gate.** The registrar's gate (no ask before the day after its term starts) runs in
  `cli::commitment_passes`; this lane's bounded wait (§8, Q16) runs inside `emit_asks`. An ask fires
  only when both allow.
- **The sign-in.** The registrar uses the discard-everything sign-in window and never the kept
  Blackboard profile. Offer A follows the same rule: its own window, nothing kept (§15.2).
- **Term dates.** R0 found per-meeting `startDate` and `endDate` on every registrar meeting, and
  Quinn's ruling G3 uses them. This lane does not borrow them to resolve a syllabus's week numbers
  (Q37): a week number resolves only from a date the syllabus itself gives, Simple Syllabus's
  structured term start included.
- **Exams.** R0 found `EXAM` meetings beside `CLAS` meetings in the registrar's `meetingTimes` (a
  final exam is not a weekly class). How #19's parser treats them is its D3 amendment's. If it files
  them as a one-off `exam` commitment for the course, a syllabus final for that course yields to it as
  it yields to the feed (§4.6): no `syllabus-exam` card, and a notice on the course where the dates
  differ. If #19 drops them, a syllabus final is proposed as §4.6 says. T4dt pins whichever is on
  `main` (§14).
- **Section match.** A Simple Syllabus document is the student's own section (the host's "my
  courses" page lists only theirs), so its rows never need §4.4's section pick. Its structured CRN is
  used only to match on the device, never sent to a model (§15.4).

## 10. Choices, with a recommendation each

Each row is one decision in §11, under the same number. Q1-Q3 were ruled on 2026-10-06. Q4-Q18 are
revision 1's questions, renumbered (old number in brackets) and updated for the wider scope. Q19-Q28
are new in revision 2.

| Q | Choice | Options | Recommendation and why |
|---|---|---|---|
| 1 | Scope | **Ruled 2026-10-06 (Quinn):** meetings, exam and quiz dates, assignment dues, weights and scale, and office hours, brought forward from Beyond (§0) | — |
| 2 | Order | **Ruled 2026-10-06 (Quinn):** the LMS first; where none is found, offer Simple Syllabus or manual entry (file or paste; typing class times last) (§0, §3) | — |
| 3 | Office hours [15] | **Ruled 2026-10-06 (Quinn), part of ruling 1:** extracted, shown on the course, never proposed (§4.8) | — |
| 4 | What the model sees [3] | (a) Four screened parts (identity 400, meetings and office hours 1,600, dated schedule lines 7,000, grading 3,000), no instructor block, e-mails and phones removed, 12,000 in all, rules first per part; (b) the whole text; (c) rules only | **(a).** Each part is what one output field needs, so the bound and the privacy paragraph say exactly what travels. Schedule rows and grading blocks are found by rule (dates, weeks, percents, headings). (b) sends policies, instructor and student content nothing needs, for at most ~2x the cost now. (c) fails on free-text schedules |
| 5 | Where extraction runs [4] | (a) Inside `judge`, as a scoped pass with a new `--only syllabus` flag, writing only `state/syllabi/`, with cards filed by `rank`; (b) a new gated engine command; (c) the app calls the cloud | **(a).** `judge` already exits 0, is already gated, and is where judgment lives. Filing in `rank` keeps cards deterministic and ungated, and files later items as they come into range. (b) edits `entitle.rs` (contract list). (c) breaks "commands.rs computes nothing" |
| 6 | Precedence in the collapse [5] | (a) Registrar > syllabus > Google/ICS (on PR #19's tiers, E7); (b) Google/ICS > syllabus | **(a).** The student's calendar entry is often a stale copy. Whichever wins, disagreeing rows both show, contested and unpreset |
| 7 | PDF text [6] | (a) A pure-Rust PDF text crate in the engine, checked against `docs/reference/toolchain.md`, rustls-only, no `tauri`; (b) send the PDF to the cloud; (c) v1 without PDF (DOCX, HTML, TXT, paste) | **(a)**, with (c) as the fallback if no crate passes the toolchain check. (b) sends the whole file off the device. Table rows from PDF are layout-dependent; the eval (T7) measures it |
| 8 | Storage, and binary files in a plain-text vault [7] | (a) Originals in `<vault>/syllabi/` and the versioned extraction JSON in `<vault>/state/syllabi/`; (b) originals in the profile's app data, extraction JSON in the vault; (c) keep no original, only the extraction | **(a).** The student's own document sits beside their notes, can be opened, and rides their backup. JSON under `state/` is not new. It is the first PDF or DOCX in a vault, which CLAUDE.md's "markdown + YAML frontmatter" and "the vault stays plain text" need ruled on. (c) loses re-extraction when the pin or prompt changes |
| 9 | Model and host [8] | (a) Pin to a model and host the page already names, chosen by eval; (b) a stronger model at a new host | **(a).** No new processor on the page; widen only if the eval fails. Schedules are harder than meeting lines, so expect Qwen at DeepInfra rather than Granite |
| 10 | Simple Syllabus [9] | **Ruled 2026-10-18 (Quinn), after S2:** build Offer A, and widen the read beyond class times to "grade information, late work policies, test and quiz dates, assignments, etc." Its own sign-in window at the school's host (a curated field), nothing kept, shown beside Offer B only where a host is curated; read in the page, since the plain GET is refused (§15); the widened read is §16; the open parts are §17 Q29-Q43 | — |
| 11 | LMS session for step 1 [10] | (a) Only inside the wizard's sign-in, at `capture_courses`, through a new onboarding command; (b) also the kept Blackboard grades profile (`grades.rs:273`), for term-start skims in the slot; (c) the LMS commands in the console list too | **(a) now.** It is the only place a session exists (§3.1). (c) would open a campus window from the console for one button. (b) later, once the grades gate's posture is settled |
| 12 | Preset "Mine" [11] | (a) Preset a syllabus class or lab row only when it is uncontested and its section matched uniquely, and never preset any contested row, of any source; (b) never preset syllabus rows | **(a).** Finish stays one tap for the common case. A course with two disagreeing rows is never one tap away from two hard classes, one of them wrong |
| 13 | Privacy timing [12] | *(restated 2026-10-18 by Quinn's rulings 3 and 5 of that day)* The syllabus lines (§6.1) ride lane L4's honest page and its `PRIVACY_VERSION` move now; the lawyer reads the whole page before the Pilot, and L4's re-consent screen re-prompts everyone then. No separate bump | — (follows from the two rulings; Quinn confirms at signing) |
| 14 | Eval material [13] | Use the syllabi in `intake/documents/` and Quinn's own UA syllabi as eval seeds, kept out of the public repo; or synthetic seeds only | **Ruled 2026-10-19 (Quinn): real seeds, kept off the repo**: never committed, never in a fixture, test, spec or report, stored only in a non-public place outside the working tree that T7 names; synthetic seeds may sit beside them in `cloud/eval/seed/`. (Was **Ask.**) Real seeds test what students will upload, and schedules and grading tables vary far more than meeting lines; the repo is public (`VISION.md:242`), and R-C2-E12 kept the eval seed empty of real data |
| 15 | Deleting a syllabus file [14] | (a) Deleting never ends or removes anything: the course's series drop with no `ended` entry, confirmed notes, approved tasks and weights stay, pending cards stay to be decided, and no new card is filed for that course; (b) deleting ends classes through the ended path and withdraws pending cards | **(a).** Deleting a source document is not evidence that a class ended or work was cancelled. Under (b), every second desktop (no file there) and every tidy-up of an old PDF would ask "has CS 100 ended?" |
| 16 | How long the ask waits [16] | (a) Only while a file is `pending`, for at most `SYLLABUS_WAIT_DAYS` = 2 vault days from `added`; no file counts as exhausted; (b) wait for any offer not yet taken; (c) no wait | **(a).** A student who never adds a syllabus keeps today's timing, and an offline or lapsed student is asked within two days. (b) never asks a student who ignores the offer. (c) asks to type while a file is being read |
| 17 | Locking the post-add run against the slot [17] | (a) The app's scoped chain (`judge --only syllabus`, `commitments`, `rank`) takes the scheduler's `running` flag (`scheduler.rs:58`); if a slot holds it, the slot reads the course, or the chain follows the slot; (b) run regardless, in parallel | **(a).** Two `judge` or `rank` processes over one vault, on one account's caps, is a race nothing else allows. The cost is a wait of at most one slot |
| 18 | Syllabi and a second desktop [18] | (a) `syllabi/` and the extraction stay local; decided cards, notes and weights sync; a second desktop files no syllabus card and shows "Add syllabus"; (b) carry syllabus files in the account, with the relay (Launch) | **(a) now.** What the student decided already follows them, and synced cards stop any double filing. (b) puts student-held documents on the server, which needs its own ruling and privacy line |
| 19 | The stage this lane joins | *(restated 2026-10-18)* Quinn's session-5 ruling 2 made it a wave-1 lane of the friends-MVP push, and the hand-off's finish line says "L3 and L5 merged if ready, else they follow in the MVP→v1 stretch"; its slip rule ships friends with syllabus off. So: built now on `main`, not an exit condition of the friends release, behind the release guard (T12, Q39) | — (follows from the session-5 rulings; Q42 asks Quinn to confirm what "off" means) |
| 20 | How an exam lands | (a) A new synced card kind, `syllabus-exam`, sharing `event-accept`'s settlement: Approve writes a one-off hard `exam` commitment and a "Study for" task due at its start; one new arm in `approvals.rs` (contract list); (b) reuse `event-accept` itself with a kind field, no `approvals.rs` edit; (c) exams as `kind: task` cards only | **(a).** An exam happens at a time, so it must block that time (`VISION.md:36-39`, events D1's reasoning); the study task puts "start studying" into the list (`VISION.md:12`). (b) writes syllabus uids into the event ledger and reports rejections as event judgments (events D9, D12). (c) leaves the hour unblocked. Quizzes are task cards: an in-class quiz is already inside a class |
| 21 | Which items are proposed, and how many | (a) Exams, quizzes and assignment dues only; proposed 14 days ahead (21 for exams), never past, at most 8 syllabus cards a day, all charged to the 15; the rest listed on the course; (b) every item at once, at upload; (c) also readings, topics and cancellations | **(a).** Six syllabi can hold 150 dates; filed at once they would take the deck for ten days and push out everything else. Two weeks ahead is when a date starts to matter, and exams get a week longer for studying. (c) floods the deck; a cancellation changes a commitment, which is its own design |
| 22 | The duplicate rule against the feed | (a) PQ13 (b)'s due-aware rule: a title match (normalised, token or kind-and-number) on the same due day, or against an undated note, is a duplicate and files nothing; a match on another day against a feed item files nothing and shows a notice that the feed's date is used; (b) title and course only; (c) file anyway and let the student reject | **(a).** The feed is authoritative (`VISION.md:46`), so a syllabus date never competes with it as a proposal, but the disagreement is shown, never silent. (b) is the rule PQ13 rejected for losing a moved date. (c) fills the deck with doubles of what the student already has |
| 23 | A feed item that arrives after the syllabus | (a) A pending syllabus card is withdrawn as superseded; an approved syllabus task is adopted by `ingest` (its `source_uid` re-keyed to the feed's), so the feed governs it; (b) `ingest` creates its own task and the syllabus task is archived as superseded; (c) leave both | **(a).** One task, keeping the student's progress, effort and hand edits, governed by the authority from then on. (b) throws away what the student logged on the first task. (c) is a duplicate on the list. Cost: a new branch in `ingest`'s new-item path, inert for any vault without `syllabus:` tasks |
| 24 | Syllabus cards on the vault's first day | (a) Held to day 2, as mail cards are (`proposed_at` and `snooze_until` day 2); (b) rows on the first-day confirm screen, beside the classes | **(a).** The confirm screen stays about the week's shape, and the email lane's rule is reused rather than a second day-1 surface built. Cost: day 1's list shows feed items only, and exam dates show on the course, not in the deck, until day 2 |
| 25 | How weights and scale land | (a) One `kind: amend` card per course; `AMENDABLE_FIELDS` gains `grade_weights` and `grade_scale` (contract list), and, by §16.3 and Q34, `late_policy` on the same card; (b) a confirm row on the course in the console, written as the student through `write`; (c) written directly as the agent, shown with Undo | **(a).** The deck is where "Knowlu noticed" waits, judge once already speaks through amend cards, and a rejected set is remembered by its `source_uid`. (b) is a second confirm surface with its own command. (c) writes a field the student never confirmed, against `VISION.md:88-90` |
| 26 | What the weights feed now | (a) Importance in `judge` and the letter scale in the grades read model now; the weighted ring (G2) waits for the GPA phase; (b) also the weighted ring now, mapping Blackboard categories to syllabus categories by name | **(a).** Both slots exist and need no new maths. A weighted ring needs a category-mapping rule and its own honesty test ("points so far" never quietly becoming a weighted guess), which is the GPA phase's design |
| 27 | Typing class times on day 1 (Offer C) | (a) An inline row on the confirm screen, same fields and checks as the `commitment-ask` card, written as the student through `commitments --confirm` (the student asked, so it acts at once); (b) the link says "Knowlu will ask on day 3" and files the ask then | **(a)**, if `--confirm` can take a typed row (§14); else (b). Ruling 2 offers typing as a last resort, and a last resort that waits two days is not an offer |
| 28 | Build sequence under the ruled order | (a) Ship Offer B (file and paste) first, then step 1 once S1 passes, then Offer A once S2, S3 and the first-pass terms read pass (S2 done 2026-10-18; Q39); whatever is built always runs in the ruled order; (b) hold every release of the lane until step 1 exists | **(a).** Offer B needs no spike and works at every school, and it is what step 1 falls back to anyway. (b) ties the whole lane to a spike that needs Quinn at the machine |

## 11. Decisions for Quinn

Each decision has its context in §10, under the same number.

*(amendment 2026-10-18)* The session-5 hand-off records Q1-Q28 as ruled, but this file never recorded
how Q4-Q28 were ruled, and no dated row in `docs/` does. This amendment does not change their text
except Q6 (E7), Q10 (Quinn's ruling), Q13 and Q19 (restated by the session-5 rulings), Q25 (the late
rule joins the card) and Q28 (S3 joins the gate). Q43 asks Quinn to confirm the rest as recommended
when signing; until then they read as below. New decisions are §17's, Q29 onward.

**Ruled**
1. **Scope. Ruled 2026-10-06 (Quinn).** Class meetings (classes, labs), exam and quiz dates,
   assignment due dates, grading weights and scale, and office hours come forward from Beyond.
   Cloud ruling 5 and `VISION.md:88-90`, :152-153 and :227 are amended and `VISION.md:236` is closed
   (§0's text, applied by T0 at signing).
2. **Order. Ruled 2026-10-06 (Quinn).** The LMS first; where no syllabus is found there, offer
   Simple Syllabus or manual entry (upload or paste; typing class times as the last resort).
3. **Office hours. Ruled 2026-10-06 (Quinn), within ruling 1.** Extracted and shown on the course,
   never proposed (`VISION.md:48`).

**Open**
4. **Model input:** the model sees four screened parts (identity, meetings and office hours, dated
   schedule lines, grading), with no instructor block and e-mails and phones removed, 12,000
   characters in all, after rules. *Recommend yes.*
5. **Where:** extraction is a `syllabus` kind inside `judge`, with `judge --only syllabus` after an
   add; the pass writes only `state/syllabi/`, and `rank` files the cards. *Recommend yes.*
6. **Precedence:** registrar above syllabus above calendar, built on the tiers PR #19 lands
   (syllabus inserted at tier 1, as a row of its per-source table, with Google and the rest
   renumbered one down; convergence round 1). *Recommend yes.* *(E7)*
7. **PDF:** add one pure-Rust PDF text crate to the engine after a toolchain check; otherwise ship
   without PDF first. *Recommend yes.*
8. **Storage, and binary files in the vault:** originals in `<vault>/syllabi/` (the first PDF or DOCX
   files in a vault, never synced, mirrored by the student's own backup), the versioned extraction
   JSON in `<vault>/state/syllabi/`. Does a plain-text vault admit the student's own binary
   originals? *Recommend yes.*
9. **Pin:** pin the kind to a model and host the page already names, chosen by eval. *Recommend yes.*
10. **Simple Syllabus. Ruled 2026-10-18 (Quinn), after S2.** Build Offer A, and widen the read beyond
    class times: "It has grade information, late work policies, test and quiz dates, assignments,
    etc." The read happens in the page (§15); the widened read is §16; the choices it opens are §17
    (Q29-Q43). Whether a student's own on-device read counts as "automated access" under the vendor's
    terms is for counsel before the Pilot (Quinn's "lawyer later"); §15.6 lists the clauses to check.
11. **LMS session:** step 1 runs only inside the wizard's sign-in, right after `capture_courses`; no
    LMS command in the console and no use of the kept grades profile. *Recommend yes.*
12. **Preset:** preset "Mine" only on an uncontested row with a unique section match, and never on a
    contested row from any source. *Recommend yes.*
13. **Privacy timing. Restated 2026-10-18** by Quinn's rulings 3 ("lawyer later") and 5 (the honest
    page now): the §6.1 lines ride lane L4's page and its `PRIVACY_VERSION` move; the lawyer reads the
    whole page before the Pilot and L4's re-consent screen re-prompts everyone then. *Confirm at
    signing.*
14. **Eval material:** may the syllabi in `intake/documents/` and your own UA syllabi be used as eval
    seeds, kept out of the public repo? *Ask.*
15. **Deleting a syllabus file:** may deleting a syllabus file ever end a class, withdraw a card or
    remove a task or weight? *Recommend no.* **Ruled 2026-10-19 (Quinn): no.** Deleting a syllabus
    file drops its series only; it never ends a class, withdraws a card or removes a task or weight.
16. **Ask wait:** the typed-times ask waits only for a `pending` file, for at most 2 vault days.
    *Recommend yes.*
17. **Locking:** the post-add chain takes the scheduler's `running` flag and never runs beside a slot.
    *Recommend yes.*
18. **Second desktop:** syllabus files and extractions stay on the desktop that added them; decided
    cards, notes and weights sync; revisit with the relay at Launch. *Recommend yes.*
19. **Stage. Restated 2026-10-18** by the session-5 rulings: a wave-1 lane of the friends-MVP push,
    merged if ready and otherwise following in the MVP→v1 stretch; not an exit condition of the
    friends release; behind the release guard. What "syllabus off" means in a release is Q42.
    *Confirm at signing.*
20. **Exams:** an exam is a new synced `syllabus-exam` card whose Approve writes a hard one-off `exam`
    commitment and a "Study for" task (one contract-list arm in `approvals.rs`). *Recommend yes.*
21. **Which items, how many:** exams, quizzes and assignment dues only; proposed 14 days ahead (21 for
    exams); at most 8 syllabus cards a day, inside the 15. *Recommend yes.*
22. **Duplicates:** PQ13 (b)'s due-aware rule against existing notes; a feed item with another date
    wins and the difference is shown as a notice, not a card. *Recommend yes.*
23. **Feed arrives later:** pending syllabus cards withdraw; an approved syllabus task is adopted by
    `ingest` and governed by the feed from then on. *Recommend yes.*
24. **Day 1:** syllabus cards are held to day 2, as mail cards are. *Recommend yes.*
25. **Weights and scale:** one `kind: amend` card per course, with `grade_weights` and `grade_scale`
    added to the amendable fields (contract list); by Q34, `late_policy` rides the same card.
    *Recommend yes.*
26. **What weights feed:** importance and the letter scale now; the weighted ring waits for the GPA
    phase. *Recommend yes.*
27. **Typing on day 1:** Offer C is an inline typed row on the confirm screen, written through
    `commitments --confirm`, if that can take a typed row; else the day-3 ask. *Recommend yes.*
28. **Build sequence:** Offer B first, step 1 after S1, Offer A after S2 (done), S3 and the
    first-pass terms read (Q39); whatever exists runs in the ruled order. *Recommend yes.*

## 12. Risks

| Risk | Effect | Mitigation |
|---|---|---|
| Wrong section matched | Confidently wrong class times or dates | Never guess between sections (§4.4); contested rows unpreset (Q12); items with an unmatched section skipped; the student confirms every row and card |
| Calendar and syllabus disagree | Two hard classes confirmed in one tap | Contested rows are never preset (§4.4, Q12) |
| Stale syllabus (last term's copy) | Old times and dates proposed | `first`/`until`, `term` and item dates checked against the course's term; a mismatch is shown ("syllabus says Fall 2025"); dates over 9 months out need a date |
| Extraction error (am/pm, a lab read as a lecture, a wrong year, a quiz read as an exam) | A bad proposal | Strict schema, per-part confidence floor, eval seeds (T7); proposals only, never auto-confirmed |
| Week numbers resolved wrongly | Every week-dated item off by a week | Resolved only from the syllabus's own week-1 date (§4.6); otherwise "needs a date" |
| A flood of cards | The deck is taken over; other proposals wait | Two-week horizon, 8 a day, inside the 15 (Q21); overflow snoozed, never dropped |
| A duplicate missed ("HW3" against "Problem Set 3") | Two tasks for one assignment | Three title tests and the due day (§4.6); feed adoption (Q23); the student rejects a double, and it stays rejected |
| A real item read as a duplicate | A missed due date | A duplicate needs the same course and the same due day, or an undated note; the item is listed as "Blackboard already has it" with the note it matched |
| Syllabus and feed disagree on a date | The student studies to the wrong day | The feed wins and the course shows both dates (Q22) |
| Wrong weights | Importance grounded in wrong numbers; wrong letters | Whole-or-nothing checks (§4.7); an amend card the student confirms; rejected sets never re-proposed |
| A corrected file reorders sections | End cards for classes still meeting | The key carries section and kind, never position (§4.4) |
| A student tidies up an old PDF | "Has CS 100 ended?" | Deleting never ends or removes anything (Q15) |
| A class split across times in a calendar (MW at 10, F at 11 as two series) against one syllabus row | Three contested rows instead of one | The student picks; nothing is preset. Revisit if the eval or the pilot shows it often |
| Simple Syllabus terms forbid automated access, or the school's contract does | Offer A cannot ship | *(amended 2026-10-18)* Code may be built (Q10 ruled), but no release carries a host before the first-pass terms read (Q39); Offer B covers the school anyway |
| Simple Syllabus is a single-page app whose HTML shape changes | Offer A breaks silently | *(amended 2026-10-18)* Read the host's own document JSON, not the rendered page and not the PDF (§15.2); a JSON shape the engine does not know is a named failure (§8), never an empty row |
| MFA or SSO at the syllabus host | No background re-fetch | Step 1 and Offer A run only with the student present (the wizard; Connect) |
| Blackboard contents unreachable by a student cookie | Step 1 empty at Blackboard schools | Spike S1 before T9; step 1 records "would not show content" and the offer follows |
| Canvas paths unproven (`lms_link.rs:477-480` written from docs, never run) | Step 1 empty at Canvas schools | The same named outcome; a Canvas proof when a Canvas student exists |
| The wizard slows | The student waits at the course list | Per-course timeout, at most 3 files a course, size bound (§4.1); "not tried" is a named outcome |
| Staged bytes left in app data | Student documents linger outside the vault | Deleted at Finish; swept after an hour at the next launch (§4.1) |
| PDF crate cost (size, build) | Breaks "base app under 50 MB" or the toolchain | Toolchain check in T3; fallback Q7 (c) |
| Student or third-party data inside an uploaded file | Personal text reaches a model | Four bounded parts, no instructor block, e-mails and phones removed (§4.2); the page says "lines", not "nothing personal" |
| A long schedule overflows its part | Later dates never read | Named clip outcome (§8); the student can paste the rest; the eval checks 7,000 against real schedules |
| *(review round 1)* The lawyer's read before the Pilot widens | The Pilot waits | The syllabus lines are already on L4's page (§6.1, Q13 restated), so the lawyer reads one page, once |
| *(review round 1)* A release before L4's page names syllabi | Syllabus text reaches a model, or a suggestion syncs, under a page that does not name it | Q42's merge gate (no app surface on `main` before the page carries §6.1 items 2-5), Q39 for a host, and T12's release-guard line |
| The extraction file format drifts | Existing vaults misread | `"version": 1` from the first release; unknown version means re-read, never guess (§5) |
| `ingest`'s adoption branch misfires | A feed item re-keys the wrong task | Same course, same title rule, an active `syllabus:` task only; tests in T4t; inert without `syllabus:` tasks, so the frozen references cannot move |
| This lane builds on the registrar's seam *(E8; corrected in review round 1)* | A syllabus arm placed wrongly beside a registrar arm ends a class or loses a card; or a "fix" to the registrar's row drops its kept-entry card, against O1 | T4t pins the `syllabus:` row only; #19's own tests pin the registrar's row and its O1 card and run unedited after T4a (§4.4); T4a edits no other row's columns, its one outside change being the tier renumbering that keeps every existing pair's order (and a #19 test pinning a tier's number stops the task for Q48); T4a, the precedence change included, runs as `integrator` after PR #19 is on `main` |
| *(convergence round 1)* `syllabus:` given Google's tier | Google wins every agreeing collapse on the calendar string, reversing Q6 | `syllabus:` holds tier 1 alone and Google moves to 2 (§4.4); T4t pins an agreeing syllabus and Google pair collapsing to the syllabus row, and the order of every source pair |
| *(convergence round 1)* A syllabus card withdrawn outside `approvals.rs` | A half-moved card holds its key, so the moved date is never proposed again | Withdrawal is `approvals::withdraw_syllabus` (T4e, contract list) with the finishing arm for a half-moved card; the filer only gives the reason and never moves a card (§4.6); T4dt pins each case |
| *(convergence round 2)* A dropped syllabus key ended with no last date | With no instances, #19's ended path finds no end and no successor; the removed and the added lab both block time all term | The ended arm stamps `last_instance` (the last meeting day before the run) and `until` (the series', else that day), so #19's path files the end or swap card unedited (§4.4); T4t pins the dates |
| *(convergence round 3)* A copy that cannot read meetings replaces one that could (a Simple Syllabus copy over an LMS copy, S2's 6 of 8) | Every class and lab of a still-meeting course gets an end card on the next `rank` | The ended arm and the fresh pass run only on a usable meetings read: status `read`, the meetings part by rules or at or above the floor, not clipped, at least one meeting (§4.4); otherwise the series are dropped with no `ended` entry (Q50 (a)); T4t pins each case. *(convergence round 4)* Residual under (a): a meeting that a later usable read leaves out is never ended, because the drop left no series to age out (§4.4); Q50 states it for Quinn to weigh against (b) and (c), and T4t pins it |
| *(2026-10-18)* A script evaluated on the wrong page | Student data from another site, or a session, reaches Knowlu | §15.2: compiled-in scripts by name only, run only while the top-level page is on the curated host (checked by the app and again in the page), results by the script-completion value, no capability grant, no IPC; T10t pins each refusal |
| *(2026-10-18)* The in-page call is refused as the plain GET was | Offer A reads nothing | S3 before T10b; the fallback order is fixed (Q29); a refusal is a named outcome and Offer B stays beside it |
| *(2026-10-18)* Inline styles read as grading | Weights proposed from `width: 50%` | The engine renders components to text before any screen runs, dropping every attribute (§15.3); a T15 test pins it on S2's inflated-percent shape |
| *(2026-10-18)* A document matched to the wrong course | One course's dates proposed on another | Subject and number against the course note, term checked, a unique match or a pick (§15.4); never a guess |
| *(review round 1)* A pick with nothing left to pick from | Two matching courses, the window gone, the student must sign in again and meets the same two-way match | The engine holds the rendering in `state/syllabi/held/` and the pick settles it (§4.1, Q47); T15t pins it |
| *(review round 1)* Simple Syllabus overwrites the student's own upload | The student's chosen copy is lost, against Q33 | The extraction file records `source` from `version: 1` (§5); the replacement rule reads it and holds the vendor's copy as `kept-own` (§4.1); T15t pins each pair |
| *(review round 1)* A policy quote leaves the device on a card | The page says quotes stay on the computer, and a synced card carries one | No card carries a quote; the console shows it from the local file (§4.6); T16t pins that the amend card's bytes hold no quote |
| *(2026-10-18)* A late-work rule misread | A past-due task shown as "still accepted" when it is not | Whole-or-nothing validity and the verbatim quote shown beside the rule (§16.3); an amend card the student confirms; `rank` does not use the rule (Q34) |
| *(2026-10-18)* A model's "quote" that the syllabus never said | A policy shown in the syllabus's voice that it does not hold | The validator keeps a quote only if it is a substring of the excerpt sent (§16.4) |
| *(2026-10-18)* The vendor's terms or the school's contract forbid the read | Offer A cannot ship, or must be withdrawn | First-pass terms read before a release carries a host (Q39); counsel before the Pilot; the host is curated data, so withdrawing it is a 0.1.x row change with no code |
| *(2026-10-18)* Mid-term onboarding (every friends tester joins in October) | Half a term's dates are past; the rest are mostly in the feed already | Past items are never proposed; the duplicate rule and Q23's adoption keep the feed's items single; the course lists the rest with reasons (§16.5) |

## 13. Rough task breakdown

Spikes run first and gate only their own step (Q28). Offer B's path (T1-T8b, T11a-b, T11d-e, T12-T13,
T17) needs neither spike. Every implementation task follows a failing test, and everything a Sonnet agent
changed goes through `reviewer` (T14) before a push. Where two tasks share a file they run in the order
shown, never in parallel: `commitments.rs` (T4a, T4b, T4c); `engine/src/cli.rs` (T2c, T4a, T4d);
`engine/src/main.rs` (T2c, T5); `app/src/main.rs` (T8a, T9, T10); `app/static/console.js` (T11a to
T11e). The plan sizes each task at about 80 lines or less and may split further.
*(amendment 2026-10-18)* Added orders: `engine/src/syllabus.rs` (T2a, T2b, T2c, T15, T17, T16);
`engine/src/main.rs` (T2c, T5, T15);
`engine/src/cli.rs` (T2c, T4a, T4d, T15); `engine/src/syllabus_cards.rs` (T4d, T16);
`app/src/syllabus_link.rs` (T10a, T10b); `app/static/console.js` (T11a to T11f).
*(review round 1)* `engine/src/surface.rs` (T17, then T16). *(convergence round 1)* The withdrawal
(§4.6) runs T4d's `syllabus_cards.rs` (the filer and `withdrawal_reason`), then T4e (`approvals.rs`,
which calls the reason), then T4d's `rank` hook in `cli.rs`, which calls `withdraw_syllabus` before
the filer; T4dt's withdrawal cases pass only after all three. Lane-wide gates from the session-5
conflict map and the close-out plan (§5.3, §5.4):
- **`console.js` and the other `app/static` files:** the session-5 `console-ui` queue is
  **L4 → L7 → #19 (B5) → L1 R11 → L5 → the redesign**. T11a starts only once L1 R11 has merged into
  `console.js` (or the controller records in the ledger that L1 R11 took a later slot, which the
  close-out plan allows only ahead of L5, never after it). One `console-ui` lane edits these files at
  a time; whoever merges second rebases its page work and re-runs `static_assets` and
  `wizard-check.py`.
- **`commitments.rs`:** T4a and everything after it start only once PR #19 is on `main`.
- **`surface.rs`:** after L7. **`approvals.rs`:** this lane's alone. **T6a's migration:** stamped
  after L8's and before L1's.
- **Q42's merge gate (as recommended):** no task that gives a student a way to add or find a
  syllabus (T8a, T8b's hand-over, T9, T10c, T11a-T11f) merges to `main` until L4's page carrying §6.1
  items 2-5 is on `main`. Engine and cloud tasks may merge before it: with no app surface, nothing
  creates a `state/syllabi/` file, so nothing is read, sent or filed.
- **`app/src/main.rs` (T8a, T9, T10c):** L6 owns it while L6 runs. Each of these tasks' handler lines
  is handed to L6's owner, or the task's `main.rs` part waits until L6 merges; the task's agent never
  edits `main.rs` while L6 is open. The rest of each task (its module and tests) runs as its row says.
- **`engine/src/main.rs` and `engine/src/cli.rs` (T2c, T4a, T4d, T5, T15):** L6's runtime removal
  touches both. The close-out plan's whoever-is-second rule applies: if L6 merges first, these tasks
  rebase over it; if this lane's commits land first, L6 rebases. Never two branches resolving the same
  hunk at once; the controller records which went first.

| # | Task | Files (main ones) | Agent | Why that agent |
|---|---|---|---|---|
| S1 | Spike: can a student cookie session list a UA Blackboard course's contents and download a syllabus file? Record the handlers and any Simple Syllabus LTI link | none (a note in `docs/notes/`) | main session, with Quinn at the machine or the proof harness under the standing account permission | A live session is the controller's job, never a subagent's |
| S2 | Spike: open one real UA section at `ua.simplesyllabus.com`; record whether meeting times, a dated schedule and weights show, the page shape and the PDF link; read the full terms | none (a note) | main session with Quinn; counsel for the terms | **Done 2026-10-18** (ledger row; facts in §3 and §15). The full terms read moved to T18 and Q39 |
| S3 | *(2026-10-18)* Spike: the in-page read, §15.7's questions, on a dev build in Quinn's block. Shapes, counts and header **names** only; no document code, course, section, CRN, name or cookie on disk, in the transcript or a commit (S2's incident) | none (a row in the session ledger, a note in `docs/notes/`) | main session with Quinn | A live session is the controller's, never a subagent's; a diagnostic port in a dev build is the spike's tool, never the product's |
| T0 | After signing: apply §0's amendment to cloud ruling 5; edit `VISION.md:88-90`, :152-153, :227 and close :236; add the lane to `HANDOFF.md` §3 at the stage Q19 picks | `docs/specs/2026-09-09-knowlu-cloud-design.md`, `VISION.md`, `HANDOFF.md` | `docs-keeper` | Docs only, after Quinn signs |
| T1 | Failing tests for T2: text per format, each table row one line with its cells joined as §4.2 says; the four parts (each keeps what §4.2 says, drops the instructor block, removes e-mails and phones from office-hours lines); each part's bound and the 12,000 total; a clipped schedule keeps its earliest lines; the rules per part; "no part left, no call"; `syllabus-add`'s writes (copy, `state/syllabi/<slug>.json` with `version`, SHA-256, *(review round 1)* `source`, `added`, `status`, per-part state, *(convergence round 3)* a part the screen kept nothing of recorded as empty), the paste source; one copy per course whatever its extension (a `.docx` replacing a `.pdf` deletes the `.pdf`); an upload or paste replaces any copy; exit 2 with nothing written on bad input. Synthetic inputs only, under **`engine/tests/syllabus_data/`**, never `engine/tests/fixtures/**`. DOCX and PDF built in-test where possible; any committed binary gets a `-text` line in `.gitattributes` and passes `scripts/ci/eol-check.ps1` | `engine/tests/syllabus.rs`, `engine/tests/syllabus_data/`, `.gitattributes` | `test-writer` | TDD first; fully specified; off the contract list by path |
| T2a | Text extraction: DOCX (`zip`), HTML, TXT, with table rows | `engine/src/syllabus.rs` | `implementer` | Specified, checked by T1 |
| T2b | The four-part screen, the bounds, the clip and the rules per part | `engine/src/syllabus.rs` (after T2a) | `implementer` | Specified, checked by T1 |
| T2c | The `syllabus-add` command: both forms, the copy, the state JSON through `ledger::dumps_value` | `engine/src/cli.rs`, `engine/src/main.rs`, `engine/src/syllabus.rs` | `implementer` | Specified, checked by T1 |
| T3 | PDF crate: choose, add, prove the toolchain, the size and table-row order; `dependency_boundary.rs` stays green | `engine/Cargo.toml`, `Cargo.lock`, a `docs/reference/toolchain.md` note | `implementer`, then `reviewer` | A dependency change with known traps |
| T4t | Failing tests for T4a-c: the key (reordered sections give the same keys; a time correction gives one change card, no end card); deletion gives no `ended` entry and no card; a removed lab ends on the same run, *(convergence round 2)* with its `Ended` entry and card pinned to dates (§4.4, "What the arm stamps"): a Friday lab dropped on a run of Wednesday 2026-10-14 stamps `last_instance` 2026-10-09 and the series' `until`, and its end card proposes `until: 2026-10-09`; with the series' `until` null the entry's `until` is 2026-10-09 too, and the end card files for a hard and for a soft note alike; a dropped series past its own `until` files no card; a series whose `first` is after the run stamps the day before that `first` (Q49); a corrected syllabus that moves the lab to Thursday under a new key whose `first` is on or after 2026-10-09 files one swap card (`source_uid` to the new key, the new `meets`) and no end card, and one whose `first` is earlier or null files the end card and the new key's check card, never neither; *(convergence round 3, §4.4's usable meetings read)* a confirmed class and lab (keyed `syllabus:<slug>:-:*`) whose copy is replaced by one with items and grading read and a meetings part below the floor give no `ended` entry and no end card on the next `rank`, for a hard and a soft note alike, and the series file holds no `syllabus:<slug>` series; the same holds when the new meetings part is empty (never sent), clipped, read with no meeting, or the new copy is still `pending` or `failed:<code>`; the confirmed notes are byte-identical afterwards; *(convergence round 4)* a later copy with a usable read that returns the class at a moved time and no lab then files the class's change card (its key is fresh again) and, for the lab, no `ended` entry, no end card and no swap card, with the lab's note byte-identical, and the same when that usable read is the replacing copy's own, first seen `pending` by a `rank` and read by a later `judge`: Q50 (a)'s second cost (§4.4), pinned so that a ruling of (b) or (c) changes this case on purpose (under either, the lab ends on that run); the ended arm's own firing on a usable read is pinned by the removed-lab cases above, where the series file still holds the lab's series; a second desktop with no file files nothing; `create_confirmed_as`'s body line; *(review round 1)* the `syllabus:` row only: a changed syllabus series files a `commitment-check` change card and applies nothing, and a hand-set field follows the change path's existing rule; T4t pins **no** registrar behaviour (no "applied, never carded" pin, no registrar landing), because #19's tests own that row and must pass unedited beside T4t; the collapse rank on #19's tiers (registrar 0, syllabus 1, google 2, the rest 3; every existing pair keeps its order); agreeing rows collapse to the syllabus row; *(convergence round 1)* pinned as orders between sources, never as tier numbers: an agreeing syllabus and `google:` pair collapses to the syllabus row (the case a shared tier would fail), an agreeing registrar and syllabus pair to the registrar's, one `source_uid` under a syllabus and a Google calendar resolves to the syllabus calendar in `by_key`, and a syllabus series and a Google series aged out on one run are taken in precedence order; *(E9)* syllabus series block time by `meets` inside the horizon; a registrar-held course gets no syllabus row of that kind; an ask waits while either gate holds; `contested` on disagreeing rows of any source; section match; the ask gate's cases (no file, pending under 2 days, pending at 2 days, failed, meetings below the floor); `sync::is_note_path` refuses `syllabi/` and `state/syllabi/` paths (a new test file calling the public function; sync's own tests untouched) | `engine/tests/syllabus_commitments.rs` | `test-writer` | TDD first |
| T4a | *(E9)* The key; the `syllabus:` row of PR #19's per-source table (the watched prefix, the change landing as a card; *(review round 1)* no other column of another row edited, #19's tests green and unedited); *(convergence round 1, moved from T4b)* `syllabus:` at tier 1 of `precedence`, with `google:` renumbered to 2 and the default row to 3, every existing pair's order kept (§4.4), and the stop rule if a #19 test pins a tier's number (Q48); the syllabus series fed to `refresh_series` as fresh each run, with its `first` and `until` from the extraction, *(convergence round 3)* only while the extraction holds a usable meetings read (§4.4's four conditions, read from §5's per-part state); the two syllabus arms beside the registrar's, the ended arm firing only on a usable read and the deletion arm also taking a present copy with none (Q50 (a)), *(convergence round 2)* the ended arm stamping `last_instance` (the last meeting day before the run, clamped) and `until` (the series', else that day) as §4.4 says, #19's `detect_changes` unedited; syllabus series out of `instances_map`; `create_confirmed_as`'s body line. Starts only after PR #19 is on `main` | `engine/src/commitments.rs`, `engine/src/cli.rs` (the source read beside the Google and ICS normalisation) | `integrator` (Opus, high) | Changes change detection on confirmed notes beside the registrar's arms, and `precedence`, which drives `by_key`, the collapse, the aged-out order and the swap pick: a silent error ends a class that is still meeting, loses every change card, or lets Google win over the syllabus |
| T4b | *(E9; convergence round 1: precedence moved to T4a)* Section match; `contested` on `Proposal` and in `--json`. Touches no tier and no table row | `engine/src/commitments.rs` (after T4a) | `implementer` | Specified, checked by T4t; neither piece changes which source wins |
| T4c | The bounded ask gate (`SYLLABUS_WAIT_DAYS`) in `emit_asks`; each course carries its outcome for the read model | `engine/src/commitments.rs` (after T4b) | `implementer` | Specified, checked by T4t |
| T4dt | Failing tests for T4d: date and week resolution (no `week1_start` means needs-a-date; over 9 months means needs-a-date); the in-class time from a confirmed class note; the key and its `@date` suffix; the 14/21-day horizon, never past, 8 a day, charged to the 15 and snoozed over it; the day-1 hold; each duplicate case of §4.6 (equal title, token, kind-and-number; same day; undated note; feed on another day gives a notice and no card; a mail card on another day files); a key in `existing_source_uids` is never refiled; a moved date supersedes a pending card and files an amend card for an approved task; a dropped item changes nothing; a pending card withdraws once a feed note matches; *(convergence round 1, the withdrawal of §4.6, through `approvals::withdraw_syllabus` end to end in `rank`)* a moved date archives the pending task card as `superseded` and files the fresh card on the same run under the same key (an archived `superseded` card does not block it), while an archived `rejected` card with that key still blocks for good; a snoozed card withdraws as a pending one does; a `syllabus-exam` card withdraws on a moved date and on a feed or `coursework` match; a pending grading amend card whose hash is no longer the extraction's is withdrawn and the new set's card files; a due amend card is keyed by its new day and a second move withdraws it; the withdrawn pending cards leave the `Approvals: N pending` count and the run line names each; a card stamped `superseded` but left in `approvals/` (the move failed) is archived on the next run and its key does not block refiling; a card with no `syllabus:` key, and every commitment card, is never touched; `syllabus_cards.rs` writes no frontmatter of an existing card; grading validity and the grading `source_uid`; a vault with no `state/syllabi/` gives byte-identical `rank` output over `vault-full`; *(2026-10-18)* a `coursework` item on another day files nothing and shows the notice, as a feed item does (Q45); the 8 a day take items due within 7 days, then amend cards, then later items (Q41); on day 2, syllabus, event and held mail cards together charge exactly 15 and snooze the rest; a syllabus final yields to a registrar `exam` commitment for the course if #19 files one (§9) | `engine/tests/syllabus_cards.rs` | `test-writer` | TDD first; the golden check reads the frozen references, never regenerates them |
| T4d | The filer: items and grading from the extraction into `kind: task`, `syllabus-exam` and `kind: amend` cards as `agent:knowlu.syllabus`; *(convergence round 1)* the pure `withdrawal_reason` and the filer's "superseded is not a decision" key check (§4.6); the course listing for items not filed; wired into `rank`, with `withdraw_syllabus` called before the filer once T4e lands | `engine/src/syllabus_cards.rs` (new), `engine/src/cli.rs` (the `rank` hook, after T4a and T4e) | `implementer`, then `reviewer` | Specified and checked by T4dt; writes only new card notes through `write` and never stamps, moves or deletes an existing card (that is T4e's); the `cli.rs` hook is a controller hand-off |
| T4e | Contract work, test first: `approvals.rs` gains the `syllabus-exam` arm (sharing `settle_event_accept`, with an `exam` commitment and a "Study for" task), and `AMENDABLE_FIELDS` gains `grade_weights`, `grade_scale` and *(2026-10-18)* `late_policy` with flow-mapping validation in `apply_amendment` (`late_policy`'s closed keys and ranges, §16.3); *(convergence round 1)* `withdraw_syllabus` (pending or snoozed `kind: task` and `kind: amend` cards with a `syllabus:` `source_uid`, and `syllabus-exam` cards; the reason from `syllabus_cards::withdrawal_reason`; stamp, archive, `transition failed:` warnings and the `pending` count, as `withdraw_asks` does) and `process_approvals`'s finishing arm for the same three shapes stamped `superseded`; its own unit tests first, beside `withdraw_asks`'s, with T4dt's withdrawal cases as the end-to-end check; `eventaccept.rs` gains the kind and the study task | `engine/src/approvals.rs`, `engine/src/eventaccept.rs`, their tests (after T4d's `syllabus_cards.rs`) | `contract-engineer` (xhigh), then `contract-reviewer` | `approvals.rs` is on the contract list; card transitions and their failure handling live there and nowhere else |
| T4ft | Failing tests for T4f: a feed item matching an active `syllabus:` task re-keys it and creates nothing; a different due is applied and listed, a hand-set due gets an amend card; no match creates as today; no `syllabus:` task leaves every frozen reference byte-identical | `engine/tests/syllabus_ingest.rs` | `test-writer` | TDD first |
| T4f | `ingest`'s adoption branch in the new-item path (`sync_tasks`, `ingest.rs:613`) | `engine/src/ingest.rs` | `integrator` (Opus, high) | A shared single-owner file on the path the frozen references run through; a misfire re-keys the wrong task |
| T4g | `judge::Heuristics::load` renders `grade_weights`; the grades read model's `letter` uses `grade_scale`; tests first in the same task, the surface references untouched | `engine/src/judge.rs`, the grades read-model module, their tests | `implementer` | Specified; both slots exist |
| T5t | Failing tests: the syllabus pass in a slot `judge` (after enrichment, at most 8); `judge --only syllabus` sends nothing else; the request carries the course note's id, the date and only the unsettled parts; per-part status; `pending` becomes `read` or `failed:<code>`; no network, no entitlement, the cap and *(review round 1)* a missing route (404) or an unknown-kind answer each leave `pending`, record their own named outcome (§8) and exit 0; the pass writes no note; all against a `127.0.0.1:0` server | `engine/tests/judge_syllabus.rs` | `test-writer` | TDD first |
| T5 | The pass and the flag | `engine/src/enrich.rs`, `engine/src/cloudmodel.rs`, `engine/src/main.rs` (after T2c) | `implementer` | Follows the existing kinds' pattern; no contract-list file |
| T6a | `Kind` gains `syllabus`; `DAILY_CAP` 20 and `judge_caps_test.ts`; a migration widening the kind check constraints (`judgment_service.sql:39,70,103,113,155`) and a models row | `cloud/supabase/functions/_shared/judge_caps*.ts`, `judge_validate.ts`, a new migration | `cloud-engineer` | `cloud/` is its lane; the staging push is the controller's |
| T6b | Prompt, the §4.3 schema, the validator (per-part confidence, the closed kind lists), `MAX_SYLLABUS_CHARS` and the part bounds with their test, the `/judge-syllabus` route; *(2026-10-18)* §16.2's `policies` block and `confidence.policies`, the quote-is-a-substring check, the policies bound and the total Q38 picks | `cloud/supabase/functions/_shared/judge_prompts*.ts`, the function | `cloud-engineer` | Same lane |
| T7 | Eval seeds and a pass bar per part (meetings, items with dates and kinds, weights, scale, office hours, *(2026-10-18)* the late rule and the three quotes; seeds in Simple Syllabus's component shape as well as files); the pin chosen by eval (Q9); a check that 7,000 characters holds real schedules | `cloud/eval/seed/` (synthetic, or consented and not public if real, Q14) | `cloud-engineer` | Same lane |
| T8a | `app/src/syllabus.rs`: `syllabus_add` (file and paste), `syllabus_status`, *(review round 1)* `syllabus_settle_held` (runs the held form, §4.1), and the scoped chain under the scheduler's `running` flag (Q17); added to the console list `main.rs:179`; recount the lists | `app/src/syllabus.rs` (new), `app/tests/syllabus.rs`; `app/src/main.rs` by hand-off | `implementer` for the module and tests; *(review round 1)* the `main.rs` handler lines go to L6's owner while L6 runs, else `implementer` after L6 merges | Thin wiring, checked by tests; `main.rs` is L6's while L6 runs |
| T8b | `pick_file`'s optional extension list; at Finish, `create_vault_in` hands staged files and outcomes to `syllabus-add` and deletes the staging folder; the one-hour sweep | `app/src/onboarding.rs`, its tests | `implementer` | Specified; disjoint from T8a |
| T9 | Step 1, after S1: `capture_syllabi` for every course in `lms_link.rs`, writing to staging, in list `main.rs:107`; tests against a local server | `app/src/lms_link.rs`, its tests; `app/src/main.rs` (after T8a) by hand-off | `implementer` for `lms_link.rs`; *(review round 1)* the `main.rs` line goes to L6's owner while L6 runs, else `implementer` after L6 merges | Specified by S1's note; `main.rs` is L6's while L6 runs |
| T10 | *(replaced 2026-10-18 by T10t, T10a, T10b and T10c below)* | | | |
| T10t | Failing tests for T10a-b, against a fake `Page` (the mail runner's seam) and a `127.0.0.1:0` server: a script runs only by a compiled-in name; it is refused on any other host, a host with a port or user part, and after a cross-origin navigation; a new-window request off the curated host is denied and one on it becomes a same-window load (Q30); the staged file is deleted after `syllabus-add` and swept after an hour; every refusal is a fixed word with no page text; a school with no curated host shows no Connect; nothing is written to a log beyond counts | `app/tests/syllabus_link.rs` | `test-writer` | TDD first; off the contract list |
| T10a | The curated data: `Curated` gains `syllabus_host: Option<&str>` and `syllabus_kind: Option<&str>` (`simple-syllabus`); every row `None` until Q39 clears a host; the host checks and the script list as pure functions | `app/src/scaffold.rs`, `app/src/syllabus_link.rs` (new, the pure half) | `implementer` | Specified, checked by T10t; rule 1: the host is data |
| T10b | The window and the read (§15.2): label `syllabus-signin`, a throwaway data folder, no capability grant, the compiled-in scripts under `app/assets/syllabus/`, the double host check, results by the script-completion value, staging per document, `syllabus-add --source simple-syllabus` per document, the wipe. After S3 and T10a | `app/src/syllabus_link.rs` (the window half), `app/assets/syllabus/*.js` | `integrator` (Opus, high), then `reviewer` | Script evaluation in a third party's signed-in page: a silent error leaks student data or a session (CLAUDE.md's "Opus" test) |
| T10c | The console commands (`syllabus_connect`, `syllabus_connect_status`) in the console list; recount by script | `app/src/main.rs` | L6's owner by hand-off, or `implementer` after L6 merges | `main.rs` is L6's while L6 runs (session-5 conflict map) |
| T15t | *(2026-10-18)* Failing tests for T15, invented documents under `engine/tests/syllabus_data/` in S2's shape (attributes, term dates, `content`, `instructor`, `material` components with HTML): the rendering drops every attribute (an inline `width: 50%` never reaches the grading screen), drops `material`, keeps only office-hours lines of `instructor` with contacts removed; the course match (unique, none, two); the term check; the week-1 anchor (Q37); a linked PDF is an outcome, never fetched; an unknown JSON shape is a named failure writing nothing; the CRN never appears in an excerpt; *(review round 1, Q47)* two matches write a held rendering and its JSON naming both slugs, touch nothing in `syllabi/` and file nothing; the held form with a candidate slug moves it to `syllabi/<slug>.html` and runs the screen; with a non-candidate slug, or an unknown id, it exits 2 writing nothing; a second Connect of the same document reuses the held id; a held file older than 30 days is deleted by the next run; `judge` and `rank` never read a held file; *(Q33)* `simple-syllabus` over an `lms` copy replaces it, over an `upload` or `paste` copy holds a `kept-own` rendering and leaves the student's copy and extraction byte-identical, and the held form then replaces it; an upload over a `simple-syllabus` copy replaces it | `engine/tests/syllabus_simple.rs`, `engine/tests/syllabus_data/` | `test-writer` | TDD first; synthetic data only |
| T15 | The `simple-syllabus` source in `syllabus-add`: parse, render to `syllabi/<slug>.html`, match, hand to the same screen; *(review round 1)* the held renderings, the held form, the replacement rule by `source` (§4.1) | `engine/src/syllabus.rs` (after T2c), `engine/src/cli.rs` (after T4d), `engine/src/main.rs` (the held form's flags, after T5) | `implementer` | Specified, checked by T15t |
| T16t | *(2026-10-18)* Failing tests for T16: the policies part (headings, bound, clip); `late_policy` validity (closed keys, ranges; invalid shown, not proposed); the amend card carries it with weights and scale and its hash moves when it changes; *(review round 1)* no card's bytes (task, exam or amend) contain any policy quote, and the amend card's context line is the fixed words of §4.6; quotes kept only as substrings; the course listing shows policies and quotes; a vault with no `state/syllabi/` leaves every frozen and surface reference byte-identical | `engine/tests/syllabus_policies.rs` | `test-writer` | TDD first |
| T16 | The policies part in the screen; `late_policy` and the quotes into the filer and the extraction file; *(review round 1)* the `policies` field of T17's course entry, and the past-due row's late note (§16.3) | `engine/src/syllabus.rs`, `engine/src/syllabus_cards.rs`, `engine/src/surface.rs` (after T17) | `implementer`, then `reviewer` | Specified; `surface.rs` waits its turn |
| T17t | *(review round 1)* Failing tests for T17, invented state files only: the `syllabi` list's shape exactly as §4.8 states it (each field, the course-slug order, items in date order with their `state` codes and counts, a held rendering listed under each candidate course); codes, never row text; read from `state/syllabi/` only; an unknown extraction `version` reads as needing a re-read; no `state/syllabi/` leaves `surface-today-{s1,s1-migrated,full}.json` byte-identical (compared, never regenerated) | `engine/tests/surface_syllabi.rs` | `test-writer` | TDD first; the surface references are read, never written |
| T17 | *(review round 1)* The `syllabi` read model (§4.8) in `surface`, serialized only when non-empty; `docs/surface/anatomy.md` follows in T13 | `engine/src/surface.rs` (after L7, before T16), `engine/src/syllabus.rs` (a reader for the state files, after T15) | `implementer`, then `reviewer` | Specified, checked by T17t; T11b and T11f render it, so it lands before them |
| T11a | Confirm screen: the "Your syllabi" section with each course's outcome and the offer; uncovered rows point at it; no preset on a contested row (`console.js:830`); the two string changes and `static_assets.rs:1754` | `app/static/console.js`, `app/static/index.html`, `app/tests/static_assets.rs` | `console-ui` | `app/static` is its lane |
| T11b | The ask card's offer and pending note; the Schedule view's course entry: office hours, items not yet proposed, needs-a-date items and notices, the offer, *(review round 1)* rendered from T17's `syllabi` fields, wording each code | `app/static/console.js` (after T11a), `app/tests/static_assets.rs` | `console-ui` | Same lane; needs T17 |
| T11c | The wizard calls `capture_syllabi` after `capture_courses` (`console.js:3180`), with its own outcome line | `app/static/console.js` (after T11b) | `console-ui` | Same lane; needs T9 |
| T11d | Offer C's inline typed row (Q27), or the day-3 note if §14 finds `--confirm` cannot take a typed row | `app/static/console.js` (after T11c), `app/src/week.rs` if a command is needed | `console-ui`; `implementer` for any command | Same lane; the command is thin wiring |
| T11e | The deck renders `syllabus-exam` cards and a grading amend card's mapping change legibly | `app/static/console.js` (after T11d), `app/tests/static_assets.rs` | `console-ui` | Same lane; needs T4e |
| T11f | *(2026-10-18)* Offer A on the page: Connect Simple Syllabus where a host is curated, its outcomes (§8), the match pick *(review round 1: and "Use Simple Syllabus's instead", both from T17's `held` and settled through `syllabus_settle_held`)*; the course entry's policies and quotes; the quote beside a grading amend card, from the local file; the past-due row's late note | `app/static/console.js` (after T11e), `app/tests/static_assets.rs` | `console-ui` | Same lane; needs T8a, T10c, T16 and T17; Quinn words new copy |
| T12 | *(amended 2026-10-18; review round 1)* Privacy: hand lane L4 §6.1's six statements and the stated bound (Q38); the release-guard line in `HANDOFF.md`: no release from a `main` carrying this lane's app surfaces, or a curated syllabus host, before L4's page carries the statements that apply; and Q42's merge gate as a ledger row for the controller | a hand-off note to L4's owner, `HANDOFF.md` | `docs-keeper` | L4 owns `site/privacy.html` and `PRIVACY_VERSION` (`app/src/account.rs` is on the contract list) |
| T18 | *(2026-10-18)* First-pass terms read: the vendor's terms of service and privacy policy (§15.6's clauses, each quoted with its section number and a one-line reading), and whether the school publishes its own contract terms with the vendor; flags, not a legal opinion | `docs/notes/` (a new note) | `researcher` | Read-only research; counsel reads it with the page before the Pilot (Quinn's "lawyer later") |
| T13 | Reference: `syllabus-add` (all four forms, the held renderings and the replacement rule), `judge --only syllabus`, `contested`, the new card kind and the three amendable fields in `docs/reference/engine-commands.md`; the Simple Syllabus window and its commands, beside the mail window's rules, in `docs/reference/app.md`; the new commands, the staging folder and `syllabi/` in `docs/reference/app.md`; the `syllabi` read model in `docs/surface/anatomy.md` | `docs/reference/*`, `docs/surface/anatomy.md` | `docs-keeper` | Docs |
| T14 | Review before push: everything a cheaper agent changed | none | `reviewer` | CLAUDE.md: cheaper agents' changes go through `reviewer` |

Contract list: **one file changes, `approvals.rs` (T4e)**, by `contract-engineer` at xhigh, reviewed by
`contract-reviewer`. `LOCAL_CARD_KINDS`, `sync.rs` and its tests, `ids.rs`, `entitle.rs`, `write.rs`,
`journal.rs` (`VIAS` included) and `engine/tests/fixtures/**` do not change. If T4a, T4d, T4f or T5
finds it needs a new `journal::VIAS` value, or a sync test has to change, that piece moves to
`contract-engineer` and `contract-reviewer` too. *(amendment 2026-10-18)* Still one contract-list
file: `late_policy` joins T4e's `approvals.rs` change, and PR #19 adds nothing to `AMENDABLE_FIELDS`
(Quinn's ruling O1/Q2 (a)). *(convergence round 1)* T4e's `approvals.rs` change is four pieces, not
two: the `syllabus-exam` arm; the three amendable fields; `withdraw_syllabus`, the only code that
stamps or moves a syllabus card (§4.6); and the finishing arm in `process_approvals` for a syllabus
card stamped `superseded` but not yet moved. If T4d or T16 finds it needs any other card transition,
that piece moves to T4e too. T10b's window touches no contract-list file; if it needs anything from
`app/src/credentials.rs` or `account.rs`, it stops for `contract-engineer`.

## 14. Unverified, to settle before the plan

- Whether a UA student cookie session reaches Blackboard course contents (S1).
- ~~Whether a UA Simple Syllabus page shows meeting times, a dated schedule and weights, and its shape
  and terms (S2).~~ *Settled by S2 on 2026-10-18 (§3), except the full terms read (T18, Q39).*
- *(E10, amended 2026-10-18)* Which shape PR #19 left in `commitments.rs`: Quinn ruled the per-source
  table (B3), so T4a adds a row; T4a still reads what is on `main` before it starts. *(review round 1)*
  In particular, whether `registrar:` is still in `SERIES_KEY_PREFIXES` (it is at `2ad1e3d`) or B3
  moved it to the table's landing column with its kept-entry card (#19's close-out amendment, items 8,
  12 and 16, still for review); the `syllabus:` row is the same either way (§4.4). R0 answered the
  rest: the rows carry per-meeting dates (G3 uses them), the rows call is not paged, and `EXAM`
  meetings sit beside `CLAS` meetings. Open: how #19's parser treats `EXAM` meetings (its D3
  amendment), which §9's exams bullet and T4dt follow. *(convergence round 1)* Also open: whether B3
  writes tiers as a number column or as row order (§4.4: the renumbering applies only to the first),
  and whether any #19 test asserts a tier's number rather than an order (Q48's case; on #19's branch
  `precedence` is still a private function returning `(u8, &str)`, and the tests read for this round,
  `by_key_takes_one_record_per_key_google_first_then_calendar_key` and
  `twins_under_two_keys_are_one_proposal_by_google_first_precedence`, assert orders, not numbers).
- *(2026-10-18)* S3's questions (§15.7): whether a compiled-in script's same-origin call is answered
  where the plain GET was refused, where the list of the student's documents comes from, the
  signed-out signal, and the response size.
- *(2026-10-18)* Whether the mail window's runner (`app/src/mail.rs` and its `Page` seam, on the email
  branch until L0 merges) can be shared or must be copied; T10b follows what is on `main`.
- *(2026-10-18)* Where the read model shows a past-due task, for the late note (§16.3, T16).
- Which exact screen Quinn saw: the per-course row (`console.js:843`) or the whole-screen status
  (`console.js:876`). §8 changes both.
- Whether the pinned models extract reliably from the parts (T7); whether 7,000 characters holds a
  real term schedule; whether PDF table rows keep their order through the chosen crate (T3).
- Whether `onboarding::create_vault_in` can run an engine command at Finish the way §4.1 needs. If it
  cannot, the staged files are handed over at the console's first open instead.
- Whether every course has a course note, and so an opaque `id:`, when the pass runs. The wizard seeds
  one per course (`app/src/scaffold.rs:769-790`); a course added later may not have one (T4c, T5).
- Whether `commitments --confirm` can take a typed row (Q27, T11d).
- Whether `apply_amendment` (`approvals.rs:697`) can set a field the note does not yet have, and what
  its stale-value check compares for a mapping (Q25, T4e).
- Where `ingest`'s new-item path can test for a `syllabus:` task before creating, and which actor and
  `via` its writes use (T4f).
- Where the grades read model's `letter` lives on `main` now that M1 has merged (T4g).
- Whether the kind-check line numbers in `judgment_service.sql` still hold (T6a).

## 15. Simple Syllabus, built (Offer A as ruled, amendment 2026-10-18)

Quinn ruled on 2026-10-18, after S2, to build Offer A. This section is how it reads. §16 is what the
read, and every other source, now yields.

### 15.1 What is curated, and what is code

- **The host is a curated data field, never code** (CLAUDE.md rule 1). `Curated` gains
  `syllabus_host` (no scheme, no path) and `syllabus_kind` (`simple-syllabus`, the one kind so far).
  The vendor's paths (the "my courses" page, the document call) are the same for every tenant of the
  kind, as Blackboard's are for `lms_kind`, so they live in code per kind and every URL is built from
  the curated host. A second Simple Syllabus school is a row, nothing more. S3 checks the paths on UA;
  a second tenant's row is the proof that only the host differs.
- **Every row carries no host until Q39 clears it**, UA's included. A release turns Offer A on or off
  by data, never a build flag.
- **A gated link that step 1 finds is evidence, never a host the app opens.** Revision 2 let a host
  come "from step 1's gated link"; this amendment drops that (Q44): a host learned from a page would
  let a page steer a signed-in window. The link is recorded as an outcome ("The syllabus is in Simple
  Syllabus") and is what tells the founder a row is worth curating.

### 15.2 The window and the in-page read

S2 showed the host's own document call answers inside the page and refuses the same GET sent outside
it with the window's cookies (403). So Offer A does not copy cookies to `ureq`, as step 1 and the
registrar do. It reads **in the page, the way the app already reads pages in its own windows**: the
mail window's runner (`app/src/mail.rs`, email spec D5 and D19), which evaluates compiled-in scripts by
name and takes each result from WebView2's script-completion value.

- **The window.** Label `syllabus-signin`, opened by the console's Connect button with the student
  present, visible, at the curated host's "my courses" page. A fresh throwaway data folder under app
  data (`lms_link`'s discard-everything pattern, never the kept Blackboard or mail profile); the
  student signs in through the school's own SSO in it. External schemes are refused, as the mail
  window refuses them. **No capability grant** covers it, so every invoke from it is denied, and it
  has no debugging port in any release build.
- **The scripts.** Small readable files under `app/assets/syllabus/`, compiled in, run by name and
  never from text anywhere else: a readiness probe (signed in or not, a fixed word), a lister (the
  student's documents on "my courses", returned as an opaque list the app holds in memory only), and
  a reader (one document, by its code, through the page's own document call from the page's origin,
  returning that call's JSON). "Teach once, run deterministically forever": each is the extraction
  script a later Connect runs; a broken one is a named outcome, never an empty row.
- **The host check, twice.** The app evaluates a script only while the top-level URL is exactly
  `https://<curated host>/…` (no user part, no port, case-blind host, grades' `on_host`), and the
  script's first line checks `location.host` again and returns a fixed refusal otherwise, because a
  script queued before a navigation can run on the next document (the mail spike's S1c).
- **New windows.** The host opens each document with `target=_blank`, which a sign-in window refuses
  today. A new-window request for a URL on the curated host is loaded in the same window; any other is
  denied (Q30). The reader by code needs no navigation at all, if S3 confirms the lister.
- **What is never done.** No CDP, no simulated keyboard or mouse, no IPC from the page, no cookie
  read, no script text from a page or the network, no page text in an error, a log or telemetry
  (every refusal is a fixed word; a log line carries counts only, never a document code, course or
  section: S2's transcript incident is the reason).
- **Bounds.** At most 16 documents a Connect, at most 2 MB of JSON each, a per-document and a
  whole-read timeout; past a bound is a named outcome ("not tried").
- **Hand-over.** Each document's JSON is written to staging under the open profile's folder (§4.1),
  handed to `syllabus-add --source simple-syllabus --file <path>` and deleted, whatever the exit.
  *(review round 1)* What a later pick or "Use Simple Syllabus's instead" needs is kept by the engine
  as a held rendering (§4.1), never by the app, so nothing staged outlives the run. The
  window is destroyed after the last document and its data folder deleted (with the mail session's
  retry while WebView2 lets go); a stale folder is swept at the next launch.
- **The fallback order (Q29).** If S3 finds the script's same-origin call refused too, the read
  falls back to (b) the window's own WebView2 response event for the page's own document call
  (`WebResourceResponseReceived`, host-side, filtered to the curated host and the one path, opening
  each document in the same window), and only then to (c) the rendered page's text. Each is a
  WebView2 API the app already holds through `with_webview`; neither is CDP.

### 15.3 What a document yields, and the copy kept

The engine, not the app, parses the JSON: `commands.rs` and the window compute nothing. It reads the
structured term (name, start and end date, timezone), the section attributes S2 found (section
title, schedule type, instructional method, credit hours, CRN) and the `components[]` in order, each
with its type and HTML. An unknown shape is a named failure that writes nothing.

- **Rendering first, then the screen.** Each component's HTML is rendered to text by §4.2's HTML
  rule, with **every attribute dropped** (S2's inline-style percents never reach the grading part)
  and each component starting a new block under its own title. `content` feeds every part.
  `instructor` feeds only office-hours lines, contacts removed (§4.2's rule); the rest of it is
  dropped. `material` (textbooks) is dropped (§2).
- **Identity by rule.** The identity part is one line built from the attributes (term, course code,
  section label); it is never the model's to find, and the CRN is never in it.
- **Term dates by rule.** The structured start anchors week numbers when the text gives none (Q37);
  both dates feed the stale-syllabus check (§12).
- **A linked PDF** inside a component is named ("links a PDF; add it with Add syllabus"), never
  fetched: the read is one call per document.
- **The copy kept** is `syllabi/<slug>.html`: the term, the attributes and the `content` and
  `instructor` components in order, attributes stripped (Q31). The JSON itself never enters the vault.

### 15.4 Which course a document belongs to

`syllabus-add --source simple-syllabus` matches the document itself, deterministically:
1. a confirmed `registrar:` note whose key carries the same term and CRN, where PR #19 made one;
2. else the subject and number from the attributes, normalised, against each active course note's
   `title` (the human code the wizard seeds), with the term's dates containing today or starting
   within 30 days.

One match gives the slug. None is "lists a course Knowlu does not have", and the document is dropped
unread. Two is "matches two of your courses; pick one". *(review round 1, Q47)* The staged JSON is
still deleted whatever the exit (§15.2), so the engine keeps what the pick needs: it writes the
rendering as a **held rendering** naming both slugs (§4.1) and records the outcome on both courses.
The student's pick runs the held form, `syllabus-add --held <id> --course <slug>` (through
`syllabus_settle_held`, T8a), with no window and no second sign-in. A held rendering not picked is
deleted after 30 days. The engine never guesses.

### 15.5 When a course already has a syllabus (Q33)

A Simple Syllabus document replaces a file step 1 found in the LMS (the school's system of record
over a copy), and never replaces one the student uploaded or pasted (the student asked). Keys are
built from content (§4.4, §4.6), so a replacement that says the same thing files nothing new.
*(convergence round 3)* A replacement whose meetings part says nothing usable (S2 found no meeting
times in 6 of 8 documents) ends nothing either. Its items and grading stand, and the course's
`syllabus:` series are dropped with no `ended` entry (§4.4's usable meetings read, Q50). A
course whose own upload is kept reads "You added your own syllabus for this course; Knowlu kept it",
with "Use Simple Syllabus's instead" beside it.

*(review round 1)* **How the engine tells them apart.** The extraction file records each copy's
`source` (§5), so the rule is read from data, not guessed from a file's extension. `syllabus-add
--source simple-syllabus` over a course whose `source` is `upload` or `paste` writes nothing in
`syllabi/` and leaves that course's extraction as it was; it holds the rendering as `kept-own`
(§4.1). "Use Simple Syllabus's instead" runs the held form, which replaces the student's copy as the
file form would. T15t pins every pair of sources.

### 15.6 The terms read: the clauses to check

S2 found the vendor's terms at `ua.simplesyllabus.com/en-US/terms-of-service` and its privacy policy at
`/privacy-policy`; neither has been read in full. By Quinn's "lawyer later" ruling, counsel reads them
with the privacy page before the Pilot. T18's first pass (`researcher`, flags only, not an opinion)
quotes and reads each of these, and Q39 decides what a friends release may carry meanwhile:
1. **Automated access**: any bar on bots, scripts, scraping, crawling or "automated means", and
   whether it is about load or about any non-manual read; whether reading one's own pages counts.
2. **Interfaces**: any bar on using the service other than through its provided interface, on
   calling its APIs directly, or on reverse engineering; the document call is the page's own.
3. **Licence scope**: personal, non-commercial, "for your own use" language, and whether a tool
   acting for the user under the user's own session is inside it.
4. **Agents and third parties**: any bar on letting a third party access the service on the user's
   behalf, or on sharing access; Knowlu holds no credential and no cookie, but a lawyer should say
   whether the in-session read is "access by Knowlu".
5. **Content and copying**: who owns syllabus content (the institution, the instructor), and any bar
   on downloading, storing, reproducing or creating derivative works; Knowlu keeps a local copy and
   extracts from it.
6. **Sending content on**: any bar on transmitting content to third parties, which the model call
   does (a bounded excerpt, to a named zero-retention host).
7. **The institution's agreement**: whether the terms defer to the school's own contract with the
   vendor, and whether that contract or the school's acceptable-use policy says more (cloud ruling 12;
   the university-policy read is suspended until 20 paying users by Amendment 2026-09-30, A13, but a
   vendor's terms are not a university policy, so A13 does not cover them).
8. **Rate and load**: any limit on request rate or volume; Offer A reads at most 16 documents a
   Connect, with the student present.
9. **Suspension and liability**: what the vendor may do to the student's account on a breach (the
   student bears that risk, so the Connect screen must say what Knowlu does).
10. **Changes**: how the terms change and whether notice is given, so the read can be repeated.
11. **Privacy policy**: what the vendor collects from a signed-in page view, and whether the
    in-page read adds anything (it should add nothing beyond the student's own page view).
12. **Governing law and minors**: anything on users under 18 (the legal landscape note's open
    question) and the venue.

### 15.7 Spike S3: the in-page read (Quinn's block)

On a dev build (the diagnostic port is the spike's tool, never the product's), the window at UA's
host, Quinn signed in, shapes and header **names** only:
1. Does a compiled-in-style script's same-origin call to the document call return 200 and the same
   JSON keys the page's own call returns? If refused, which request headers differ in **name** (an
   anti-forgery header, `Origin`, `Referer`, a bearer from page storage)?
2. Where does the student's list of documents on "my courses" come from: which call, its key names,
   whether it is paged, and whether it lists only the current term?
3. What does a signed-out page look like to a probe: a redirect host, a status, a title?
4. How a document link opens: its URL's shape (pattern only), and whether a same-window load of it
   works.
5. Whether the response event of fallback (b) fires for the document call and its body is readable.
6. The size of a document's JSON (largest of the student's, in KB), for §15.2's bound.
7. The value shapes of `Instructional Method` and `Schedule Type` (the closed lists, no course), for
   Q36's later ruling.
Cleanup as S2's: window closed, dev build quit, no session folder left, no scratch profile kept.

## 16. The widened read (Q10 as ruled, amendment 2026-10-18)

Quinn's words: "It has grade information, late work policies, test and quiz dates, assignments,
etc." Most of that was already in ruling 1 (§0); what is new is the late-work policy and, under
"etc.", the makeup and attendance policies. The widening applies to **every** source (step 1, Offers
A and B), not only Simple Syllabus: one screen, one schema, one filer. Its VISION test: a past-due
assignment whose late window is still open is a "what's next?" answer ("still worth 70% until
Friday"), and one whose window has closed is not; a student who misses an exam needs the makeup rule
at once.

### 16.1 What is read, and where each lands

Existing note kinds first. Nothing here adds a note folder, so `ids::NOTE_FOLDERS`, `sync.rs` and the
tripwires stay untouched.

| Read | How | Lands in | How it lands |
|---|---|---|---|
| Exam and quiz dates | schedule part, model (§4.3) | `commitments/` (a one-off `exam`) and `tasks/` ("Study for"), or `tasks/` for a quiz | `syllabus-exam` or `kind: task` card (§4.6), unchanged |
| Assignments with dates | schedule part, model | `tasks/` | `kind: task` card (§4.6), unchanged |
| Assignments with no date | the same | nowhere: listed on the course as "needs a date" | never proposed (`VISION.md`: Knowlu never invents a due date) |
| Grade weights and categories | grading part, rules then model | `courses/<slug>.md` `grade_weights` | the course's amend card (§4.7). *New:* weights given in points become percents only when the syllabus states the total and they sum to it; otherwise shown, not proposed |
| Grading scale | grading part | `courses/<slug>.md` `grade_scale` | the same card |
| **Late-work rule** | new policies part (§16.2), rules then model | `courses/<slug>.md` `late_policy` (§16.3) | the same card (Q34) |
| **Makeup, missed-exam and attendance policy** | the policies part | nowhere in a note: a verbatim quote in `state/syllabi/<slug>.json` | shown on the course, never proposed (§16.4, Q35) |
| Office hours | meetings part | nowhere in a note (§4.8) | shown, unchanged |
| Term dates (Simple Syllabus) | structured, by rule (§15.3) | the extraction only | the week-1 anchor and the stale check |

**Why not `info/`.** An `info/` item is a thing to know with an open-and-close lifecycle, one of four
fixed kinds (`package`, `notice`, `heads-up`, `other`, `engine/src/info.rs`) and an expiry. A course's
policy has no close, would sit among Today's things to know all term, and on a second desktop would be
a synced note a student never asked for. The course is where a student looks for a course's rules.

### 16.2 The screen's fifth part, and the schema's additions

§4.2's screen gains a **Policies** part: every line of a block whose heading matches `late`,
`make-?up`, `missed`, `extension`, `absence` or `attendance`, up to the next heading; and any line
outside such a block carrying "late" beside "accept", "penalt", "deduct", "%" or "per day". Bound
**1,500** characters. The instructor rule still applies (lines with a contact are dropped). The total
becomes **13,500** (Q38). A rule settles one shape: a line saying late work is not accepted, as the
sole late line, gives `accepted: false` with that line as its quote. The rest is the model's.

§4.3's draft schema gains, inside the same `version: 1` (nothing has shipped):

```
"policies": { "late": { "accepted": bool|null, "penalty_percent": number|null,
                        "per": "day|hour"|null, "max_days": int|null, "grace_hours": int|null,
                        "quote": string|null },
              "makeup": { "quote": string|null }, "attendance": { "quote": string|null } },
"confidence": { "meets": 0..1, "items": 0..1, "grading": 0..1, "policies": 0..1 }
```

### 16.3 The late-work rule on the course note

- **The field.** `late_policy: {accepted: true, penalty_percent: 10, per: day, max_days: 3}`, one
  single-line flow mapping through `write`'s frontmatter surgery, emitted by `yamlemit` as
  `grade_weights` is. A key with no value is left out, never written as `null`.
- **Valid only when whole.** `accepted` is set; with `false`, no other key; with `true`,
  `penalty_percent` in 0..100, `per` from the closed list, `max_days` in 0..60, `grace_hours` in 0..72,
  each where present; the late quote survived the substring check (§16.4). Otherwise nothing is
  proposed and the course shows the quote with "Read a late-work policy; could not turn it into a
  rule". A rule that differs per kind of work ("no late exams; homework −10% a day") is quote-only.
- **How it lands.** On the course's one amend card with weights and scale (§4.7, Q25).
  *(review round 1)* The card carries the rule's values and Knowlu's fixed context line (§4.6), never
  the policy's words: cards sync, and quotes stay on this desktop (§16.4). The console shows the
  quote beside the card from the local extraction file where it has one; a second desktop shows the
  card without it. `AMENDABLE_FIELDS` gains `late_policy` (T4e, contract list).
- **What reads it.** The read model only, deterministically: a past-due task of that course shows
  "Late: −10% a day, up to 3 days (your syllabus)", "Late work not accepted (your syllabus)" or, past
  `max_days`, "Past the late window (your syllabus)". `rank` does not read it (Q34): using it to order
  past-due work is a ranking change with its own honesty test, and a misread rule must never quietly
  bury a task. A due the student set by hand, an extension, is the student's: the note shows the rule
  beside it and changes nothing.
- **Judge once.** The amend card is the proposal judge once allows; a rejected set is remembered by
  its `source_uid`; a value the student typed into `late_policy` is never proposed back over unless
  the syllabus itself changes (a new hash).

### 16.4 Quotes, and why they are verbatim

A policy is shown to the student in the syllabus's own words, never a model's summary (`VISION.md`:
classification over generation). The model returns each quote as it appears; **the service's
validator keeps a quote only if it is a substring of the excerpt it was sent** (after folding
whitespace), at most 300 characters, and otherwise drops it, and drops the late rule with its quote.
Quotes stay in the local extraction file and show on the course on this desktop, like office hours.
*(review round 1)* No card carries one, pending or settled (§4.6). The late rule's values sync on
the pending amend card and, once approved, on the course note.

### 16.5 Meeting the coursework already in the vault

Nothing a syllabus says may duplicate what the LMS or a platform already put in the vault, and nothing
the student decided is proposed back. §4.6's rules carry the weight; this amendment widens one term:
- **"The feed" in §4.6 reads "an authoritative source"**: a task whose `source_uid` came from `ingest`
  (the LMS feed) **or from `coursework`** (a homework platform). Either is the authority on its own
  items; a syllabus date that disagrees with one files nothing and shows the notice (Q45). A pending
  card from mail stays beside a syllabus card, as PQ13 (b) rules for two noticed sources.
- **The match** is §4.6's: the same course, a title match (normalised, every token of one in the
  other, or the same kind-and-number key), then the due day. The synonym table gains the platform
  words a syllabus uses for platform work (`zybooks`, `participation`, `challenge`, `lab` → assignment)
  only if the eval shows misses (T7); until then a platform item and a syllabus item that do not match
  by these rules both stand, and the student rejects the double, which stays rejected.
- **Adoption (Q23)** stays in `ingest`'s new-item path only. A platform item arriving after an
  approved syllabus task does not re-key it in this lane; the pending-card withdrawal still applies to
  platform items (`approvals::withdraw_syllabus` with the *authoritative match* reason, §4.6; T4e,
  pinned in T4dt), so only an already-approved syllabus task can sit beside a later platform task
  (Q45).
- **Judge once** holds as written: an agent never re-sets a field the journal shows the student set
  (`journal::human_edited`); a syllabus change to such a field is an amend card.
- **Onboarding mid-term**, which every friends tester does in October: past items are never proposed;
  most items inside the next two or three weeks are already in the feed and fall out as duplicates;
  what is left is mostly exams and assignments beyond the feed's reach, which is the value. The course
  says so in one line ("31 dates in your syllabus: 12 past, 9 already in Blackboard, 4 to review, 6
  proposed nearer the time").

### 16.6 The "etc.": what is not read, and why

Quinn's "etc." is open, so this amendment names what it does not read and asks (Q36): required
materials and textbooks (the `material` component: not a "what's next?" answer); credit hours (GPA is
Beyond; the registrar or the LMS is the better source); extra credit and per-item points (the item's
share of the grade could later ground importance, but needs the category mapping Q26 defers); the
instructional method (an online, asynchronous section could spare the student the typed-times ask,
which is real value, but its values are each institution's configuration: S3 records their shape and
a later ruling maps them as curated data); readings, topics and cancellations (Q21).

### 16.7 Which judgment runs, and where

- **One kind, `syllabus`**, as §4.3 says, now with the policies block (Q40). One call a syllabus
  reads every part left after the rules. The prompt, the strict schema, the validator (the closed
  lists, the per-part confidence floor, the quote check, the bounds) and the pinned model live
  server-side (T6b). The engine sends the excerpt, the vault-local date and the course note's `id:`.
- **`judge` is where it runs**, in the slot after enrichment or as `judge --only syllabus` after an
  add or a Connect; gated by `entitle.rs` like every `judge` pass; always exit 0, each refusal a
  named line. **The pass writes only `state/syllabi/<slug>.json`.**
- **`rank` never calls a model.** It files every card from the extraction file, deterministically
  (§4.6), and the read model shows the policies from the same file.

### 16.8 The caps: the deck's 15 a day, the first day, and the service's daily cap

- **The deck.** Every syllabus card (task, exam, and the course's amend card) is charged 1 to the 15
  new proposals a day (`proposal_weight`); at most 8 syllabus cards are first proposed a day (Q21);
  overflow is snoozed by `defer_over_budget`, never deleted. Within the 8, items due within 7 days
  come first, then amend cards, then later items, soonest first (Q41): a Connect of eight courses
  would otherwise spend a whole day's 8 on grading cards.
- **The first day.** Syllabus cards are held to day 2 (Q24). On days 2 to 14 they share the 15 with
  the event cards lane L3's first-day **event** allowance lets `judge` produce, and with held mail
  cards; nothing changes the 15 or `defer_over_budget`'s order, and a test pins the sum.
- **The service.** `DAILY_CAP.syllabus` is 20, doubled to 40 on an account's first two judging days by
  the existing rule; no `judge_allowances` row for `syllabus` (Q41).

## 17. Decisions for Quinn (2026-10-18)

**Ruled 2026-10-19 (Quinn): Q29-Q47 all as recommended** (Q43 with the signature; Q49 and Q50 as marked below; Q48 stays conditional). Recommendation first, one line of why, the alternative after. The controller asks them one at a time.
Q29-Q33, Q44 and Q47 are about the read, Q34-Q38, Q45 and Q46 about what it yields, Q39-Q43 and
Q48 about running and shipping it. *(review round 1)* Q42 is rewritten and Q46 and Q47 are new; the
rest are unchanged. *(convergence round 1)* Q48 is new and asked only if its case arises.
*(convergence round 2)* Q49 is new. *(convergence round 3)* Q50 is new. *(convergence round 4)*
Q50 (a)'s second cost is stated, and its recommendation is marked QUINN TO CONFIRM.

29. **How the page is read.** *Recommend:* a compiled-in script, run by name, calling the page's own
    document call from the page's origin, its JSON returned by the script-completion value, exactly
    the mail window's rules (§15.2); if S3 shows that call refused, (b) the window's own WebView2
    response event for the page's call, and only then (c) the rendered page's text. *Why:* it is the
    pattern already built and reviewed for mail, and it reads the structured JSON S2 found.
    *Alternatives:* (b) first (reads exactly what the page fetched, but must open every document in
    the window); (c) first (no call at all, but the most fragile).
30. **The `target=_blank` links.** *Recommend:* a new-window request on the curated host loads in the
    same window; every other is denied. *Why:* the student can still open a document by hand, and
    nothing leaves the host. *Alternative:* deny all, and read only by code.
31. **The copy kept in the vault.** *Recommend:* the engine's rendering, `syllabi/<slug>.html` (term,
    attributes, `content` and `instructor` components, attributes stripped); the host's JSON never
    enters the vault. *Why:* readable by the student, re-extractable, and no vendor configuration in
    a plain-text vault. *Alternative:* keep the JSON as sent.
32. **Who matches a document to a course.** *Recommend:* the engine, by a confirmed registrar note's
    term and CRN first, then subject and number against the course note's title within the term; one
    match or a pick, never a guess (§15.4). *Why:* deterministic, tested, and `commands.rs` computes
    nothing. *Alternative:* the app maps before calling `syllabus-add --course`.
33. **Two syllabi for one course.** *Recommend:* Simple Syllabus replaces an LMS-found file and never
    the student's own upload or paste, which offers "Use Simple Syllabus's instead" (§15.5). *(review
    round 1)* The extraction file records `source` from `version: 1` so the engine can apply this, and
    the vendor's copy waits as a held rendering (Q47). *Why:* the
    school's system of record over a copy, and the student's own choice over both. *Alternative:* the
    latest add always wins.
34. **The late-work rule.** *Recommend:* `late_policy` on the course note, through the course's one
    amend card with weights and scale (`AMENDABLE_FIELDS` gains it, contract list), shown on that
    course's past-due tasks, and **not** read by `rank` now (§16.3). *Why:* the course is where its
    rules live, it syncs, and a misread rule never reorders work. *Alternatives:* (b) also let `rank`
    lift past-due work still inside its late window (a ranking change, its own design); (c) an
    `info/` note per course (rejected in §16.1); (d) shown only, never stored.
35. **Makeup, missed-exam and attendance policies.** *Recommend:* shown on the course as verbatim
    quotes from the local extraction, never fields and never proposed (§16.4). *Why:* they have no
    shape a rule can use, and a quote is honest. *Alternative:* store them on the course note as
    quoted text, so a second desktop shows them.
36. **The "etc."** *Recommend:* nothing more in this lane: no materials, credit hours, extra credit or
    per-item points; S3 records the instructional method's shape for a later ruling that could spare an
    online section the typed-times ask (§16.6). *Why:* each fails or defers the "what's next?" test
    now. *Ask:* did "etc." mean anything else?
37. **Week numbers from structured term dates.** *Recommend:* Simple Syllabus's structured term start
    anchors "Week N" when the text gives no week-1 date (week 1 is the Monday-to-Sunday week holding
    the start); the registrar's dates do not (§9). *Why:* it is the syllabus's own data; the
    registrar's is another source's, and week-numbering habits vary. *Alternative:* text dates only,
    as revision 2 says.
38. **The excerpt bound.** *Recommend:* the policies part at 1,500 and the total at **13,500**
    characters, every other part unchanged. *Why:* the parts S2 sized stay whole, and the page states
    one number. *Alternative:* keep 12,000 by cutting the schedule part to 5,500.
39. **What a friends release may carry.** *Recommend:* the UA row gets its `syllabus_host` only when
    S3 has passed, T18's first-pass terms read flags no clause that plainly bars a student's own
    in-session read, and L4's page carries §6.1's statements; otherwise the row ships without one
    (data, no build flag) and Offer B still ships. Counsel before the Pilot. *Why:* "lawyer later"
    without shipping against a plain bar. *Alternative:* carry the host from the start, terms read
    later.
40. **One judge kind.** *Recommend:* one `syllabus` kind with the policies block, and the
    quote-is-a-substring check server-side (§16.4, §16.7). *Why:* one call a syllabus, one pin, one
    eval. *Alternative:* a second kind for policies, pinned separately.
41. **Caps and order.** *Recommend:* `DAILY_CAP.syllabus` 20 (40 on the first two judging days, the
    existing rule), no first-day allowance for it; in the deck, the 8 a day take items due within 7
    days first, then amend cards, then later items (§16.8). *Why:* one call a syllabus builds no
    backlog, and a near due date beats a weight. *Alternative:* amend cards outside the 8.
42. **What "syllabus off" means** under the session-5 slip rule. *(Rewritten in review round 1.
    Revision 2's "the service's `syllabus` cap is 0 in prod" does not work: `DAILY_CAP` is one code
    constant shared by staging and prod, no task built a per-environment row or its outcome, and
    rules settle some parts with no model call at all, so Offer B would still file cards.)*
    *Recommend:* **a merge gate, not a switch.** No task that gives a student a way to add or find a
    syllabus (T8a, T8b's hand-over, T9, T10c, T11a-T11f) merges to `main` until L4's page carrying
    §6.1 items 2-5 is on `main` (§13's gate list). The engine and cloud tasks may merge first:
    without an app surface nothing creates a `state/syllabi/` file, so nothing is read, sent or
    filed. So a release has either no syllabus surface or a page that names it. A curated host stays
    Q39's. A build ahead of its server gets a named outcome, never a failure (T5t's 404 case). *Why:*
    nothing to configure per environment, nothing to forget at L9, and no state where the page and
    the app disagree. L4 is ahead of L5 in the `console-ui` queue anyway, so the gate costs nothing
    unless L4 slips. *Alternatives:* (b) a per-environment server setting (a function secret, say
    `SYLLABUS_ENABLED`, unset in prod), with a new `unavailable` outcome on the route, a T5t case and
    a T6b task; but rule-settled parts would still file cards, so the app must hide Offer B too,
    which needs a way for the app to learn the setting (new contract work); (c) hide the surfaces by
    a build flag, against "data, never a build flag".
43. **Q4-Q28's record.** *Ask:* confirm Q4-Q28 as recommended in §11 (Q6, Q10, Q13, Q19, Q25 and Q28
    as amended here), since this file never recorded how they were ruled. **Ruled 2026-10-19
    (Quinn), with the signature:** Q4-Q28 as recommended, and Q19's restated stage.
44. **A host learned from a page.** *Recommend:* never: a Simple Syllabus link step 1 finds in the LMS
    is an outcome and a reason to curate a row, not a host the app opens (§15.1). *Why:* a page must
    not steer a signed-in window. *Alternative:* revision 2's "or from step 1's gated link".
45. **Homework platforms as authorities.** *Recommend:* an item from `coursework` (zyBooks, VHL)
    counts as authoritative in the duplicate rule, as the LMS feed does; `coursework` gets no adoption
    branch in this lane (§16.5). *Why:* the platform is the source of its own due dates, and a second
    adoption path is a second place a wrong re-key could happen. *Alternative:* adoption in
    `coursework` too.
46. *(review round 1)* **What a pending exam card carries off the device.** Every syllabus card
    syncs (§4.6), and a `syllabus-exam` card proposes a one-off commitment, so its payload (title,
    course, date, times, the room as the syllabus writes it, one quoted schedule row) is on the
    server before the student decides. `CLAUDE.md` keeps *commitment proposals* local, but by kind
    (`LOCAL_CARD_KINDS`), and `event-accept` cards already sync with a `where` (`eventaccept.rs`,
    `WHERE_MAX` 80). *Recommend:* keep `where` on the card, as `event-accept` does, and say so on the
    page (§6.1 item 4). *Why:* a card approved on a second desktop writes a whole exam, and an exam
    room is coursework of the same class as a confirmed class note's `where`, which already syncs.
    *Alternative:* leave `where` off the synced card and add it at approve from the local extraction;
    a card approved on a desktop without the file then writes no room, and the student adds it.
47. *(review round 1)* **Where a Simple Syllabus document waits for the student.** A two-way match
    (§15.4) and a course with the student's own upload (§15.5) both need the document after the
    Connect window is gone. *Recommend:* the engine holds its rendering under
    `state/syllabi/held/` (never synced, read by nothing but the held form, deleted after 30 days),
    and the pick or "Use Simple Syllabus's instead" runs `syllabus-add --held` (§4.1). *Why:* the app
    keeps nothing staged past one run, the engine's copy is the same rendering the vault keeps
    anyway, and the student never signs in twice for one choice. *Alternatives:* (b) the app keeps
    the staged JSON until the student picks (host JSON outside the vault for days, against §15.2's
    "deleted, whatever the exit"); (c) drop the document and point the student at Add syllabus, so
    Offer A never reaches that course.
48. *(convergence round 1)* **If a PR #19 test pins a tier's number.** Q6 needs `syllabus:` on a
    tier of its own between the registrar and Google, so `google:` moves from 1 to 2 and the default
    row from 2 to 3 (§4.4). That keeps every pair's order, so any #19 test that asserts an order
    passes untouched. Only a test asserting a number (that `google:` "is tier 1") would fail, and
    this spec forbids editing #19's tests. This question arises only if T4a finds such a test on
    `main`; it stops until Quinn rules. *Recommend:* rewrite that one assertion to the order it
    protects (registrar before Google, Google before the rest), in a commit of its own whose message
    says why, reviewed by `reviewer`, and change no other #19 test. *Why:* the number was never the
    contract, the order is, and the rewritten assertion still fails on any reordering.
    *Alternatives:* (b) the registrar lane's owner changes the test on its own branch first, so this
    lane edits nothing of #19's (slower; one more hand-off); (c) leave the numbers and break the
    syllabus and Google tie by a third key (more code for the same order, and the number still
    misleads).
49. *(convergence round 2)* **A syllabus meeting removed before it began.** The likeliest correction
    comes at onboarding: a student confirms a lab, then adds a corrected syllabus before the term's
    first week, and the lab is gone from it. The ended arm (§4.4) then stamps the day before the
    series' `first` (or, with `first` null, the last meeting weekday before the run, which also falls
    before the term), and the end card proposes an `until` earlier than the lab's first meeting.
    *Recommend:* (a) keep that. The end card's existing copy shows the date, and approving it leaves a
    note that never blocks time; #19's path and the console stay unedited. *Why:* it is correct (the
    lab never meets), it uses one card kind the student already knows, and the note stays in the
    vault as a record the student can delete. *Alternatives:* (b) give an end card whose `until` falls
    before the dropped series' `first` its own copy ("No longer in your syllabus"), a console change for
    `console-ui` and one more read-model field; (c) file a delete proposal instead, a new card kind
    and an approve path that removes a note, which nothing in the commitment model does today.
    **Ruled 2026-10-19 (Quinn): (a).** A confirmed meeting removed before its term began gets the
    existing end card, stamped with a date before its first meeting (approving it leaves a note that
    never blocks time); no new card kind, no console change.
50. *(convergence round 3)* **A syllabus copy that cannot speak for the meetings.** The likeliest case
    is at a school with no registrar row, or a student who confirmed before connecting the
    registrar. Step 1 finds the LMS syllabus, the student confirms the class and its lab, then
    connects Simple Syllabus, which replaces the LMS copy (Q33). S2 found no meeting times in 6 of 8
    documents, so the new copy's items and grading read well and its meetings part is empty or below
    the floor. Ending every meeting the new copy does not name would file an end card for a course
    that is still meeting. This spec now lets the ended arm fire only on a **usable meetings read**
    (§4.4: status `read`, the meetings part settled by rules or at or above the floor, not clipped,
    at least one meeting). That part is a fix, not a choice. What happens to the course's existing
    `syllabus:` series when the present copy has no usable read is Quinn's call.
    **Ruled 2026-10-19 (Quinn): (a).** A syllabus copy with no usable meetings drops the course's
    syllabus series with no end card (as Q15 (a)). Accepted costs: unconfirmed proposals are lost,
    and a meeting a later usable read leaves out is never ended.
    *Recommended (restated in convergence round 4):* (a) the deletion arm's rule
    (Q15 (a)). Drop those series outright with no `ended` entry, so no end card is filed. A confirmed
    note stays, unwatched, and still blocks time by its `meets`, until a usable read arrives. *Why:* a
    read that saw no meetings is no more evidence of an end than a deleted file is. It reuses an arm
    already designed and pinned, and the series file never carries meetings from a copy the vault no
    longer holds. *Costs, two.* First, a class proposal the student had not yet confirmed goes with
    the old copy, and the course's times are asked for (Offer C, or the ask card, §8). That matters
    only at a school with no registrar row, since a registrar-held course gets no syllabus row at all
    (§4.4).
    *(convergence round 4)* Second, the drop leaves no series to age out. So a meeting that a later
    usable read leaves out is **never ended**: no `ended` entry, no end card, no swap card, and its
    confirmed note keeps blocking its old times. That is the outcome round 2's stamping was written to
    prevent (§12). It needs three things in order. First, a confirmed syllabus meeting. Then a copy
    with no usable read reaching a run. A pending copy counts, for example an upload whose post-add
    `judge` left it pending (no network, no entitlement, the daily cap, or a service that cannot read
    syllabi yet, §8) before `commitments` or `rank` ran. So does a Simple Syllabus copy with no
    meeting times. Last, a usable read that leaves the meeting out: a later copy's, or that pending
    copy's own once `judge` reads it. The deletion arm has the same cost for a file deleted and
    re-added with a run between. The meetings that later read does return are watched again, so a
    moved time still gets its change card. The recommendation stays (a), narrowly: the cost needs all
    three steps in order, and (b) adds a third arm to `refresh_series`, in the integrator-owned path
    where a silent error ends a class still meeting (T4a). If Quinn weighs a never-ended meeting
    above that, pick (b), the lighter of the two alternatives. The sentences that name the drop (§3,
    §4.4, §12, §15.5) and T4t's replacement cases are written for (a); a ruling of (b) or (c) revises
    them before T4a starts.
    *Alternatives:* (b) keep the previous run's `syllabus:` series for that course unchanged, never
    aged out, while the present copy has no usable read. A confirmed note stays watched, an
    unconfirmed proposal survives the replacement, and a meeting a later usable read leaves out still
    ends, because its series is still in the file. But the series file then holds meetings from a
    copy that is gone, and `refresh_series` gains a third syllabus arm ("kept as is"). (c) Carry
    the earlier copy's meetings part into the new extraction file, marked as from the earlier copy.
    This gives the same result as (b), derived from the extraction rather than from generated
    series state. But it adds a `version: 1` field, and one copy's data outlives that copy.
