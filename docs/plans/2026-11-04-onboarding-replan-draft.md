# Wave B replan after the 2026-11-04 live sign-in run: draft

**Status: DRAFT for Quinn's checkpoint. Not reviewed, not ruled, nothing dispatched.** It amends
`docs/plans/2026-10-23-wave-b-long-onboarding-plan.md` ("the plan") and does not edit it. Once Quinn
rules on §10, the rulings fold into the plan as a new §14 entry, and this file is its record.

**Date:** 2026-11-04. **Lanes:** O3 and O4. **Branch read:** `feat/o3-long-onboarding`.

**Authority.** The cloud design wins where it speaks. Then Quinn's ruling of 2026-11-04 (the plan's §3,
"Results of the 2026-11-04 live run"): the registrar is optional, and onboarding must not depend on a
registrar sign-in. That ruling retires spec §4.2 item 1 ("Registrar, automatically"), and Q1's "one
sign-in" stands only if outcome A holds. The rest of the spec and the plan stand unless this draft
says otherwise.

**How to read it.** §1 corrects three premises against the code. §2 says what decides A or B. §4 is
the task list. §5 is the registrar, §6 the class-search reader, §7 the LMS link. §10 holds the
questions for Quinn.

---

## 1. Three things the code says that today's record does not

These do not reopen today's facts. They change what the class-search reader costs.

1. **The class-search answer is not in the shape `parse_banner` reads.** The record says "the same row
   shape". The fields inside one meeting match (days, `beginTime`, `endTime`, `meetingType`, dates,
   building, room). The container does not:

   | | `parse_banner` | Class search, as recorded |
   |---|---|---|
   | Rows | `data.registrations[]` | not recorded; R0b records it |
   | A row's meetings | flat `meetingTimes[]` | `meetingsFaculty[].meetingTime` |
   | Registered status | `statusIndicator` | no such thing: a public list has no student |

   A test pins the difference: `p2_the_meetings_faculty_wrapper_is_not_read`. Handing class-search
   bytes to `commitments --registrar` would exit 2 with "not a list of registration rows". So C1 adds a
   second reader beside `parse_banner`, sharing its meeting rules. That test stays as it is.

2. **`run_parsed` reads an answer as the whole term.** A confirmed note whose section is missing from
   the answer gets its `until` pinned to yesterday. The class ends. Class search asks about one
   course at a time, so one failed or skipped query must never read as "dropped". Class search
   therefore never goes through `run_parsed`, and nothing it omits ends anything (§6.6).

3. **`R24` must not run on class search.** The registrar proves the student is enrolled, so its rows
   are confirmed hard with no press. Class search proves only that a section exists and when it meets.
   At the test account 15 LMS shells begin with the current term, more than a normal load, so some are
   not real classes. Auto-confirming them would put hard blocks on a student's week that Knowlu
   invented. Every class-search row is a proposal the student confirms (§6.7).

One smaller fact: the app's `ureq` has no cookie jar (`app/Cargo.toml` says nothing needs one), and
Banner holds the chosen term in an anonymous session. C4 carries that one session cookie by hand, on
the stack, for one run. It is not a sign-in and is never stored.

## 2. What decides outcome A or B

**Outcome A:** one visible sign-in on a fresh folder, then the LMS link and the mailbox are both
silent in a new process. **Outcome B:** two visible sign-ins, the LMS and the mailbox.

| R0a's result on a fresh folder | Outcome | Where the visible sign-in happens |
|---|---|---|
| LMS sign-in alone leaves the LMS link and the mailbox silent | A | the LMS (`A-lms`) |
| Mailbox sign-in alone leaves both silent; LMS-first does not | A | the school mailbox (`A-mail`) |
| Neither alone covers both | B | the LMS at step 3, the mailbox at step 5 |

**The link's lifetime is a separate axis.** If the link dies before 12 hours, A and B still describe
the first run, and the cost lands on grades alone. The feed link is a bearer URL captured once, so
assignments keep arriving. Only grades, and the marks they drive for turned-in work, need the LMS
session. That goes to Quinn as RQ3. It adds no task to either list.

**Why the shared folder is common to both.** The one silent run came after a mail sign-in with MFA
and "stay signed in", on the same folder. The likely carrier is the sign-in host's persistent cookies,
which the mail sign-in wrote. Two separate folders might therefore lose the silent link even in B.
R0a's item (d) tests this directly. Until then this draft recommends one folder in both outcomes
(RQ4), which is why the S tasks sit under "common, on RQ4".

## 3. Checked against VISION and the two rules

- **"What's next?"** Class times become known without typing or a registrar sign-in, so the week and
  free time are honest sooner. Passes.
- **Commitment 1, deterministic engine.** The parse and the name reading are pure engine code with no
  model. Passes.
- **Commitment 4, confirmed commitments.** Nothing lands without the student's press. Passes.
- **Commitment 5, who started it.** Knowlu inferred the section from an LMS name, so this is "Knowlu
  noticed": it proposes and waits. The public list is not treated as an authoritative change.
- **Fetch on device, think in the cloud.** The read runs on the student's computer, anonymously. The
  cloud never fetches it, and nothing reaches a model.
- **Data minimization.** One query per course, public data only, deleted when setup ends.
- **Failures are visible.** Each course ends in a named state (§6.5). A school that closes guest
  search reads as closed, never as "no classes".
- **Rule 1, no single-user assumptions.** Class search, the link prefix and the sign-in place are all
  curated row data, per school. Nothing is per student or hand-edited. Open point: `VISION.md`'s
  source list has no public class search (RQ10).
- **Rule 2, frozen references.** No task touches `engine/tests/fixtures/**`. New tests use invented
  rows inline, as `engine/src/registrar.rs`'s tests already do. No frozen reference changes.

## 4. Tasks

The plan's §2 constraints all carry over: TDD with `todo!` stubs, 0 warnings, about 80 lines per file
per task, tests never leave the machine, no new capability, every live step the controller's.
**Contract list:** the only contract-list file touched in the whole replan is `app/src/account.rs`,
by the plan's existing S5, S7 and F2c. No new task touches the list. `commitments.rs`, `registrar.rs`,
`scaffold.rs`, `lms_link.rs` and `grades.rs` are off it, but C2 and C8 can silently damage vault
meaning or a session, so they take stronger agents or reviewers than the list alone would require.

Task ids are new and do not collide: `R` decision runs, `C` common, `A` outcome A, `B` outcome B.

### 4.1 Decision runs (common; first)

| Task | What | Agent | Why | Contract list? | Depends on |
|---|---|---|---|---|---|
| R0a | Tomorrow's sign-in run (brief in §8) | main session | live; Quinn at the machine; a throwaway build | no | none |
| R0b | The class-search spike: public data, no sign-in (brief in §8) | main session | live reads are the controller's | no | none |
| T3a | S-G alone, split from T3: hidden reads while the student is elsewhere, now through the link | main session | live; S4a-S4c depend on it | no | R0a's build |

### 4.2 Common to both outcomes: can start before the result

Rows C1 to C8 depend on no sign-in result. C1's and C4's exact keys wait on R0b, not on R0a.

| Task | What | Agent | Why | Contract list? | Depends on |
|---|---|---|---|---|---|
| C1 | Engine: `section_in_name`, `parse_class_search`, the `School` row's name shape | `test-writer`, then `implementer` | specified and test-checked | no | R0b |
| C2 | Engine: the `classsearch:` source row, its tier, keep-alive, `classify` arm, and no ask for a listed no-time section | `test-writer`, then `integrator`; `contract-reviewer` reads the diff | a wrong tier silently reverses which source wins; the syllabus plan gave the same edit to `integrator` | no, but vault meaning | C1; RQ1 |
| C3 | Engine: `commitments --class-search`; `onboarding --preview --class-search` | `test-writer`, then `implementer` | specified; both read-only of notes | no | C1, C2 |
| C4 | App: `app/src/class_search.rs` (the fetch) and `Curated.class_search` | `test-writer`, then `implementer` | specified; proven on a local fake server | no | C1, R0b |
| C5 | Queue: `registrar.confirm` out; `school.class_search` and `class_times.apply` in | `test-writer`, then `implementer`; `contract-reviewer` reads the diff | orchestration; the file holds capture state | no | C3, C4; takes F8h-runner's place in the `onboarding_jobs.rs` chain |
| C6 | Page: step 10's class-search rows, the proof line, the job labels | `test-writer` (pins), then `console-ui` | `app/static` | no | C3; RQ2 |
| C7 | Page: the week strip's "N turned in, kept off your list" note (amends §14.58.5) | `test-writer` (pins), then `console-ui` | `app/static`; Quinn's addition | no | none |
| C8 | App: the LMS sign-in link, read per school; hidden and visible windows open at it | `test-writer`, then `implementer`; `contract-reviewer` reads the diff | a wrong host leaks a session (the plan's Q-B6) | no, but sessions | R0a item (e) for the row's data only; lands after S4a if S4a is in flight |
| C9 | Docs: amendment drafts, the curation check, reference pages | `docs-keeper` | docs only | no | C1 to C8 |

**Common, on RQ4 (one folder in both outcomes).** S1, S2, S3, S4a, S4b, S4c, S5, S6t, S6a, S6r, S6b,
S7, S8 and S9, as §4.5 changes them. They need Quinn's word at this checkpoint, because the plan says
no O4 task goes out before the finding reaches Quinn and the planner. If Quinn rules two folders in
outcome B, they become outcome A's alone.

**After the rewrite, common.** **T3b** (main session): S-E and S-F's live half, moved so they time the
chosen flow (§4.6).

### 4.3 Outcome A only

| Task | What | Agent | Why | Contract list? | Depends on |
|---|---|---|---|---|---|
| A1 | Row data: the founder's school's `school_sign_in` as `AtLms` or `AtMailbox`, and `sign_in_hosts` | `implementer` | one data line plus a `CAMPUSES` test; the value cites R0a | no | R0a, S2 |
| A2 | S4d, as A: one visible window at the row's place; the signed-in test for that place; then the hidden reads | `implementer`, reviewed by `contract-reviewer` | as S4d | no | A1, S4a, S4b, C8 |
| A3 | Page: step 3's one sign-in and its "stay signed in" line; step 5 with no further ask | `console-ui` | inside F11e1 and F11b, in the panel merge order; Quinn reads the words | no | A2 |
| A4 | S10 and F15's outcome-A checks: one prompt in all, mail and LMS silent | main session | live; Quinn's go | no | A2, A3, T3b |

### 4.4 Outcome B only

| Task | What | Agent | Why | Contract list? | Depends on |
|---|---|---|---|---|---|
| B1 | Row data: `school_sign_in` as `TwoSignIns`, and `sign_in_hosts` | `implementer` | as A1 | no | R0a, S2 |
| B2 | S4d, as B: the visible LMS sign-in at the link at step 3; step 5's visible mailbox sign-in on the folder RQ4 chose | `implementer`, reviewed by `contract-reviewer` | as S4d | no | B1, S4a, S4b, S4c, C8 |
| B3 | Page: step 3's and step 5's words for two sign-ins; spec §1.3's "You won't sign in to these again" goes | `console-ui` | inside F11e1 and F11b; Quinn reads the words | no | B2 |
| B4 | S10 and F15's outcome-B checks: two prompts at most, later reads silent | main session | live; Quinn's go | no | B2, B3, T3b |

Outcome B also needs a signed note that Q1's "one sign-in" is not met at this school. C9 drafts it.

### 4.5 Existing tasks: dropped, changed, kept

**Status caveat.** The planner has no shell. Merge state below comes from reading this branch's code:
`app/src/onboarding_live.rs` does not exist, `set_school` and `write_capture_file_for_gen` do not
exist, `lms_link::validate_only` and `capture_link_only` are still `todo!` stubs, and no
`school_session.rs` exists. The controller re-checks before the first dispatch.

**Dropped (2).**

| Task | Why |
|---|---|
| F8h-runner | `registrar.confirm` leaves onboarding, so its runner and `registrar_file_missing` have no job |
| F8b's registrar half | Onboarding no longer captures registrar rows. `REGISTRAR_FILE` and `onboarding-registrar.json`, which F2b2b's comment reserves, are never built; C5 removes the comment |

**Changed (19).**

| Task | Change |
|---|---|
| T3 | Split: T3a (S-G) stays early; T3b (S-E, S-F's live half) moves after the rewrite (§4.6) |
| S1 | `add_use(Registrar)` goes; the uses are grades and school mail |
| S2 | `shared_sign_in` becomes `school_sign_in`, an enum (`Separate`, `AtLms`, `AtMailbox`, `TwoSignIns`), every row `Separate` until A1 or B1 |
| S3 | The registrar leaves tests 1, 3, 5 and 8; the fake-agent host pairs drop it. The navigation allow list still refuses the registrar host |
| S4b | `lms_link.rs` only. `registrar.rs` stays on its incognito throwaway window, so "the registrar never shares this profile" stays true |
| S4d | Replaced by A2 or B2 |
| S6b | The row reads "used for grades and school email" (spec §2.5 said "grades, class times and school email") |
| S7 | Quinn's verbatim line of 2026-10-24 no longer fits; see RQ6 for the replacement |
| S8 | The plan's §9 row A6 is withdrawn; rows A1 and A5 lose the registrar |
| S10 | The registrar read leaves it. A4 or B4 holds the prompt count |
| F8b | Capture half only: one hidden window through the link does the feed, the course list and the grades before it closes, because the session dies with the last window on the folder (§7.3). No `registrar_rows` |
| F8b2 | The capture files it fences are `onboarding-class-search.json`, not the registrar file |
| F8w1 | No `registrar.confirm` arm, so no interim `registrar_runner_pending`. It gains the two class-search arms |
| F11e1 | Step 3's sign-in follows A3 or B3 |
| F11e2 | The proof's course list marks a course that was not found in class search with a "doesn't look like a class" hint. The student still decides (§6.4) |
| F11b | Step 5 follows A3 or B3 |
| F12 | The walk-through drops its registrar fixture and adds class-search rows, and one run with no class search at all |
| F13 | The plan's §9 rows change as C9 lists them |
| F15 | `finish_ready: true` on the build to merge, once with class-search rows and once with class search failing or absent, and never by a registrar read |

The plan's §8 O3-to-`main` checklist changes with F15: its `registrar_runner_pending` search and its "registrar
gave class times" item go. In their place: `app/src` holds no `registrar.confirm`, and F15's two runs
passed.

**Kept, not altered.** S4a, S4c, S5, S6t, S6a, S6r, S9, F2c, F3z, F4c, F8la, F8lb, F8b2-advance, F8g,
F8w2, F11d, F11f and F14. Every merged task stands. F7a's `--registrar` input stays in the engine: it
is small, tested and documented, and it does no harm unused. C8 also edits `grades.rs`, so S4a and C8
are a collision note: the second to land rebases.

### 4.6 The timing test moves (Quinn, 2026-11-04)

S-E and S-F's live half run **after the rewrite**, as T3b, on the chosen flow and before F15. Timing
the old flow would measure a sign-in path that no longer exists. Moving them changes four things:

- **T3 cannot move whole.** S4a, S4b and S4c depend on T3 for S-G's result, so S-G stays early as T3a.
- **S-F rides with S-E.** The plan runs them in one session and S-F's check needs a first full slot.
  This is the planner's call, not Quinn's words. The cost is that a thin step-3 proof shows up late.
- **S-E's gates become checks on built code.** Its "before F8f" gate is past, since F8f is merged. A
  stage over 60 seconds, or a wait past Q7's 3 minutes, now goes to Quinn as a change to built work.
- **Every number here is provisional until T3b:** the class-search budget (§6.8), and the copy's minutes.

T3b also records class search's seconds, and the step-3 wait it adds.

### 4.7 Order

1. R0a, R0b and T3a. In parallel, C7 and the test halves of C1 and C8.
2. C1, C2, C3 and C4 in that chain, C8 beside them. S1 to S4c once RQ4 is ruled.
3. C5 in `onboarding_jobs.rs`'s chain, in F8h-runner's old place. C6 and C7 in the panel merge order,
   before F11b. After the F11 screens: A or B.
4. T3b, then A4 or B4, then F14 and the plan's §8 checklist. **No wave-B release before S7's page is deployed**,
   as the plan already rules.

## 5. The registrar is optional in both outcomes

**What stays on `main` as a bonus, unchanged:**
- `app/src/registrar.rs`, `engine/src/registrar.rs`, `commitments --registrar`, R24, and applied-and-listed.
- The Schedule view's and first-day screen's "Get my class times" button, where a curated row has a
  registrar. It keeps its own incognito window and its own sign-in.
- The registrar's top tier. A later press outranks class search and corrects it (§6.6).

**What onboarding stops requiring:**
- Step 3 never opens or reads the registrar. `school.capture` carries no registrar call.
- `registrar.confirm` leaves `JOB_ORDER`, `LANE`, `NOTE_JOBS`, `LANE_REQUIRED` and `inputs_ready`.
  `finish_refusal` no longer waits on it, so no registrar state can hold Open Knowlu.
- `class_times` in step 3's proof may be `null`, and the proof omits it. Step 10 still opens.
- S-A's pass no longer names the registrar. S-A phase 2's table row for it is history.
- F15 passes without any registrar read.

**A known gap, not fixed here.** The curated `start` page is public and never redirects, so the bonus
button needs the student to click through to the schedule before pressing "I'm signed in". Pointing
`start` past the menu is a one-line data change, and it is a follow-up (§9).

## 6. The public class-search reader

**What it is for.** A student at a curated school gets class times proposed at step 10 with no
registrar sign-in. **What it is not:** a general answer. Banner is about 24% of the North American
higher-ed student-system market (ListedTech, January 2025; an analyst blog, one secondary source, not
checked here). Whether guest class search is open also looks set per school. So most students will
never have this reader, and the calendar, syllabus and typing paths must stand on their own.

### 6.1 Where each part lives

| Part | Where | Why |
|---|---|---|
| Reading a course name; parsing rows | `engine/src/registrar.rs`, beside `parse_banner` | ruling R1-a keeps every Banner key in one place; pure, no network |
| The fetch | new `app/src/class_search.rs` | "the app carries the bytes", as with the registrar |
| The school's data | `Curated.class_search` in `scaffold.rs`; the term table in `registrar::SCHOOLS` | rule 1: a row, never a per-student setting |
| Writing proposals | `commitments --class-search` | one engine command, notes untouched |
| Confirming | today's `commitments --confirm` from step 10 | the student's press, journal first, no new write path |

The fetch uses plain `ureq`, with no window and no capability. It sends no student cookie, because
nothing here needs a sign-in. A test pins that the reader never touches a window or a kept profile.

### 6.2 The fetch

Per run, in order:
1. `POST term/search?mode=search` with the term, once. Keep the one anonymous session cookie from the
   answer on the stack. No jar, and no change to `ureq`'s features.
2. For each distinct `(subject, number)`: `POST classSearch/resetDataForm`, then `GET
   searchResults/searchResults` with `txt_term`, `txt_subject`, `txt_courseNumber`. R0b records
   whether the reset is truly needed; it stays either way, since the sample only ran with it.
3. Write one bundle: `{school, term, queries: [{subject, number, sections, status, body}]}`, with
   `sections` the sections wanted and `body` the raw answer. No course name, no student id.

**Paging is unrecorded.** R0b checks a course with many sections. If answers are paged, C4 reads the
answer's count and follows pages for that one course only, at most 3. A course whose section could sit
on an unread page ends `not_found` only after the pages are read; otherwise it is `fetch_failed`.

### 6.3 How a course name becomes a query

`section_in_name(name, shape) -> Option<SectionRef{term, subject, number, section}>`, a pure function.
The shape is one named variant on the `School` row (`TermSubjectNumberSection` is the only one today).
A new school's naming is one new variant. Rules, in the style of `course_code_in_name`:
- Six-digit term, then subject (two to four capitals), three-digit number with an optional trailing
  letter, then a section of one to four letters or digits, dash-separated: `209940-XYZ-100-001`.
- The run must start the name and end at a boundary, so trailing words are allowed. A further dash
  group means a merged shell (`209940-XYZ-100-001-002`) and **does not fit**. It is never cut down.
- Never a guess: anything else is `None`. `suggest_course` and `course_code_in_name` are not used here.

The school's LMS names decide this, and LMS **codes** never carried the shape at the test account, so
only the name is read. Today's sample: 14 of 19 names fit.

### 6.4 What happens to each kind of course

Each kept course ends in exactly one state. The vocabulary is closed, and the preview counts by state.

| State | Meaning | What the student sees |
|---|---|---|
| `found` | one section matched and was read whole | a proposal row at step 10 |
| `no_time` | one section matched, listed with no day or time | "No set meeting time listed" at step 10, no proposal, no later ask (RQ9) |
| `part_read` | matched, but a meeting could not be read (`Reading::Partial`) | no proposal; the calendar and typing paths as today |
| `not_found` | the query answered, no such section | no proposal; a hint at step 3's list |
| `ambiguous` | two or more rows match subject, number and section | no proposal: never picked |
| `other_term` | the name's term is not the term being read | no query, no proposal |
| `no_fit` | the name does not fit the shape | no query; the calendar, syllabus and typing paths as today |
| `fetch_failed`, `paused`, `closed` | a network failure, a stop for politeness, guest search closed | no proposal; one named line at step 10; never a failed job |

- **Sections with no time.** Many have none, and that is an answer, not a fault. `parse_banner`'s P4
  rule holds: nothing is filled in. A read of only no-time rows is not an "empty parse" failure,
  unlike the registrar's rule. An answer with no rows at all stays `not_found`.
- **Non-class shells.** A shell that fits the shape but has no section is `not_found`, and step 3's
  removable list shows the hint. Knowlu never removes a course on this evidence, since hiding a real
  class is worse than offering an old one (`lms_link.rs`'s rule). A shell that **does** match a real
  section, such as a dropped class still in the LMS, cannot be told apart. That is why §6.7 asks.
- **Wrong term.** The term read is `registrar::fetch_term`'s. A name with another term is
  `other_term`. A returned row whose `term` differs from the one asked, or whose `until` is before
  today, is dropped and counted. The summer gap in the `terms` table stays, as the phase-3 spec names
  it: a summer student gets `other_term`, not a wrong fall schedule.
- **Cross-listed sections.** Only the subject and number the name carries are asked about, so a
  cross-listed twin never appears. Two LMS courses naming one CRN are one series, and `by_key` keeps
  one record. A twin under a calendar series with the same meeting signature collapses, as today.
- **Linked sections (a lecture with a lab or recitation).** The name gives one section, and the linked
  one cannot be known from public data. It is never guessed. If R0b finds a link field, a `found` row
  whose section is linked carries `linked: true`, and step 10 shows "This class may also have a lab
  or recitation. Add it if so." A separate LMS shell for the lab is read as its own course.
- **Lab or class.** The title rule is `parse_banner`'s (P5). R0b records which of the kind fields this
  answer carries, since the registration call's `scheduleDescription` may not be there.
- **Reader-side checks.** A row is matched only when the returned subject, number and term equal the
  query's and `sequenceNumber` equals the section. The answer is never trusted to be filtered.

### 6.5 What a run reports

`commitments --class-search` prints the counts per state, `term`, and `warnings`. **It always exits 0
and writes only `state/calendar-series.json`**, so it fits `CLAUDE.md`'s `commitments` line and needs no
rule change. An unreadable bundle is a named line with nothing written. It holds the series lock, is
never gated, fetches nothing, makes no model call and writes no journal record.

`onboarding --preview --class-search <file>` fills the existing `class_times` key and adds
`class_search: {found, no_time, ...}`. It never writes, reaches no network and exits 0.

### 6.6 Where class search sits under the existing precedence

**Recommended order: registrar, class search, syllabus, Google, the rest. This is Quinn's call (RQ1).**
The syllabus spec fixed registrar above syllabus above calendar, and class search is new to that
order. For this place: it is the school's live section record, where a syllabus is a model's reading of
a document that can be out of date. Against: Knowlu inferred the section, and a syllabus belongs to the
student's own course. The student's confirmation covers that weakness either way.

**Under that order,** `SOURCES` gains a row at tier 1, `google:` becomes 2 and the rest 3. L5 has not
landed, so its `syllabus:` row becomes tier 2 and those below it move down one: a spec E4 amendment
(C9). The row:

| Column | Value | Why |
|---|---|---|
| `calendar` | `classsearch:` (`classsearch:<key>:<term>`) | its own tier and keep-alive |
| `uid` | `Some("classsearch:")`, matching nothing | `None` is the catch-all in `by_uid`, so `None` here would swallow every later row |
| `keep_alive` | a new arm | a read covers only the courses asked about, so an omitted row stays; it ages by `until`, as `InPlay` does |
| `instances` | false | a weekly rule only |
| `holds_per_kind` | false | an unconfirmed series must not hide a calendar proposal, because it may not be the student's class |
| `lands` | by uid (below) | |

**One key for one section.** A series carries `source_uid` `registrar:<key>:<term>-<crn>`, under the
`classsearch:` calendar. The CRN names the section however Knowlu read it, and `by_key` exists for one
key under two calendars. What this buys, read from the code:
- A later registrar press finds the confirmed note and treats it as its own. It applies a changed
  time, and it ends a section the student is not registered in, which is right, because that answer
  **is** the whole term. A second note can never be made for one section.
- A confirmed note holds its course's kind against calendar proposals, through the `registrar:` uid.
- The note's body already reads "Found in your school's class schedule."

**The cost.** A `registrar:` uid no longer proves the registrar read it. `Lands` follows the uid, so
class search itself can never change a confirmed note, which matches §6.9. C2 walks every use of the
prefix and pins each: `registrar_proposals` in the overview, `withdrawal_reason`, `detect_changes` and
`registrar_in_play`. **The alternative** is a `classsearch:` uid: a cleaner table, but a later
registrar read cannot recognise the note, and differing times would leave two hard notes for one class.

`classify` takes the registrar's no-eligibility branch for this calendar too, because a series with no
instances would otherwise fail `eligible`. `asks_wait_for_registrar` does not count it.

### 6.7 How a row becomes a confirmed commitment

1. `class_times.apply` runs `commitments --class-search` after `school.courses`, so the code table
   holds the student's courses. Only `found` rows that name a kept course become series.
2. Step 10 shows them through `commitment_proposals`, beside any calendar proposal, each with Mine, Not
   mine and the firmness control F11p3 built.
3. The student's press writes through `commitments --confirm`: the student's token, journal first.
   Default level is the class kind's. **Nothing is written before the press** (RQ2).
4. "Not mine" is a decline marker and holds. A left-blank row returns as today's asks.

So `R24` does not run, no card is charged at step 10, and the 15-a-day cap is untouched. Confirmed notes
and decline markers sync; the series file and the bundle never leave the device.

### 6.8 Rate limits and politeness

- **One query per distinct `(subject, number)`**, with the reset before it. Two sections of one course
  share a query.
- **No bulk download, by construction.** The query type cannot be built without a subject and a number,
  and a test pins that. The term's whole list is never asked for.
- **Serial, with a gap** of at least 1 second between requests. No parallel requests, from one device.
- **Caps:** 20 distinct queries and 60 seconds a run. Past either, the rest are `paused`. The cap
  exists to stop a bug or an odd enrolment; the test account's 15 current-term shells fit under it.
  T3b sets the final numbers.
- **Stop at the first refusal.** A 429, a 403 or a 5xx ends the run: the rest are `paused`, with no
  retry and no backoff loop. An answer that is not JSON, or a redirect to a sign-in, ends it as `closed`.
- **Honest identity:** a `user-agent` naming Knowlu, its version and `knowlu.com`.
- **Once per setup.** Try again is the student's press, at most one, after any `Retry-After`.
- **Each student's own computer.** The cloud never relays or caches it, so no shared list of a school's
  sections is built.

**A risk this leaves.** Removing a school means a new release, because the row ships in the app. A
remote off-switch is a later question if a school ever objects.

### 6.9 After onboarding

Wave B reads class search during setup and nowhere else: no slot step, no scheduler entry. A confirmed
time can go stale if a section moves mid-term, and a registrar press corrects it where the school has
one. Whether it re-reads at a new term is RQ8.

### 6.10 How a school opts in (rule 1)

`Curated.class_search: Option<ClassSearch>`, `None` on every row by default.

```rust
pub struct ClassSearch {
    pub school: &'static str,  // the key in registrar::SCHOOLS, which holds the term table and name shape
    pub host: &'static str,
    pub prefix: &'static str,
    pub checked: &'static str, // YYYY-MM-DD: the curation check below passed on this date
}
```

It stands apart from `Registrar`, so a school can have class search with no registrar row. A class-search
school needs a `SCHOOLS` entry with `registered: &[]`.

**Guest search being open is a per-school fact, so it is curated, never detected.** Knowing a school
runs Banner proves nothing. There is no runtime probing of uncurated schools, and no inference from
`lms_kind` or the mail provider. A row reaches `main` only after the **curation check**, run by the
controller, with no sign-in, and recorded by date in the row:
1. The term POST and one search answer JSON with no sign-in.
2. A sample of sections, found by subject, number and section, each match one row.
3. A sample of that school's real LMS names, or its published naming, fit a shape on the `School` row.
4. The school's `robots.txt` and any terms for the page were read (RQ7).

A test over `CAMPUSES` pins the structure offline: `school` is a `SCHOOLS` key, `checked` is a date,
the host carries no scheme or path, and `prefix` starts and ends with `/`. No test reaches the school.
If a school closes guest search later, the run reads `closed`, and setup goes on without class times.
Adding a school is one row: no vault, no student and no credential is involved.

### 6.11 The queue

Two jobs replace `registrar.confirm`:
- **`school.class_search`**, serial, in `UNGATED` (public data, before payment), after `school.capture`
  and before `school.preview`, so the proof can count class times. It runs only when the row has
  `class_search`, and it writes the bundle to `onboarding-class-search.json` through F8b2's fenced
  writer. It always ends `Done`: any failure is a state in the bundle, never a failed job.
- **`class_times.apply`**, local lane, after `school.apply` and `school.courses`. It runs the command,
  deletes the bundle, and always ends `Done`, with a named note when the engine could not read it.
  It is not in `LANE_REQUIRED`, so it cannot refuse Open Knowlu.

The bundle is also deleted by `change_school`, at finish and at sign-out, which are the deletes the
plan gave the registrar's file. The student's `onboarding.json` state version stays 1: no released
build has written `registrar.confirm`.

## 7. The LMS sign-in link

### 7.1 How the link is obtained

**The row names only a prefix. The link itself is read fresh, from the public login page.**
- `Curated.lms_sign_in: Option<&'static str>` holds the path prefix (`/auth-saml/saml/login`), `None`
  by default. It is per school because a school may use another sign-in, and no tenant is assumed.
- Each time a window needs it: one anonymous GET of the LMS host's login page, with no cookie, then a
  pure `lms_link::sign_in_link(html, lms_host, prefix) -> Option<String>`.
- The link is the same for every student at a school. It holds no user data and is never stored, so
  nothing per-student exists to hand-edit. A tenant changing its identity provider id breaks nothing.

**The rules of `sign_in_link`:** HTML entities in `href` decoded; absolute on exactly `lms_host`, or
root-relative; `https` only; the path starts with the prefix. Refused: another host, a look-alike
suffix, `//host`, userinfo (`@`), a backslash. **Exactly one distinct link passes, or the answer is
`None`.** Two is a page Knowlu does not understand, and it never guesses.

**Unknown until R0a's item (e):** whether a plain GET sees the link, or only a rendered page does. The
record says "readable with no cookies", not how it was read. If only the rendered page has it, the row
pins the whole link instead, and a changed tenant shows up as signed out. This is the one design point
R0a can overturn.

### 7.2 Where it is used

- **Hidden:** `grades::open_hidden` starts at the link, and at `https://<host>/ultra/` as today when the
  row has no prefix or the link was `None`. `read_hidden` is unchanged.
- **Visible:** a visible LMS sign-in opens at the link too, so the student skips the login page. That
  may also avoid the password ask that R0a's item (b) is chasing.
- **Everything else stands:** no capability, cookies read per host, and `users/me` read only on the LMS
  host. A test pins that the anonymous GET sends no `cookie` header.

### 7.3 One window's life

The LMS session dies when the last window on the folder closes, so F8b does the feed link, the course
list and the grades capture in **one** hidden window before it closes. After a close, the next open
waits the measured 250 ms (S-B's step 4), and `school_session`'s lock holds it for that long.

### 7.4 When a hidden refresh fails

No new state. It is today's signed-out path, reached from a new start page.

| What happens | Named outcome | What follows |
|---|---|---|
| `users/me` answers 200 within `SETTLE` (45 s) | capture | the mark clears |
| The window sits on the identity provider, never reaching the LMS | `SignedOut` | `grades-signed-out` set; row `grades (skipped: signed out)` |
| It reaches the LMS and `users/me` answers 401 or 403 | `SignedOut` | as above |
| No answer, or a 5xx | `Unreachable` | row `grades (skipped: Blackboard unreachable)`; no mark |
| The login page cannot be fetched, or `sign_in_link` is `None` | falls back to `/ultra/`, then as above | a changed page reads as signed out, visibly |

Always: the hidden window is destroyed, never shown, and nothing is typed. One try per slot, so a
long-dead session costs one window a slot and never hammers the identity provider. The row stays exit
0, so the tray stays green. The student sees the sign-in row in Settings and signs in there.

**Inside onboarding** the same failure right after a visible sign-in means the session did not carry.
`school.capture` then shows one visible window at the link, rather than sending the student to the
paste fallback. If that is closed or fails, step 3's feed-link paste stands, with its named cost.

## 8. Task briefs

Behaviour and tests only. Every test uses invented names, sections and hosts. Each `test-writer` half
leaves its tests compiling and failing at run time.

### R0a. The sign-in run (main session, Quinn at the machine)

The plan's §3 rules apply: a throwaway build, no debug port, counts and host kinds only, folders deleted.
Each item runs on a **fresh** folder unless it says otherwise.
- **(a) Lifetime.** Sign in, then a hidden link read at 12 hours or more in a new process. Add 24 hours
  if the clock allows. Record silent or not, and seconds.
- **(b) The password ask.** Repeat a click-through and an open at the link, each after a close. Record
  which surface asked, and what had closed before it.
- **(c) One sign-in.** Sign in at the LMS link only, then the mailbox hidden. On a second folder, sign
  in at the mailbox only, then the LMS link hidden. This decides §2's table.
- **(d) What carries it.** LMS sign-in alone, with no mailbox ever opened, then a new process and the
  link hidden. This decides RQ4, and it is what a student who skips school email gets.
- **(e) The link.** A plain anonymous GET of the login page: does the raw HTML hold exactly one link
  with the prefix? Does `/` redirect first?
- **(f) "Stay signed in" declined,** if time allows: the same as (a) at a shorter gap.

### R0b. The class-search spike (main session, no sign-in)

Public pages only. No student's courses: use a few sections from the public list itself. Record keys,
counts, seconds and status codes. Record: the rows' container; whether `term`, `subject`,
`courseNumber` and `sequenceNumber` are on each row; which kind fields exist; whether the session
cookie is required; whether a search needs `resetDataForm`; paging on a many-section course; any link
or cross-list field; seconds per query; whether a plain `ureq`-style request is answered. Stay under
§6.8's limits. Then run §6.10's curation check for the founder's school.

### C1. Engine: the name and the parse

Tests: `section_in_name` over the fitting form, trailing words, a merged shell, four-digit numbers, a
trailing letter, lower case, no term, and a name with the code mid-string, each with its expected
`Some` or `None`. `parse_class_search` over inline rows: one section matched among many, two rows for
one section is `ambiguous`, a wrong `term` or subject is dropped, `EXAM` skipped, a no-time row is
`no_time`, a part-read row is `part_read`, an all-no-time course is no failure, a meeting across
midnight is dropped, and a school with a non-empty `registered` set still reads rows that carry no
status. `p2_the_meetings_faculty_wrapper_is_not_read` stays, unedited and green.
Build: `School` gains the name shape; `parse_class_search` shares `read_meetings`'s rules, by lifting
each row's meetings into the flat form. Output is sorted, so the same bundle gives the same bytes.

### C2. Engine: the source row

Tests pin orders, never numbers, as the syllabus plan's T4t does: registrar before class search before
Google before the rest, in all four readers of `precedence`. Then: a class-search row and a Google row
with one signature collapse to class search; a series alone holds no Google proposal; a confirmed note
holds it; a later registrar fetch of the same CRN applies a change and makes no second note; a registrar
answer missing the CRN ends the note; a read omitting a course leaves its series and ends nothing; an
unfetched `classsearch:` calendar stays while its `until` is open; `by_uid` of a `gcal-series:` and an
`ics-series:` key is unchanged; a listed no-time course is not `uncovered`. Each use of the
`registrar:` prefix gets one line in the report: unchanged, or changed and why.

### C3. Engine: the two commands

Tests: `--class-search` writes the series file only, with no note, card or journal record; a bad bundle
exits 0 with a named line and nothing written; it conflicts with `--confirm` and `--registrar`; an
unknown `--school` is a named line. `--preview --class-search` fills `class_times` and the state
counts, with `null` for an unreadable file, and writes nothing. The `surface_oracle` references stay.

### C4. App: the fetch

`app/tests/class_search.rs`, on a fake Banner bound to `127.0.0.1:0`. Tests: the call order, with reset
before each search; the anonymous cookie carried, and no other; one query for two sections of a course;
subject and number in every search; the 1-second gap on an injected clock; the query and time caps;
paging stopped at 3; 429 and 403 stop the run; HTML gives `closed`; no store of a cookie in any file;
the bundle holds no course name; and a `CAMPUSES` test (§6.10). Errors use fixed words, as
`registrar::fetch_error` does. The founder's row gets `class_search` only after R0b's curation check.

### C5. Queue

Tests first, through the existing fakes: `registrar.confirm` is absent from every job list; the page
and `finish_refusal` never wait on it; `school.class_search` runs between capture and preview, and is
skipped with no row; it ends `Done` on every failure; `class_times.apply` waits for `school.courses`;
a failed read leaves Open Knowlu offered; `change_school`, finish and sign-out delete the bundle; a
stale generation never writes it. Removes `registrar_confirm_rows`, `RegistrarRow` and
`Answers.registrar_rows` (a serde default keeps any dev file readable). Existing tests naming the job
change with it; each is listed in the report with why. No frozen or contract test is among them.

### C6. Page: step 10

Pins, then the page. `summaryLine10` stops saying "set from your school's registrar": the words become
Quinn's, and until then "N class times found at your school. Check them below." Class-search rows show
as proposals with no answer pre-pressed. `no_time` shows its line, and a `closed`, `paused` or
`fetch_failed` state shows one line with no error styling. The labels for `registrar.confirm` go, with
`w10.reg`'s reads, in favour of `class_times.apply`. Acceptance: side-by-side shots.

### C7. Page: the week strip's turned-in note (amends §14.58.5)

Quinn's addition, common to both outcomes. The plan's §14.58.5 gains one bullet:
- **The note.** Under the days, "N turned in, kept off your list", with N exactly `step3.turned_in`.
- **When it shows:** only while the strip shows, and only when the field is a number of at least 1.
  `0`, `null` or absent shows nothing, with no placeholder. `null` means no grades bundle, so the count
  is unknown, not zero.
- **Rules kept:** the page sums nothing, the figure is the field, and it is a count with no titles.
  `turned_in` is already totalled over kept courses, so removing a course at step 3 lowers it.

Pins in `app/tests/onboarding_static.rs`, over `weekStripModel`: 3, 1, 0, `null`, absent, hidden steps,
and a fixture before and after a removal. About 8 lines in `onboarding.js` and 3 in `onboarding.css`.
The prototype has no such note, so the reviewer treats it as a ruled departure in the shots.

**The promise has to be true.** O1 archives Blackboard's submitted work at first sight only if the first
slot's grades capture ran. If that hidden read is signed out, those items arrive live and are marked
done a slot later, so the note would be briefly false. This is one more reason for §7.3's single window.
T3b's S-F check covers the number: N equals what the vault then holds as turned-in LMS work, or differs
by a stated reason. It is not the reveal's whole `imported_done`, which also counts homework sites.

### C8. App: the link

Tests over `sign_in_link`: one link; none; two; an entity-encoded query; root-relative; another host; a
suffix look-alike; `http`; `//host`; `@`; a backslash. `start_url` falls back to `/ultra/` on `None`. The
GET sends no cookie, on a local listener. Hidden failures keep §7.4's table, through `read_hidden`'s
existing fakes, and open no visible window. `Curated` gains `lms_sign_in`, `None` on every row until
R0a's item (e) passes.

### C9. Docs

`docs-keeper` drafts, for Quinn's signature: spec §1.3 step 3 and §2.4's uses; a dated line retiring spec
§4.2 item 1; the plan's §9 row A6 withdrawn and rows A1, A3 and A5 narrowed; the syllabus spec's E4 tiers; the plan's
§14.58.5 bullet; and, on outcome B, the note on Q1. Also the curation check in `docs/reference/app.md`,
`commitments --class-search` in `docs/reference/engine-commands.md`, and app data's new file.

## 9. Follow-ups, not in wave B

- **A 12-hour clock for class times (Quinn, 2026-11-04).** Its own task after onboarding, across every
  surface that shows a time. C6 keeps today's time format, so wave B neither fixes nor worsens this.
- The bonus registrar button's `start` page, pointed past the public menu (§5).
- Class search at a new term, if RQ8 says so; a name with no section, when a course has only one
  section; a remote off-switch for a school's row (§6.8).
- Other student-system class searches. Banner is a minority of schools, so wider coverage needs other
  readers, each its own spec.

## 10. Open questions for Quinn

One at a time, each with a recommendation. RQ1, RQ2 and RQ4 hold work back; the rest can wait for
their tasks.

- **RQ1. Where does class search sit in the precedence? Recommended: below the registrar, above the
  syllabus.** It is the school's own record of the section. A syllabus is a model's reading of a
  document that may be out of date, and every row is confirmed by the student anyway. The other order
  is safer if you weigh Knowlu's guess at the section more heavily. Holds C2, and it amends the syllabus
  spec's E4 tiers.
- **RQ2. Are class-search rows ever written without a press? Recommended: never.** With 15 current-term
  shells at the test account, some are not classes, and a dropped class cannot be told from a real one
  in public data. Each row needs Mine, as calendar proposals do. A single "these are all mine" press
  would keep it to one action, and that is a words-and-layout choice for you. Holds C6.
- **RQ3. If the link dies before 12 hours, what happens to grades? Recommended: keep one hidden try a
  slot, show the named signed-out row, and let the student sign in from Settings.** That could mean a
  sign-in most days, which may be too much to ask. If so, the alternatives are to refresh grades only
  when the student signs in, or to keep a hidden window open between slots. I do not recommend the
  second: it costs memory all day and keeps a live session running unseen. Assignments are not at risk
  either way. Answer after R0a.
- **RQ4. In outcome B, one folder or two? Recommended: one, unless R0a's item (d) shows LMS-only is
  silent by itself.** The only silent run was on a folder that also held the mail sign-in. Two folders
  would keep "Disconnect" simple but could lose the silent link. Holds S1 to S4c.
- **RQ5. If both places work in outcome A, where does the sign-in happen? Recommended: the LMS.** A
  student who will skip school email should not have to sign in to their mailbox. If only the mailbox
  works, step 3's words must say why it opens there, and ask the student to accept "stay signed in".
- **RQ6. Can the privacy line change? Recommended: yes, to the text below.** You approved the 2026-10-24
  line verbatim, and it speaks of "the class list your school sends". Class search is new behaviour: the
  student's computer asks the school's public page about their courses.
  > "At schools with a public class search, Knowlu looks up your classes there from this computer,
  > without signing in. The results stay on this computer and are deleted when setup finishes or you
  > sign out. The class times you confirm become part of your plan, which syncs to your Knowlu account."

  The Settings row also becomes "used for grades and school email". Holds S7, so it holds the release.
- **RQ7. What must be read before a school's row ships? Recommended: the page's `robots.txt` and any
  posted terms, dated in the row, with the lawyer's read added to the list already going to them.** This
  is an automated read of a public page, kept small. It is not the sign-in question that A13 suspended,
  and A13 does not obviously cover it. The founder's school's check is due before C4's row lands.
- **RQ8. Does class search run again after setup? Recommended: not in wave B.** A new term would then
  need the student's calendar, syllabus or the registrar button. A press at a new term is the natural
  next step, once students have used this.
- **RQ9. Should a section listed with no time skip the later ask? Recommended: yes.** Many sections have
  no time, and asking "when does this meet?" about an online class is noise. The cost: a section whose
  time is simply not posted yet gets no ask. The student can still add it, and the ask card's own
  words already allow for online classes.
- **RQ10. Does `VISION.md` name this source? Recommended: yes, one line under "Where it gets its
  information":** "A school's public class search, where a school offers one, on the student's computer
  and with no sign-in." `VISION.md` is signed text, so it needs your signature.

## 11. Fidelity ledger

| Source | Requirement | Task | Proof |
|---|---|---|---|
| Quinn, 2026-11-04 | The registrar is optional; onboarding does not depend on its sign-in | C5, F8b, F8w1, F15 | C5's tests; F15's run with no registrar read; the plan's §8 checklist item |
| Quinn, 2026-11-04 | What stays as a bonus is unchanged | none: no edit | `registrar.rs` absent from every diff but C1's new functions; existing registrar tests unedited |
| Quinn's additions | The week strip's "N turned in, kept off your list", from `step3.turned_in`, in both outcomes | C7 | C7's pins; shots; T3b's count |
| Quinn's additions | A 12-hour clock is a follow-up, not wave B | §9 | no wave-B task changes a time format |
| Quinn's additions | The timing test runs after the rewrite, on the chosen flow | T3b | §4.6; F15 depends on T3b |
| Quinn's additions | Class search is a per-school curated fact, confirmed by a check at curation | C4, C9, R0b | `CAMPUSES` test; `checked` date; §6.10's check in `docs/reference/app.md` |
| Live run | Session cookies die with the last window; the link signs in hidden | C8, F8b, S4a | C8's tests; R0a; A4 or B4 |
| Live run | The registrar's start page never redirects | §5 | follow-up, §9 |
| Live run | Mailbox silent after one MFA sign-in | S4c, A2 or B2 | R0a; A4 or B4 |
| Live run | Many sections have no begin time | C1, C2, C6 | C1's and C2's no-time tests |
| Live run | 15 current-term shells: some not classes | C6, F11e2 | every row needs a press; hint at step 3 |
| Spec §1.3 | Step 3 asks one sign-in | A2, A3; on outcome B, B2, B3 and a signed note | A4 or B4's prompt count |
| Spec §1.5, Q7 | Reveal waits at most 3 minutes; failure is named | C5 | `class_times.apply` never holds the gate; T3b |
| Spec §1.7, Q12 | Skips and costs unchanged | F11e1, F11b | F11t's pins |
| Spec §2.4 | Cookies per host; no capability; scripts per window | C8, S3, S4a | S3's tests; C8's no-cookie test |
| Spec §2.5, Q9 | One Forget; last use deletes; sign-out always | S2, S5, S6a | the plan's S1, S5 and S6t tests, minus the registrar use |
| Spec §4.2 item 1 | Retired by the ruling | C9 | Quinn's signature on the dated line |
| Spec §4.2 item 5 | Typing stays the last resort | C6 | no typing added; existing asks stand |
| Syllabus spec §4.4, E4 | Precedence through the per-source table | C2 | C2's order tests; RQ1 |
| `VISION.md` commitment 4, 5 | Knowlu noticed: it proposes and waits; the student confirms | C5, C6 | `--class-search` writes no note (C3); RQ2 |
| `VISION.md` data minimization | One query a course, no bulk read, deleted after setup | C4, C5 | C4's cap tests; C5's delete tests |
| `VISION.md` failures visible | Each course ends in a named state | C1, C3, C6 | C3's report; C6's lines |
| `CLAUDE.md` rule 1 | Per-school data, nothing per student | C4, C8, A1, B1 | `CAMPUSES` tests |
| `CLAUDE.md` rule 2 | No frozen reference, no fixture touched | every task | F14 checks `engine/tests/fixtures/**` is absent from the diff |
| `CLAUDE.md` `commitments` line | Without `--confirm`, exit 0 and only the series file | C3 | C3's tests |
| `CLAUDE.md` tests | Tests never leave the machine | C4, C8 | `127.0.0.1:0` fakes |
| `CLAUDE.md` contract list | Only `account.rs`, by `contract-engineer` | S5, S7, F2c | S9; F14 |

## 12. Checkpoints

1. **Quinn reads this draft** and rules RQ1, RQ2 and RQ4. RQ6 and RQ7 are due before S7 and C4's row.
2. **`reviewer`, plan mode,** on this draft before any C or S task goes out; its report lands in
   `docs/reports/`. Most useful targets: §1's premises and §6.6's shared key.
3. **R0a's result** picks A or B and feeds RQ3 and RQ5. R0a's item (e) may change §7.1.
4. The rulings fold into the plan's §14, and the plan's §7 table gains these rows, by `docs-keeper` on the
   controller's word.
5. After the build: T3b, then Quinn's go for A4 or B4, then F14 and the plan's §8 checklist.


## Rulings, Quinn, 2026-11-04

- **RQ1:** class search sits below the registrar and above the syllabus.
- **RQ2:** a class-search row is never written without the student's press.
- **RQ8:** class search runs at setup only in wave B.
- **RQ9:** a section listed with no meeting time skips the later "when does this meet?" ask.
- **RQ6:** the replacement privacy text in §10 is approved as written, and the Settings row reads "used for grades and school email".
- **RQ7:** before a school's row ships: `robots.txt` and any posted terms read and dated in the row, and the question added to the lawyer's list.
- **RQ10:** `VISION.md` gains the one line in §10.
