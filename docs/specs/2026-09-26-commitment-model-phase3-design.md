# The commitment model, phase 3: the UA registrar

**Amended 2026-10-18:** the D3 amendment (R0) under D3, and "Amendment 2026-10-18 (A2)" at the end,
which states Quinn's close-out rulings of that day (apply and list). Where a line above either
disagrees, the amendment wins. **Revised the same day after spec review round 1:** the card rule,
where the list lives, the registration-status rule (P6) and the finals. **Revised again after
round 2:** an ended note is never ended again (11, pinned to its first-missing date); a row the
parser saw but could not read is never "missing" (P9, 11); the per-source table's two key types
(16); the asked-once wording, the card's `was`, the filing lock and the block's bytes (12, 13); the
finals premise checked against L5's amended §9 (Decision 1). **Revised again after convergence
round 3:** a kept-field card whose kept entry is replaced or removed is withdrawn (12, "Withdrawn
with its entry"); P9's merge carries a row only for a CRN with a confirmed note, so a row nobody
confirmed and the fetch could not read whole forms no series (P9, P6). Three choices that need Quinn are in
"Decisions for Quinn (A2)" at the end, and nothing that depends on them is built before Quinn
answers.

**Status: DRAFT 2026-09-26, written while Quinn was away. For Quinn's review.** The parent spec is
`2026-09-23-commitment-model-design.md` (§3.1 R1, R24, §10 Phase 3, §11 Q1 as ruled). Phase 2 is
`2026-09-24-commitment-model-phase2-design.md`. This document designs what §10 left to "the plan":
the sign-in sequence, the fetch and the parse. Where it is silent, the parent governs.

Quinn's rulings that bind it:
- **Registrar sources:** only UA's before the pilot (2026-09-23).
- **The myBama login:** only through the school's own sign-in window. Never a stored password; the
  session is thrown away (§11 Q1).
- **Login-only onboarding:** the student brings nothing but their logins.

## 1. Decisions

- **D1. The same pattern as the LMS link.** The fetch copies `app/src/lms_link.rs`, which already
  works and already matches the privacy page:
  - It opens an incognito `WebviewWindow` on a throwaway data directory, with no capability grant,
    so the campus page has no IPC.
  - The student signs in on the school's page (Okta, then Okta Verify at UA today) and presses
    **I'm signed in** in Knowlu.
  - The app reads that window's cookies for the registrar host only (`cookie_url`'s host rule), and
    makes one or two GETs with `ureq`.
  - It closes the window and wipes the directory.

  The cookies live only on the stack. Nothing is stored except the parsed schedule.
- **D2. The host and endpoint are data, curated per school.** The UA row of the curated campus table
  (`app/src/scaffold.rs`, where `lms_host` already lives) gains a `registrar` entry with four parts:
  - the sign-in start URL
  - the registrar host (`bannerssb.ua.edu`)
  - the Banner 9 path prefix (`/StudentRegistrationSsb/ssb/`)
  - the endpoint shape
  
  A school without the entry shows no registrar button, which is every school but UA before the
  pilot. Adding a school is a code change, as §10 says.
- **D3. Which endpoint is settled by a spike with Quinn, before any parser code (R0).** UA runs
  Banner 9 Self-Service (`https://bannerssb.ua.edu/StudentRegistrationSsb/ssb/registration`).
  Ellucian's student registration app exposes two readable shapes:
  - **(a)** `classRegistration/getRegistrationEvents?termFilter=`: one JSON object per meeting
    occurrence in a week, carrying the CRN, subject, course number, title, start and end.
  - **(b)** The registration-history and class-details calls, which return structured meeting
    patterns: `courseReferenceNumber`, `subject`, `courseNumber`, `sequenceNumber` (the section),
    `scheduleTypeDescription`, and `meetingTime {beginTime "0930", endTime, monday…sunday booleans,
    startDate/endDate "MM/DD/YYYY", building, room}`.

  **Preference: (b).** A weekly pattern with dates maps directly onto `meets`, `from` and `until`.
  (a) would have to be folded back into a rule, as the ICS path does.

  The spike answers five questions with Quinn signed in once:
  - which calls a signed-in student's session may make
  - whether a term must be selected first (a `term/search?mode=registration` POST)
  - the term code (UA's `202640` is Fall 2026, per C1c §0's `202640-BUI-100-101`)
  - what an online or TBA section looks like
  - whether Banner answers a request that carries no browser-only headers

  The spike records the shape, never a value: the test fixture is written by hand with invented
  courses.

### D3 amendment (R0, 2026-10-18)

**Source.** The R0 spike of 2026-10-18: the controller, with Quinn signed in once, in a dev copy of
this branch at `2ad1e3d`, in its LMS sign-in window; Chrome DevTools Protocol network shapes only
(the session-5 ledger's R0 and R0-additions rows). Nothing below is a student value: keys, types,
enums, counts and patterns only. The term codes are Banner's public identifiers. Where D3 above, §3's
"Parse" or the plan disagree with this subsection, it wins. Shape (b) is confirmed, in a form that
differs from D3's sketch: meetings are a flat `meetingTimes[]` on each row, not
`meetingsFaculty[].meetingTime`.

**What the page called.** On *View Registration Information*, after the sign-in redirect, the page
made these XHRs, all under `/StudentRegistrationSsb/ssb/`. Each sent `X-Synchronizer-Token` and
`X-Requested-With`; Q5 shows Knowlu needs neither the token nor any browser-only header.

| Method | Path | Answer | Knowlu uses it |
|---|---|---|---|
| GET | `selfServiceMenu/data` | XML (the menu) | No |
| GET | `userPreference/fetchUsageTracking` | JSON, three usage-tracking flags | No |
| GET | `menu?type=…` | XML | No |
| GET | `registrationHistory/reset?term={term}` | JSON, the rows (below) | **Yes: the rows call** |
| GET | `classRegistration/getRegistrationEvents?termFilter=` | JSON array, shape (a): one object per weekly occurrence (`start`/`end` `YYYY-MM-DDTHH:MM:SS-ZZZZ`, `crn`, `subject`, `courseNumber`, `title`, `term`, …); 17 events | No |
| GET | `classRegistration/getMeetingInformationForRegistrations` | JSON array per section, carrying both `meetingTimes[]` and `meetingsFaculty[].meetingTime`; no term in its query (it answers for the page's selected term) | No |

**The call list.** The curated UA row's `calls` (`app/src/scaffold.rs`), as `scaffold::Call` rows,
the rows call last:

```rust
calls: &[
    // The term list: recorded, recommended out (see "The term list").
    // Call { method: "GET", path: "classSearch/getTerms?searchTerm=&offset=1&max=12", form: None },
    Call { method: "GET", path: "registrationHistory/reset?term={term}", form: None },
],
```

- Both provisional rows go: the `term/search?mode=registration` POST (Q2: no selection call) and
  `classRegistration/getRegistrationEvents?termFilter=` (shape (a), which `parse_banner` cannot read;
  as built, a live run would skip every row and exit 2). `UA_ROWS_PATH` becomes
  `registrationHistory/reset?term={term}`.
- **Headers:** the window's cookies for the registrar host only (two cookies in the spike),
  `accept: application/json` and `x-requested-with: XMLHttpRequest`, which is what
  `app/src/registrar.rs`'s capture already sends. No synchronizer token, no User-Agent.
- **Redirects:** none on the rows call (200 directly). `ureq` drops `cookie` on a redirect, so a
  redirect still means the sign-in is gone; it reads as signed out, never as a schedule.

**The term list.** `GET classSearch/getTerms?searchTerm=&offset=1&max=N` (`N` = 12 in the spike)
answers a JSON array whose entries carry `code` and `description`; only those two keys were read, so
whether an entry carries dates is not recorded. It is the record Q3's suffixes come from. **Recommended
out of `calls`:** the app picks the term itself (Amendment 15's G3 rule over `registrar::term_for`), and
the capture keeps only the last call's body, so a first call would cost one request and change
nothing. Fetching it to check that the chosen term exists would be a design change (an open item for
B4, not assumed here).

**The rows call's shape.** `registrationHistory/reset?term={term}`, as an indented key tree. Types
are as recorded; "seen" sets are the spike's enum values.

```
(object)                                   no `success`, no `totalCount`; `data` is an object
  data
    maxHours, minHours                     string
    totalBill, totalCeu, totalCredit       string
    registrations[]                        one per registered section (8 for 202640; [] for others)
      term                                 string, "<year><suffix>" (Q3)
      termDescription                      string (not `termDesc`)
      courseReferenceNumber                string
      subject, courseNumber                string
      sequenceNumber                       string (the section)
      courseTitle, courseDisplay           string
      subjectDescription                   string
      scheduleDescription                  string (values not recorded); there is NO `scheduleType`
                                           and NO `scheduleTypeDescription` in this call
      instructionalMethodDescription       string (all 8 face-to-face in the spike)
      partOfTerm, partOfTermDescription    string
      startDate                            string
      statusIndicator, statusDescription   string (values not recorded; see P6)
      courseRegistrationStatusDescription  string
      registrationStatusDate               string
      creditHour, billHour, …              number
      faculty[]                            bannerId, displayName, emailAddress, primaryIndicator,
                                           courseReferenceNumber, term, category, class
      instructorNames[]                    string
      meetingTimes[]                       flat: no per-instructor wrapper (10 meetings over 8 rows)
        meetingType                        "CLAS" | "EXAM" (seen)
        meetingTypeDescription             string
        meetingScheduleType                "LEC" | "LLB" | "RCT" | "SEM" (seen; no "LAB" seen)
        beginTime, endTime                 "HHMM" (pattern 9999)
        startDate, endDate                 "MM/DD/YYYY", on every meeting
        monday … sunday                    boolean
        building, buildingDescription      string
        room                               string
        campus, campusDescription          string
        category, class                    string
        courseReferenceNumber, term        string
        creditHourSession, hoursWeek       number
      properties                           object: a near-copy of the row's own keys, without
                                           `subject` and `term`; never read
      …                                    about forty override, grade, waitlist and billing keys,
                                           never read
```

**The answers.**
1. **Which calls a signed-in session may make:** the rows call above, with the window's cookies for
   the registrar host. Nothing else is needed.
2. **Whether a term must be selected first:** no. The rows call carries the term in its query; no
   `term/search` POST.
3. **The term code:** `<year><suffix>`, read from `classSearch/getTerms`: Fall `<year>40`
   (`202640` = Fall 2026), Spring `<year>10`, Summer `<year>30`. `registrar::SCHOOLS`' UA entry,
   `[(1, "10"), (6, "40")]`, agrees for Spring and Fall. Summer is not in it, so a summer-only
   student gets no summer fetch: unchanged from the branch, and named as a gap rather than designed
   here. Three other terms probed answered 200 with an empty `registrations` (see P1).
4. **What an online or TBA section looks like: NOT answered.** All 8 registrations were
   face-to-face, 10 meetings, none without a time or a day. Until one is seen, the parser does not
   guess (P4). The fixture's online row (null times, every day false, null place) stays as an
   invented stand-in. The first time a real one is seen (B9 or a later term), its shape is recorded
   in a dated line under this subsection.
5. **Whether Banner answers without browser-only headers:** yes. Cookies only, plus `accept` and
   `x-requested-with`, gave 200 with no redirect and no synchronizer token (8 registrations for
   `202640`).
6. **(close-out plan q6) Term dates:** every `meetingTimes` entry carries `startDate` and
   `endDate`. The term list was read for `code` and `description` only. So a term's dates come from
   its rows, which settles G3 toward using them (Amendment 15).
7. **(close-out plan q7) Paging and finals:** the rows call is not paged. `data`'s keys are the six
   in the tree; there is no `totalCount`, `pageOffset` or `pageMaxSize`, so one answer is the whole
   term. A final-exam schedule is reachable in the same answer, as `EXAM` meetings (P3).

**What the parser must do on this shape.** These rules replace §3's "Parse" where they differ.
B4 implements them in `parse_banner`, the only place a Banner key is read (Plan ruling R1-a).
- **P1. Rows.** Rows are read from `data.registrations`. An answer without that array is exit 2
  ("not a list of registration rows"). A present but empty array is the empty parse: exit 2, nothing
  written. This matters under Amendment 2026-10-18 (A2). Three empty terms answered 200; if an
  empty answer counted as the term's whole truth, it would end every class in that term. It never does.
- **P2. Meetings.** A row's meetings are its flat `meetingTimes[]`. The `meetingsFaculty` /
  `meetingTime` wrapper is not in this call and is not read.
- **P3. Only `meetingType` `CLAS` becomes a series.** A row's `meets`, `first`, `until` and `where`
  come from its `CLAS` meetings only. **`EXAM` meetings sit beside `CLAS` meetings in the same
  `meetingTimes[]`, and a final is not a weekly class.** Folded into `meets` as the branch does
  now, a final's weekday would become a hard weekly block for the whole term, and its dates would
  stretch `first` and `until`. An `EXAM` meeting forms no series and is counted in `dropped.exam`
  without a warning (it is expected). Whether a final is also listed or written waits on Quinn
  (below, and Decision 1 at the end). A meeting whose `meetingType` is missing or is any other value
  is skipped, counted in `dropped.meeting_type` and warned. It is never read as `CLAS`.
- **P4. No time, no day.** A `CLAS` meeting with no `beginTime` or `endTime` (absent, null or
  empty), or with no day `true`, is skipped. It is never filled in from another meeting, a default
  day or the title. A row left with no usable `CLAS` meeting is dropped, counted in
  `dropped.no_time`, and named in `warnings` by its `SUBJ NUM` ("listed, never guessed"). §3's
  midnight rule is unchanged.
- **P5. Lab or class.** §3's kind rule reads `scheduleTypeDescription` and `scheduleType`, which
  this call does not carry; as built, every row would be a `class`. On this shape the signal is each
  `CLAS` meeting's `meetingScheduleType` (seen: `LEC`, `LLB`, `RCT`, `SEM`) and the row's
  `scheduleDescription` (values not recorded). Rule until the values are recorded: a row is a `lab`
  when a `CLAS` meeting's `meetingScheduleType` is `LAB`, or its `scheduleDescription` names "lab"
  or "laboratory" as a whole word (the plan review's M2 rule). `LEC`, `LLB`, `RCT` and `SEM` give
  `class`. Whether UA's `LLB` is a combined lecture-lab section or a lab section is open; B9 records
  each `meetingScheduleType` with its `meetingTypeDescription` and the row's `scheduleDescription`
  (enum descriptions only).
- **P6. Registration status (revised after review round 1).** Rows carry `statusIndicator` and
  `statusDescription`. The spike did not record their values, or whether a dropped or withdrawn
  section stays in registration history under another status. Under A2 this matters: a dropped
  section that is still returned is never "missing", so under the branch's rule (read every row) it
  would keep a hard weekly block all term and never be listed as ended. The rule:
  - **The registered set is data.** `registrar::SCHOOLS`' UA entry gains a set of `statusIndicator`
    codes that mean "registered", beside its term suffixes. An empty set means "every row counts",
    which is the branch's behaviour and the only behaviour before B9a.
  - **With a non-empty set,** a row whose `statusIndicator` is outside it forms no series and
    creates no note. It is counted in `dropped.status` and named in `warnings` by `SUBJ NUM` and its
    `statusDescription` (a Banner enum description, never a student value).
  - **A confirmed note whose row comes back under such a status** is neither applied nor ended. It
    is listed under `status` (A2 item 13) as "myBama lists this course as <statusDescription>;
    nothing changed". It is not "missing": item 11's "missing" means the fetch returned no row with
    that CRN at all.
  - **Why not end it:** an unrecorded code might be a second way of saying "registered". Ending a
    class that still meets is worse than keeping a dropped one, and the list makes the case visible. Turning a recorded drop or withdraw code into an end is a later
    change, signed by Quinn, recorded in a dated line under this subsection.
  - **Who records the set.** B9 records the `statusIndicator` and `statusDescription` of every row
    the signed-in account returns, with Chrome DevTools Protocol network shapes as R0 did (enum codes
    only; never from Knowlu's warnings, which name courses). Task B9a (close-out plan) writes the
    registered codes into `SCHOOLS`, with a test, before L10's tag. The friends release's condition
    on this is Decision 2 at the end.
- **P7. Never read.** `faculty[]` (instructors' names and email addresses), `instructorNames`,
  `properties`, and every grade, billing, override and waitlist key. The parser does not read
  them; nothing logs them or puts them in an error or a warning. The temp file holding them is
  deleted after the engine run (D4).
- **P8. Unchanged.** Times are `"HHMM"`, dates `"MM/DD/YYYY"` and days the seven booleans. The place
  is `buildingDescription` (else `building`) plus `room`, at most 80 characters. The key stays
  `registrar:<school>:<term>-<crn>`.
- **P9. What the parser saw (review round 2).** Besides its series and its `dropped` counts,
  `parse_banner` returns **`seen`**: every `courseReferenceNumber` in `data.registrations`, whether
  or not its row formed a series. Each CRN carries how it was read: `whole` (a series, no `CLAS`
  meeting skipped), `partial` (a series, but at least one `CLAS` meeting skipped under P4, or a
  meeting skipped under P3's unknown-type rule), `unread` (no series: P4 left no usable `CLAS`
  meeting, or P3 left only unknown-type meetings) or `status` (P6). `EXAM` meetings never make a
  row `partial`. This is the input to A2 item 11's "missing" test: a CRN in `seen` is never missing,
  whatever became of its row. B4 owns `seen` (it is what the parser returns); B3 reads it.
  - **Why:** a confirmed in-person section moved to TBA mid-term comes back with null times and is
    dropped by P4. Without `seen`, the run would find no series for its CRN, end the note and list it
    "ended by myBama" while the student is still enrolled.
  - **The merge keeps what it could not read (convergence round 1, for review; narrowed in
    convergence round 3).** The series `run` merges for the fetched term (through `refresh_series`)
    are the parser's `whole` rows, plus, for each CRN that `seen` marks `partial`, `unread` or
    `status` **and whose key a confirmed note in `commitments/` holds**, **that CRN's previous row
    from the series file, carried unchanged**: its `meets`, `first`, `until`, `where`, kind and
    course as stored, never the reduced read. Its `last_seen` becomes the fetch day, as for every row of a
    fresh calendar. A carried row counts as returned, so it is not moved to the file's `ended`. A
    previous row that already sits in the file's `ended` (its note was ended, A2 item 11) is left
    where it is.
    - **With no previous row, nothing is stored.** A `partial` row's reduced series never reaches the
      file, so a first fetch that reads a row only in part proposes nothing for it and creates no
      note (A2 item 10). It is listed under `unread` (A2 item 13). An `unread` or `status` CRN forms
      no series anyway (P4, P6).
    - **With no confirmed note, nothing is carried (convergence round 3, for review).** A `partial`,
      `unread` or `status` CRN whose key no confirmed note holds (an unmatched course's proposal, a
      declined key, a row never confirmed) is left out of the merge input, as P6 and P4 say for its
      row. Its previous row is not carried, so `refresh_series` moves it to the file's `ended` as for
      any row that leaves (its `ended` entry goes when a later fetch holds the key again, which is
      how a whole re-read brings it back). Three things follow, all from existing machinery:
      - **It proposes nothing.** `rank` builds no proposal from a key in `ended`. A `rank` whose
        `refresh_series` ran before the fetch's write still holds the key under `series` in its
        own file, so it files no card for it once it has re-read the file under
        `SERIES_LOCK_FILE` (A2 item 12, "A second call, under the lock"; convergence round 5).
      - **Its pending card goes.** A pending or snoozed proposal card for the key is withdrawn
        (stamped `superseded`) by `withdrawal_reason`'s proposal arm ("ended"), at the fetch's
        withdrawal pass (A2 item 12, "Withdrawn with its entry") and at every later `rank`.
      - **Its approval writes nothing (convergence round 5, for review).** Withdrawal alone does
        not keep the card from being settled: `withdraw_stale` reads only pending and snoozed
        cards, the app's approve settles at once (`decide` runs `process_approvals` in-process),
        and `rank` settles before it withdraws. So `settle_approved`'s proposal arm refuses it too
        (A2 item 12, "At settlement too"). Approving the card writes no note while the series file
        on disk holds the key in `ended` or under no calendar, so a hard note is never written for
        a section the registrar no longer lists as registered or could not read. A file that reads
        as empty says nothing, as everywhere else.
      - **It holds nothing back.** A registrar row that is not carried gives up D7's registrar
        precedence, so a Google series with the same signature is proposed again as it would be for
        any registrar row that left.
      - **Why only confirmed notes:** A2 item 12 needs `prior` only for a confirmed note's hand-set
        field. Carrying an unconfirmed row would keep a registrar series in `series` all term for a
        row P6 says forms no series, and `withdrawal_reason` would never withdraw its card: the key
        is still under a calendar, not in `ended`, and not past its `until`.
      - **What the student sees:** the row is still counted in `dropped` and named in `warnings`
        (P4, P6), and listed under `unread` by its key when it is `partial` or `unread` (A2 item 13).
    - **Why:** A2 item 12 reads `prior` from the series file. If a `partial` read replaced the last
      whole row, the next whole fetch would see a "change" from the reduced value back to the
      registrar's unchanged one and propose it against the student's own override, which is the
      re-proposal "judge once" forbids. If an `unread` or `status` CRN's row moved to `ended`, the
      next fetch would find no prior row and list a real registrar change as listed-only, never
      carded.
    - **What the branch does now:** `run` passes `parsed.series` to `refresh_series` as it is
      (`engine/src/registrar.rs`, `run`), so a reduced row replaces the stored one and an unread CRN's
      row moves to `ended`. B3 builds the carry; B4 only returns `seen`.

**`EXAM` meetings: open for Quinn (Decision 1 at the end).** No ruling of 2026-10-18 covers finals.
What is decided is P3's correctness rule: a final never becomes a series, never adds a weekday to
`meets` and never moves `first` or `until`, and it is counted in `dropped.exam`. That rule holds
whichever way Decision 1 goes, so B4 builds it and its tests now. What a final then becomes is the
open choice:
- **What L5 says today (checked in review round 2).** The amended L5 spec on the main checkout
  (`docs/specs/2026-10-05-syllabus-class-times-design.md`, amendment 2026-10-18, §9 "Exams" bullet)
  does **not** design a registrar finals rule. It defers to this amendment: if #19 files a final as a
  one-off `exam` commitment, a syllabus final for that course yields to it (no `syllabus-exam` card,
  a notice where the dates differ); if #19 drops finals, a syllabus final is proposed as L5 §4.6
  says; T4dt pins whichever is on `main`. So no lane owns registrar finals unless this decision
  names one. Round 1's premise ("L5 §9 holds the registrar-final authority", "L5's amendment is
  being written to cover" it) came from the close-out plan's proposed E6, which ruling 9 folded into
  L5's amendment; the amendment as written does not carry it.
- **(a) Skipped and counted, and named deferred.** No `exams` key, no page rows, no write. L5's §9
  handles this outcome as written (a syllabus or feed final is still proposed where one exists).
  Registrar finals become a named deferred item with an owner: a `HANDOFF.md` §4 row ("registrar
  `EXAM` meetings: skipped and counted; a later lane decides (b) or (c)"), written by `docs-keeper`
  in B6. Nothing built here would be reworked. The cost: the friends MVP shows no registrar finals,
  and at UA a syllabus rarely carries them (S2: dated schedule content in 2 of 8 documents).
- **(b) Listed, never written, now.** The report gains `exams`, one entry per `EXAM` meeting (the
  row's `source_uid`, the date, start and end), kept in the registrar block (A2 item 13), and the
  "From myBama" list shows each as the course's final. No series, no note. The cost: a report key,
  a block section and page rows. L5's §9 has no arm for it: a listed registrar final beside a
  proposed syllabus final for the same course would be two signals, so L5's amendment would need a
  third arm, which is a hand-off to L5's planner.
- **(c) Written now, as a dated one-off under R24.** `main` already has one: a confirmed note with
  one `meets` entry and `from` equal to `until`, which the campus-events accept writes through
  `commitments::create_confirmed`; `exam` is a declared kind, default level `hard` (parent R17). The
  final would be a hard `exam` note keyed `registrar:<school>:<term>-<crn>-exam`, created and applied
  under A2's rules. The most complete for "what's next?", since the final blocks its time from the
  first fetch. L5's §9 already has the arm for it (a syllabus final yields). The cost: a second
  create path, a moved-final case and a dropped-final case in B2 and B3, all before the friends
  release. The spike also did not record that `startDate` equals `endDate` on an `EXAM` meeting, so
  B9 would have to record it before (c) ships.

**Recommendation: (a), with the deferral named.** It is the smallest build before the friends
release, L5's §9 handles it as written, and nothing is reworked when a later lane picks (b) or (c).
The honest cost is that registrar finals have no owner until that `HANDOFF.md` row is picked up.
(c) is the strongest answer to "what's next?" and L5 is ready for it; it is the alternative if Quinn
wants finals in the friends MVP. (b) is not recommended: it needs a new L5 arm. The first draft of
this amendment recommended (b); round 1 found it built in as fact without a ruling, and round 2
found round 1's premise for (a) was not in L5's text.

*End of the D3 amendment; D4 continues §1.*

- **D4. The engine parses; the app carries bytes.**
  - The app writes the raw JSON it fetched to a temp file in the profile folder.
  - It runs `knowlu-engine commitments --vault <v> --registrar <file> --school ua`, then deletes the
    file, whatever the outcome.
  - The engine parses the file, normalises it into `Series` records, and merges them into
    `state/calendar-series.json`.
  - It writes the R24 confirmed notes (§3) and prints the result as JSON.

  `commands.rs` and the new app module compute nothing, as in phase 2 (`week.rs`).
- **D5. Registrar series persist until the next registrar fetch.** Today, `refresh_series` drops a
  calendar that is neither configured nor read in the last 14 days (`UNSEEN_DAYS`). A registrar is
  read once a term, so under that rule its series would age out two weeks into the semester.

  Instead, a calendar key starting `registrar:` counts as configured. Its series are replaced only by
  a fresh registrar fetch, and a series past its own `until` ends by the existing `until` rules. A
  row that is missing from a later fetch is a dropped course: it goes through the existing
  aged-out path to `ended` at once, since the fetch is the whole term's truth, and files a §5.4 end
  card.
- **D6. Where the button lives.** A **Get my class times from myBama** button appears in two places,
  and only when the school has a `registrar` entry:
  - under *Your classes* on the phase-2 confirm screen (`#week-setup`)
  - at the top of the *Schedule* view

  Not in the wizard: the wizard writes nothing before Finish, and these writes need the vault.
  Once a term (D8), the Schedule view says "Refresh from myBama" instead.
- **D7. The registrar outranks Google and the course card.**
  - A registrar series and a Google or ICS series with the same signature collapse to the registrar
    one, the way Google already outranks ICS (`precedence`).
  - A course with a registrar class needs no §5.3 ask card: the "uncovered" rule already reads
    confirmed class notes.
  - A vault whose series file holds a `registrar:` calendar skips `emit_asks` until the day after
    the registrar's term starts. This is the parent's "waits for the registrar pass instead".
- **D8. Once a term, never unattended.**
  - The fetch runs only when the student presses the button. It is never a slot step (§10).
  - The Schedule view shows "Refresh from myBama" when today is on or after the start date of a
    term the file does not hold. The term start comes from the fetched rows' `startDate`, and from
    Banner's term list when the spike shows one.
  - Nothing nags. The button is the whole reminder.

## 2. The flow

1. **The student presses the button.** The `open_registrar_window` command opens the incognito window
   on the school's registrar start URL. The console shows "Sign in to myBama in the window Knowlu
   opened, then press **I'm signed in**."
2. **The student presses I'm signed in.** `capture_registrar` reads the window's cookies for the
   registrar host and makes the spike's calls with `ureq`, sending the same headers the LMS capture
   sends. It returns one of:
   - `{ ok: true, bytes }` (the raw JSON, which the command keeps and never returns to the page)
   - `{ ok: false, error }`, with the plain reason when Banner answers with its sign-in page
     ("You're not signed in yet")
3. **The engine runs.** The command runs `commitments --registrar <file> --school ua` against the
   vault (§3). `close_and_wipe` then runs whatever the outcome.
4. **The page updates.** The confirm screen reloads its proposals, or the Schedule view repaints.
   Confirmed rows now show as confirmed. The others appear as proposal rows, marked "from myBama".

A failure at any step is a named message in the console, never a partial write: the engine writes
nothing unless the parse succeeds. **An empty parse is a failure, never an empty semester** (§10).

## 3. `commitments --registrar <file> --school ua`

- **Parse.** A row is kept when it has a CRN, a subject, a course number and at least one meeting
  pattern with days and times. Each kept row becomes one `Series`:

  | Series field | Taken from |
  |---|---|
  | `source_uid` | `registrar:ua:<term>-<crn>` (R1) |
  | `calendar` | `registrar:ua` |
  | `title` | `"<SUBJ> <NUM>"`, plus `" Lab"` when the schedule type is a lab, so the file name and the card read like the course (§2.2) |
  | `where` | `building room`, at most 80 characters |
  | `meets` | one entry per meeting pattern: days from the booleans, `HH:MM` from `"0930"` |
  | `first`, `until` | `startDate`, `endDate` |
  | `event_type` | `registrar` |
  | `rule` | weekly, interval 1 |
  | `instances` | none |

  - **Rows dropped.** A row without meeting times (online or TBA) is dropped and counted. A row
    whose times cross midnight is dropped with a warning (§2.2).
  - **Kind.** A schedule type naming a lab (Banner's `LAB`, "Laboratory") gives `lab`; anything else
    with a meeting time gives `class`. The classifier is not consulted, because the registrar is the
    authority.
  - **Course.** `course` is matched through the vault's code table (§3.4). An unmatched row keeps its
    title and has no course.
- **Merge.** `refresh_series(vault, &[("registrar:ua", rows)], today)` merges under D5's rule.
- **Write (R24).**
  - Each row whose `course` matches a vault course is written as a confirmed note through the phase-2
    confirm path: `commitments::confirm` with the level `hard`, actor `quinn` via `dashboard`. The
    student started the fetch, so it is their confirmation, and it is idempotent.
  - Other rows become ordinary proposals: they are listed, and carded from day 2 as §5.2 says.
    (Amended: a row the parser reads `partial`, `unread` or `status` with no confirmed note forms no
    series and so no proposal, and its pending card is withdrawn; D3 amendment, P9, convergence
    round 3.)
  - A confirmed note whose registrar row changed in a later fetch gets a §5.4 change card, and one
    whose row disappeared gets an end card. Both are existing machinery.
- **Output.** `{"term": "202640", "rows": n, "confirmed": n, "proposed": n, "dropped": {"no_time": n,
  "midnight": n}, "warnings": [...]}`.
- **Exit codes.** 0 on success. 2 on an unreadable file, a parse with no usable row, or an unknown
  `--school`. In every exit-2 case nothing is written.
- **No network.** The command makes no network call; a test pins this the same way `--confirm`'s is
  pinned.

## 4. Privacy

- **The privacy page.** `site/privacy.html` says the school window is used "to fetch two things —
  your calendar link and your course list". That becomes three things, adding "and, at schools
  Knowlu supports, your class schedule". The page must change in the same release that ships the
  button. The wording goes to the lawyer with P1; that is Quinn's item.
- **What stays on the device.** The fetched JSON is a temp file that is deleted after the engine run.
  The series file is local and unsynced, as today. Only confirmed notes sync, as today (P21).
- **Google's consent screen.** Unchanged: this is not Google data.

## 5. Tasks (for the plan)

| # | Task | Blocked on |
|---|---|---|
| R0 | **Spike, with Quinn at the machine.** Quinn signs in once in a dev build's registrar window, and the controller captures the call sequence and the response shapes, keeping no values. The result is a fixture shape and an amendment to this spec's D3. | Quinn |
| R1 | The engine: parse Banner rows into `Series`, from a hand-written fixture of the R0 shape; D5's `registrar:` rule in `refresh_series`; D7's precedence and ask gate. | R0 |
| R2 | The engine: `commitments --registrar`, covering the R24 writes, the output, the exit codes and the no-network pin. | R1 |
| R3 | The app: the `registrar` entry in the curated table, plus `open_registrar_window`, `capture_registrar` and `close_registrar_window`, modelled on `lms_link.rs`. The window gets no capability, so the console window's command count goes up by three. | R2 |
| R4 | The console: the button on the confirm screen and in the Schedule view, the "from myBama" marker, and the refresh-by-term rule. | R3 |
| R5 | The privacy page line, the docs, the recount and full verification; then a live proof on a scratch profile, with Quinn signing in. | R4 and Quinn |

## 6. Open for Quinn

1. **R0 needs you at the machine, once, for about ten minutes**, to sign in to myBama with Okta
   Verify in a dev build window. Nothing is kept but the response shapes.
2. **The privacy wording** in §4 goes to the lawyer with P1.
3. **Confirm R24 as written.** Registrar rows that match a course are confirmed without asking, at
   level `hard`.

## Amendments (2026-09-26, plan review)

Controller rulings at the pre-execution review of
`docs/plans/2026-09-26-commitment-model-phase3-plan.md` (review:
`…-phase3-plan-review.md`). Where a line above disagrees, this section wins.

1. **One calendar key per registrar term** (plan problem 1, Plan ruling R2-a). §3's `calendar` is
   `registrar:<school>:<term>` (`registrar:ua:202640`), not `registrar:ua`. D5's "a row missing
   from a later fetch is a dropped course" applies only to a fetch of the **same term**: UA
   students register for spring in November, and a spring fetch under one key would end every
   fall class. The `source_uid` is unchanged (`registrar:ua:<term>-<crn>`). D5's "counts as
   configured" holds while the term is in play (a series with an open `until`, or one less than
   28 days past); after that the term ages out like a removed feed (R2-b).
2. **A dropped course is dated the day before the fetch** (plan problem 2, R2-c). A registrar
   series has no instances, so its `ended` entry records `last_instance` = the day before the
   fetch that no longer returned it; the §5.4 end card proposes that date as the note's `until`.
   Without it the card would propose the term's own end, equal to the note's, and none is filed.
3. **Registrar series are left out of the instance map** (plan problem 3, R2-d).
   `SeriesFile::instances_map` would make a series' actual instances the only busy time inside
   its 28-day horizon, and a registrar series carries none. Left out, a confirmed registrar note
   blocks time by its weekly `meets`.
4. **R24 stays as written; change detection changes instead** (plan problem 4). §3's R24 notes
   are written as `quinn` via `dashboard`, as are phase 2's confirm screen's. Commitment change
   detection counts "the student set this field" only from a later `set` record by `quinn`; the
   `create` record no longer counts (the fix lands on `p2-commitments` and is merged before plan
   R1). So a note the student confirmed, here or on the confirm screen, still gets its §5.4
   change and end cards, and a field they later edit by hand is still never proposed back. §6
   item 3 is answered: R24 as written.
5. **Task order** (§5). The plan's R1–R3 (this spec's R1–R2) proceed before R0 against Banner 9's
   documented shape (b), on a hand-written provisional fixture; R0 must precede the app task
   (plan R4), which needs the call list and headers. If R0 finds a rows call with no per-meeting
   start and end dates, the controller stops for Quinn: D5, D7's ask gate and D8 read them.

## Amendments (2026-09-26, final review)

Controller rulings at the final whole-branch review (`final-review.md` in the plan's SDD ledger).
Where a line above disagrees, this section wins.

6. **The registrar holds a course's class and lab proposals** (I1, option a). D7's collapse by
   signature is not enough: a Google class a minute off Banner's times would be a second hard note.
   The collapse is per kind: once a course has a confirmed `class` note keyed `registrar:`, or a
   current `registrar:` `class` series, no Google or ICS `class` series of that course is proposed,
   on the confirm screen or as a card, whatever its times; a registrar `lab` does the same for
   Google or ICS `lab` series. A Google lab stays offered when Banner lists only the lecture. Accepted residual: a Google note the student
   confirmed *before* the fetch stays beside the registrar's note; the overlapping busy time
   subtracts nothing extra, and nothing proposes removing it.
7. **The parent's privacy line on the registrar is superseded** (final review m6). The parent
   spec's §9 line "the login stays in Credential Manager on the device" predates §11 Q1's ruling
   and contradicts it: no myBama login is stored anywhere. The student signs in in the school's own
   window, the session is thrown away when that window closes, and the fetch runs on the device.
   Only rows that become confirmed notes sync. The parent's line is read as this one.

## Amendment 2026-10-18 (A2), signed by Quinn's rulings of 2026-10-18 (ledger)

**Authority.** Quinn's rulings of 2026-10-18 on the PR #19 close-out
(`.superpowers/sdd/2026-10-18-session-5/progress.md`, the rows headed "QUINN RULINGS #19 close-out
§8"): (1) R24, (2) the card for a hand-set field, (3) G2-G5, (4) the per-source table, (5) the
privacy sentence, (8) the friends release. Above them stand the cloud design's ruling 3 (an
authoritative source applies and lists) and `VISION.md` commitment 5. The close-out plan is
`docs/plans/2026-10-18-p3-registrar-close-out-plan.md` (main checkout), §4 A2. The rulings are
Quinn's; this text states them and what follows from them, and `reviewer` (spec mode) reviews the
wording. Three kinds of line sit here, and each says which it is: a ruling, stated; a consequence of
a ruling that review rounds 1 and 2 and convergence rounds 1, 3, 4 and 5 made precise (the ending in 11,
the card's filing, withdrawal and settlement in 12, the registrar block in 13, the D3 amendment's P6 and P9),
marked **for review**; and a choice no ruling
covers, which is not built and waits in "Decisions for Quinn (A2)" (the finals; the friends
release's status condition; the ruled `CLAUDE.md` sentence, which no longer matches what the
command does). Where a line above disagrees, this section wins. In particular, §3's "Write (R24)" third
bullet, D5's end card, Amendment 2's end card, Amendment 4's actor and Plan ruling R2-g are replaced.

8. **Apply and list, at the fetch.** In `commitments --registrar`, after a parse that succeeds,
   each confirmed note keyed `registrar:<school>:<term>-<crn>` of the fetched term is compared with
   its new row. A changed `meets`, `where` or `until` is applied and listed. Two kinds of note are
   not compared field by field: a note whose CRN is missing goes to item 11 (which leaves an
   already-ended one alone), and a note whose CRN the parser saw as `partial`, `unread` or `status`
   (P9) gets nothing applied. Nothing is applied anywhere else: `rank` files no §5.4 change, end or swap card for a `registrar:` key from
   `detect_changes`. Its one registrar card is item 12's held kept-field card, offered again from the
   registrar block until it is filed. Plan ruling R2-g ("every registrar calendar counts as fresh" in
   `rank`) goes. `rank` still files §5.4 cards for `gcal-series:` and `ics-series:` keys, unchanged.
9. **What an applied write is.** Only `meets`, `where` and `until` are ever written. Never `level`,
   `kind`, `course` or the title. Each changed field is written through `write`: journal record
   first, then one single-line frontmatter edit, as **`agent:knowlu.registrar` via `dashboard`**
   (the student pressed the button; the change is the registrar's).
   - **Why an agent actor:** a write as the human would make every applied field read as hand-set to
     `journal::human_edited`, and no later registrar change would apply.
   - `agent:` keeps `provenance::is_agent` true, so no new `journal::VIAS` entry is needed. If B3
     finds one is needed after all, that piece moves to `contract-engineer`.
   - A `meets` change compares as phase 2 compares (the set of day, start and end triples), over
     `CLAS` meetings only (D3 amendment, P3).
10. **R24's create, as the student (ruling 1).** A matched row's note is still created as phase 2's
    confirm path creates one: `hard`, via `dashboard`, written as **the vault's human token**. That
    is `config/actor.yaml`'s `student` in a new vault and `quinn` in a legacy vault without it
    (cloud ruling 11). This replaces §3's and Amendment 4's literal `quinn`.
    - The branch's engine CLI default `--actor quinn` goes, and `registrar_argv` stops relying on
      it. B1 takes `main`'s actor mechanism, and the bare-literal test is the check.
    - **The create record is never a hand edit.** Only a later `set` record by the human makes a
      field hand-set (Amendment 4's rule, kept).
    - **A row read only in part is not created** (convergence round 1, for review). A row P9 reads
      as `partial` (or `unread`, or `status`) creates no note at that fetch, even when P9's merge
      carries an earlier whole row for it: R24 creates only from a row the fetch read `whole`, so a
      hard note is never written from a partial read. It is listed under `unread` (13) until a fetch
      reads it whole.
11. **A dropped course is ended, applied and listed.** A confirmed note is **missing** when a
    successful fetch of its own term returns no row with its CRN at all: its CRN is not in the
    parser's `seen` (D3 amendment, P9). A CRN in `seen` is never missing, whatever became of its
    row: under a status outside the registered set (P6), or with no usable `CLAS` meeting (P4) or
    only unknown-type meetings (P3). A missing note gets `until` = the day before the fetch
    (Amendment 2's date, R2-c). The write is applied as in 9 and listed as ended; no end card is
    filed. A fetch that exits 2 (an empty `registrations` included, D3 amendment P1) ends nothing. A
    row of another term is never "missing" (Amendment 1).
    - **A row the parser saw but could not read** (review round 2, **for review**). A confirmed note
      whose CRN is `unread` or `partial` in `seen` is left unchanged: no field is written, nothing is
      ended, no kept entry is recorded. It is listed under `unread` (13) with the parser's reason
      ("myBama lists this course with no class time; nothing changed", or "with a meeting Knowlu
      cannot read; nothing changed"). A hard note is never written from a partial read.
    - **Ended once (review round 2; the test corrected in convergence round 1, for review).** The
      registrar's end for a note is the day before the **first** fetch that found it missing since
      it last came back (below), and it is never recomputed. Each `ended` entry in the block (13) is
      **open** until the note comes back, and then **closed** (it gains a `returned` date). At most
      one `ended` entry per note is open at a time. A missing note is **already ended** when its
      term's registrar block holds an **open** `ended` entry for it. A closed entry never counts:
      the test reads the note's open entry, never "any `ended` entry".
      (B3 minors, 2026-10-21) The fetch takes the series-file lock before its first read and holds
      it to the merge's write, so P9's carry, this test and 12's kept detection read the file as it
      stands under that lock. Two fetches at once end a note once.
      - **Block absent** (13's accepted residual): the note is already ended when the journal's last
        `until` record for it is by `agent:knowlu.registrar` and holds a date before the fetch.
      - **What already ended means.** An already-ended note that is still missing gets no write, no
        new `ended` entry and no new kept value. It is not listed again as news: its open `ended`
        entry stands (13). So the course's end does not creep forward with each press, and a
        hand-set end is asked once (below).
    - **When the CRN comes back (corrected in convergence round 1, for review).** A later fetch
      whose `seen` holds an ended note's CRN as `whole` (the student re-registered) **closes** the
      note's open `ended` entry: it gains `returned`, the fetch day. That happens whether or not
      anything is written. The note is then compared as in 8: its `until` moves to the row's value,
      applied and listed under `applied`; a hand-set `until` follows 12 instead (kept, with the row's
      value as the registrar's).
      - **Why closing the entry, not "a later `applied` `until` entry":** with a hand-set `until`,
        the come-back writes nothing, so no `applied` entry would mark it. A second drop would then
        read as already ended: never written, never listed, no card.
      - **A later miss is a new drop.** With no open entry, it gets a new first-missing date and a
        new open `ended` entry. With a hand-set `until`, it also gets a new kept value and its one
        card. The closed entry stays as history.
      - **A come-back that is not whole is not a come-back.** A CRN that returns as `partial`,
        `unread` or `status` writes nothing, closes nothing and leaves the note ended. Its row is
        listed under `unread` or `status`.
    - **When the student set `until` by hand** (review round 1, **for review**): nothing is
      written. The note is listed as "ended by myBama, kept yours", and the end is a kept entry under
      12 whose registrar value is the pinned end (the day before the first fetch that found it
      missing since it last came back), so every later fetch computes the same value and the change
      card is filed once. The `ended` entry is recorded and opened all the same, so a come-back
      closes it and a second drop is a new drop (above).
12. **Judge once, and the card (ruling 2; the filing rule revised after review round 1, for
    review).** A field that `journal::human_edited` shows the student set by hand after the note was
    created is never written by the registrar.
    - **Detection, at the fetch.** For each such field, `prior` is the registrar's value for it in
      the series file before the merge, and `new` is the fetched row's value. Because P9's merge
      carries a confirmed note's `partial`, `unread` or `status` CRN's previous row unchanged, `prior`
      is always the last **whole** reading, never a reduced one. `until` has three special cases:
      - **A missing note:** `new` is the pinned end of 11 (the day before the first fetch that found
        it missing since it last came back).
      - **An already-ended note:** `prior` is that same pinned end, read from the note's **open**
        `ended` entry (11; a closed entry is never read). So `new` equals `prior` and nothing new is
        recorded.
      - **A note that comes back:** `prior` is the pinned end of the open `ended` entry this fetch
        closes (11).

      A come-back's row has no prior row in the series file (the drop moved it to the file's
      `ended`), so for `meets` and `where` `prior` is the value that same open `ended` entry kept
      from the dropped row (13; B3eR fix 2, for review). Without it, a hand-set field the registrar
      moved while the course was gone would never be shown. An entry that kept no value (the file
      held no row at the drop, or the block was lost) is "no prior row": listed only, never silent.

      A note whose CRN `seen` marks `unread`, `partial` or `status` gets no detection at all (11,
      P6). When `new` differs from both `prior` and the note's
      value, the fetch records a **kept entry** in the registrar block (item 13): the note, the
      field, the registrar's value and the student's. When the series file holds no prior row for
      the note, the kept entry is recorded as listed only, with no card. A fetch whose `new` equals
      the note's value removes any kept entry for that note and field: the two agree, and the
      entry's pending card is withdrawn ("Withdrawn with its entry", below). A fetch whose `new`
      replaces a kept entry's registrar value does the same to the old value's card. Between
      fetches, a kept entry whose note now holds the registrar's value (the student accepted the
      card, or edited the field to match) is neither listed nor offered; the next fetch removes it.
      `rank` never edits the block, except that `refresh_series` drops a term's block when that
      term's calendar ages out (13).
    - **Why not compare the registrar with the note alone:** the student's own override of the
      registrar's value would then be proposed back at every fetch, which is the re-proposal "judge
      once" forbids. Detection uses the change since the previous fetch; filing, below, is durable,
      so a card held back at one fetch is not lost.
    - **Filing.** Each kept entry that carries a card is offered to `commitments::emit_checks` as a
      change (target the note, `change` the registrar's value, `was` the note's value **as read at
      that filing**, not the student value stored in the entry), at the fetch and then at every
      `rank`, joined to the changes `rank` already passes, with the budget `rank`'s commitment pass
      computes. It is the **existing `commitment-check` change card**. Reading `was` from the note
      means a student who edits the field to a third value between fetches gets a card whose `was`
      matches the note, so P13 does not supersede it at once; the entry's stored student value is
      for the list only and is refreshed to the note's value at the next fetch.
    - **One filer at a time (review round 2, for review).** A slot's `rank` and a button fetch can
      run at once. Both hold `SERIES_LOCK_FILE` (`state/calendar-series.lock`, the lock that already
      guards `refresh_series`) from reading `emit_checks`' asked-once index for kept entries to the
      last card write, so the index read and the card write are one critical section and two
      processes cannot file the same `(note, field, value)`. Once it holds the lock, each process
      re-reads the series file. The registrar block its withdrawal test reads, and the kept entries
      it files, come from that re-read, never from a copy read before the lock (convergence round 4;
      "Where it runs", below). Each `registrar:` proposal `rank` files there is one the re-read
      still holds under `series` (convergence round 5). `rank` takes the lock after
      `refresh_series` has released it, since
      the lock is not re-entrant, and holds it from that re-read across its one `emit_checks` call
      (kept entries joined to its other changes). If the lock is held, the process waits (it
      blocks, as `lock_series_file` does today). B3 states in its commit which call sites take it.
    - **The cap, as built on the branch.** `emit_checks` files at most
      `min(the day's remaining approval budget, CHECKS_PER_DAY = 5 − commitment-check cards first
      proposed today)`. A change over that allowance is **not filed and not snoozed**
      (`defer_over_budget` never sees it). The kept entry holds it, and the next `rank` offers it
      again. Change cards pass the vault's first-day rule untouched; only proposals and window cards
      wait for day 2 (`cli.rs`'s commitment pass). B3 follows whatever `main`'s `emit_checks` does
      after B1 (lane L3's first-day allowance may move these numbers) and cites it in the commit.
    - **Asked once, as M-f defines it (wording corrected in review round 2).** `emit_checks` skips a
      change when a card neither `superseded` nor `expired` has the same `target` and the same
      canonical `change`. So while a card for one `(note, field, value)` is pending, snoozed,
      approved or rejected, no second card is filed. A card that **expired unanswered**, or was
      superseded, no longer closes the question (M-f; the branch's `commitments.rs` asked-once index
      and its test `a_card_that_expired_unanswered_does_not_close_the_question`): the kept entry still
      stands, so the next `rank` offers it and a new card is filed, inside the cap. That is the
      deck's rule for every change card, not a registrar exception. A later registrar value is a new
      change and gets its own card, and the card for the value it replaced is withdrawn (below); a
      fetch that only repeats the pinned end of 11 is not one.
    - **Withdrawn with its entry (convergence round 3, for review).** A kept-field card asks about
      one standing kept entry, and it goes when that entry goes. The branch's change arm cannot see
      this: `stale_change` withdraws only when the note no longer holds the card's `was`, and a
      registrar value that is replaced, or comes to agree with the student, leaves the note as it
      was. So:
      - **The rule.** A pending or snoozed `commitment-check` change card whose `target` is a note
        keyed `registrar:` is withdrawn (stamped `superseded`, one warning) when its term's registrar
        block holds **no standing kept entry** with the card's target and canonical change (the
        entry's field and registrar value). That covers a value replaced by a later one (A to B to
        C: B's card goes when C's entry replaces B's), the come-back of 11 (the pinned end's card
        goes when the row's term-end value replaces it), a later drop (the term-end card goes when
        the new pinned end replaces it), and agreement (the entry is removed, so its card goes).
      - **Only from a block that exists.** A term with no block (absent, or a series file
        unreadable this run) withdraws nothing, as `withdrawal_reason` already reads "under no
        calendar" only from a file that knows a calendar. A card on a target outside `registrar:`
        is untouched.
      - **Where it runs (call sites corrected in convergence round 4).** `withdrawal_reason`'s
        change arm (`commitments.rs`) adds this test after `stale_change`, reading the block from the
        `SeriesFile` it is given, so `approvals.rs`'s `withdraw_stale` and its signature do not
        change ("What does not change", below). `withdraw_stale` calls `withdrawal_reason` for every
        pending or snoozed card (`approvals.rs`, `withdraw_stale`), so the test runs at every
        `withdraw_stale` call, and each call site below says which block it passes. B3e names the
        three call sites in its commit. It runs:
        - **at the fetch:** after the series-file write has succeeded (13), the fetch takes
          `SERIES_LOCK_FILE` again, as `rank` does after `refresh_series`. Holding it, the fetch
          re-reads the series file (what it wrote, plus any `rank` merge since, which carries the
          block unchanged), runs `withdraw_stale` over that re-read, then runs `emit_checks`
          (filing, above) with the re-read's kept entries. It is the pass `rank` runs, so the fetch
          withdraws nothing `rank` would not; it also withdraws the proposal cards P9's narrowed
          carry leaves behind (D3 amendment). If the series-file write failed, or the re-read is
          malformed or unreadable, the fetch withdraws and files nothing; the block on disk still
          holds the old entries, and the next fetch recomputes them;
        - **at every `rank`, in two places** (`cli.rs`'s `commitment_passes`):
          - **The I5 call stays where it is** (the branch's `cli.rs` :826, after `commitments::load`
            and before `detect_changes`). It runs over the file `refresh_series` returned **with its
            registrar block emptied**, as `watched` empties `ended` after a bad read (:838-841). By
            "Only from a block that exists", the new test withdraws nothing there, and every other
            arm runs as it does today. The call does not move. `successor_keys` (:845) reads
            `approvals/` and `archive/` and skips a `superseded` card, so a card this call withdraws
            frees its key for the same run's `proposals`; moving the call later would change that
            output.
          - **A second call, under the lock.** Holding `SERIES_LOCK_FILE` for `emit_checks`, `rank`
            re-reads the series file (`read_series_file`, which takes no lock). It runs
            `withdraw_stale` again over the file `refresh_series` returned, with the re-read's block
            in place of its own for each term calendar that file holds. A term `refresh_series` aged
            out this run stays out, and a term the file does not hold has no block to read. It then
            counts the budget and calls `emit_checks` with the kept entries of that same block, and
            with the proposals (`asked`, :855) less every one whose key starts `registrar:` and is
            not under `series` in the re-read (a fetch since `refresh_series` moved it to `ended`,
            P9, or the re-read no longer holds its term; convergence round 5). Nothing else in the
            proposals changes: they are still built from the file `refresh_series` returned, and
            `withdraw_asks` and `emit_asks` read them as today. The drop is needed because
            `emit_checks`' asked-once index skips a `superseded` card, so a proposal card the fetch
            withdrew no longer stops a second card for its key. So a `rank` whose `refresh_series`
            ran before a fetch's write neither withdraws the card that fetch just filed nor files
            again a kept-field card, or a `registrar:` proposal card, that the fetch withdrew.
          - **Why the second call may run late.** A kept-field card's `change` holds a field value,
            never a `source_uid`, so withdrawing it there changes nothing that `successor_keys`,
            `proposals` or `withdraw_asks` read. With no fetch running alongside, the re-read's
            block for a held term is the one `refresh_series` carried, and the other arms read what
            the I5 call read, so they withdraw nothing more. In a vault with no registrar block,
            the second call has no block to read and withdraws nothing the I5 call did not. The same
            holds for the drop: with no fetch alongside, the re-read holds under `series` every
            `registrar:` key the returned file holds, and a vault with no registrar term has no
            `registrar:` proposal, so nothing is dropped.
          - **On a bad read.** When `refresh_series` could not read the series file, both calls are
            skipped, as the I5 call is today, and no kept entry is offered. When only the re-read is
            malformed or unreadable, the second call is skipped, no kept entry is offered and no
            `registrar:` proposal is filed. Kept entries are durable, and a `registrar:` series
            still in the file is proposed again, so a later `rank` that reads the file offers both.
            Each call's withdrawn-pending count comes back out of the pending line.
      - **At settlement too.** `settle_approved`'s change arm applies the same test, reading the
        series file on disk, and returns `Settled::Superseded` (not applied) for a card whose entry
        is gone. That closes the moment between a fetch's series-file write and its withdrawal pass.
        A `superseded` card does not close the question (asked once, above), so if the registrar
        later returns to that value, its new kept entry gets a new card.
        - **And a `registrar:` proposal card (convergence round 5, for review).** P9's narrowed
          carry needs the same test on the proposal arm. After its confirmed key-or-signature test
          and before `create_confirmed` (beside B4a's hold), `settle_approved` reads the series file
          on disk (`read_series_file`). For a card whose key starts `registrar:`, it returns
          `Settled::Superseded` (the reason, then "; nothing written") when that file's `ended`
          holds the key, or when the file knows at least one calendar and holds the key under none.
          These are the "ended" and "left the calendar" tests of `withdrawal_reason`'s proposal arm,
          made on the file on disk. A file that reads as empty (missing, malformed or unreadable) says
          nothing, as "under no calendar" is read today, and the card settles as it does now. A card
          keyed outside `registrar:` does not get this test. It runs after the confirmed-note test,
          so a card whose note this card already created still reads as executed. It closes the
          paths the withdrawal cannot reach: an approval the app settles between a fetch's
          series-file write and its withdrawal pass, and an approved card whose settlement failed
          and is retried by the next `rank`'s `process_approvals`, which runs before any withdrawal
          (`approvals.rs`, the `commitment-check` arm). The stamp is `superseded`, as the
          withdrawal's is, so the question stays open: a later fetch that reads the row whole puts
          the key back under `series`, and the next `rank` files one new card (P9's test "A dropped
          elective is not carried"). That test fixes the stamp, so it does not wait on the close-out
          plan's §8 Q12, which asks the same question for B4a's hold. B3e builds this test in
          `commitments.rs` and the drop above in `cli.rs`; `approvals.rs` does not change.
      - **No pre-A2 cards.** Item 8 retires `rank`'s §5.4 cards for `registrar:` keys. A pending one
        filed by the branch before A2 (a scratch or test vault) has no kept entry and is withdrawn
        by this rule, which is what item 8 intends. Nothing needs migrating.
      - If B3 finds that `approvals.rs` must change for any of this, that piece moves to
        `contract-engineer` (`approvals.rs` is on the contract list).
    - **Local.** The card is local by kind (`LOCAL_CARD_KINDS`, never synced), and the registrar
      block lives in the series file, which never leaves the device.
    - **What does not change.** No change to `approvals::AMENDABLE_FIELDS` and none to
      `approvals.rs`. If the registrar's value is unchanged since the previous fetch, or equals the
      student's, nothing is filed.
    - **Never silent.** Every kept entry is listed as "kept yours", with its card shown as filed or
      waiting. Whether it is filed is read from `approvals/` and `archive/` with `emit_checks`' own
      asked-once test, so the block stores no card state.
13. **The list, and the registrar block (where it lives: revised after review round 1, for
    review).** The command's JSON gains:
    - `applied`: per note, per field, `from` and `to`;
    - `ended`: per note, the `until` written, or "kept yours" (11);
    - `kept`: per note, the hand-set field, the registrar's value, the student's, and whether its
      card is filed, waiting, or listed only (12);
    - `status`: per note, the rows returned under a status outside the registered set (P6);
    - `unread` (review round 2): per note, a row the parser saw as `unread` or `partial` (P9) and
      its reason; the note is unchanged (11). A `partial` row with no note yet is listed by its key,
      never created (10);
    - `dropped` gains `exam`, `meeting_type` and `status` counts beside `no_time` and `midnight`.

    `exams` is added only if Quinn picks Decision 1 (b). The confirm screen's and the Schedule
    view's "From myBama" list shows added, changed, ended, kept, status and unread rows.
    - **The registrar block.** `state/calendar-series.json` gains a top-level `registrar` object,
      keyed by term calendar key (`registrar:ua:202640`). Per term it holds `first_fetch` and
      `last_fetch` (dates), `applied` and `ended` (each entry dated by its fetch day), `kept` (the
      standing entries of 12), `status` and `unread` (the latest successful fetch's). Each `ended`
      entry holds the pinned end of 11 (the `until` written, or the registrar's value when it was
      "kept yours"), and `meets` and `where`, the dropped row's last whole values, when the series file
      held its row at the drop (12's come-back; B3eR fix 2). Once the note comes back, it also holds
      `returned`, that fetch's day (11). An
      `ended` entry without `returned` is **open**, and the note's open entry is the record 11's
      "already ended" test reads.
      - (B3 minors, 2026-10-21) A `kept` entry with no prior value to compare (12's "no prior
        row") also holds `listed: true`: it is listed only, and no card is filed for it.
      - (B3 minors, 2026-10-21) An `ended` entry also holds `day`, the fetch day that ended the
        note, and `kept_yours`: `true` when the student had set `until` by hand and nothing was
        written, `false` when the fetch wrote the pinned end.
      - (B3 minors, 2026-10-21) B3d's ruling: a missing note whose `until` is already on or before
        the pinned end is skipped. Nothing is written, no `ended` entry is opened and nothing is
        listed, so `until` never moves later.
    - **How it changes.** `applied` and `ended` accumulate over the term: a fetch adds only its new
      entries, and an already-ended note adds none (11). The one change to an existing entry is a
      come-back: it sets `returned` on the note's open `ended` entry, once. `kept` is current state, changed only by
      12's rules. `status` and `unread` are replaced by each successful fetch. A fetch that exits 2
      leaves the block as it was. The term's block goes when its calendar ages out (Amendment 1).
    - **Its bytes (review round 2, for review; corrected in convergence round 2).** The block goes
      into the one value `file_bytes` builds and is written through `ledger::dumps_value` like the
      rest of the file. Nothing serialises it on its own path, and `ledger.rs` (contract list) does
      not change. The same history gives the same bytes:
      - **object keys are in `dumps_value`'s sorted order, at every level.** This spec fixes no key
        order of its own. So the top level reads `calendars`, `ended`, `registrar`, `series`; terms
        are in ascending key order; each term's keys read `applied`, `ended`, `first_fetch`,
        `kept`, `last_fetch`, `status`, `unread`; an entry's own keys, `returned` included, sort the
        same way;
      - every term key is present (an empty list is `[]`);
      - what this spec does fix is **the order of list entries**, which `dumps_value` keeps as
        given: `applied` and `ended` entries in (fetch day, note path, field) order, and `kept`,
        `status` and `unread` in (note path, field) order. `file_bytes` sorts them before the
        write, as it sorts `series` today;
      - an `ended` entry's `returned` key is absent while the entry is open (convergence round 1).
        Closing the entry inserts that one key and value in its sorted place. No other key or value
        of the entry, and no other entry, changes;
      - **when no term has a block, the `registrar` key is omitted**, so a vault that has never
        fetched writes `state/calendar-series.json` byte-identical to today's (item 16's "no output
        change", and `vault-full`).

      B2's "the block's bytes" test pins the sorted key order and the list orders above. Its
      expected bytes are written in that sorted key order, so B3c turns it green through
      `file_bytes` and `dumps_value` alone.
    - **When it is written.** In the same series-file write as the merge (whose input carries the
      previous rows of `partial`, `unread` and `status` CRNs that a confirmed note holds, P9), under
      the series-file lock, after
      the note writes, holding only writes that succeeded. If that write fails, the
      command warns and its JSON still lists the fetch, so the page shows it once; the journal still
      holds every applied write. Accepted residual: that fetch's list is not shown after a restart.
    - **Every writer keeps it.** `refresh_series` rewrites the series file on every `rank`; it
      carries the block unchanged, dropping only an aged-out term's entry. A series file without the
      block reads as an empty one.
    - **After a restart:** `commitments --json`'s overview reads the block for every in-play term,
      so there is no new command and no new Tauri command. The journal's `agent:knowlu.registrar`
      records stay the record of every write; the block is the page's view of them, and a test pins
      that each `applied` and `ended` entry has its journal record.
    - **Where these values go:** the page and the device only. None enters telemetry, a log line or
      an error string (plan constraint 9). The block is in the series file, so `sync`'s tripwires
      keep it off the wire with the rest of the file.
14. **Unchanged:**
    - R24's create, as amended in 10;
    - exit 2 writes nothing, applied writes and cards included;
    - the per-term key (Amendment 1), the in-play rule (Amendment 1, R2-b) and the per-kind hold
      (Amendment 6);
    - registrar series stay out of the instance map (Amendment 3);
    - idempotence (widened in review round 2): a later fetch with the same answer, **on any later
      day**, applies nothing, files no card that is already filed, withdraws no kept-field card
      (its entries are unchanged), adds no entry to `applied` or `ended`, and closes no `ended`
      entry. Nothing the run computes depends on the fetch day once a note is ended: 11 pins
      the end to the first-missing date, and 12 reads that pinned value. The standing `kept`,
      `status` and `unread` entries are still listed: they are state, not news, which is how this
      line and 12's "never silent" agree. A waiting kept card may be filed by that later fetch if the
      day's allowance now has room; that is the card being filed once, not a second card. Only
      `last_fetch` changes.
15. **G2-G5, the session's guesses of 2026-09-26 (ruling 3).**
    - **G2, confirmed:** one calendar key per term; a dropped course is dated the day before the
      fetch, now as the applied `until` of 11 rather than an end card; registrar series stay out of
      the instance map.
    - **G3, confirmed, using the rows' per-meeting dates** (the D3 amendment, answer 6). The rule,
      for review:
      - **When the button flips.** The Schedule view's *Refresh from myBama* (`registrar::status`'s
        `refresh`) turns on when every registrar term the series file holds is **over**: the latest
        `endDate` of its `CLAS` meetings is before today. "Over" is a new test in `registrar.rs`,
        named for it (`term_over`, say). It is **not** Amendment 1's in-play rule (an open `until`,
        or less than 28 days past), which keeps a term's series alive and must not be reused here:
        under it the button would stay off for four weeks after the last class. A test pins the two
        apart: a term 10 days past its last class is over for the button and still in play for
        keep-alive.
      - **Which term it fetches then.** The term after the latest held one, in `SCHOOLS`' order:
        a fall `<year>40` is followed by spring `<year+1>10`, and a spring `<year>10` by fall
        `<year>40`.
      - **With no held term,** `term_for`'s month table (1 January, 1 June) still picks the term.
      - **Who decides.** The engine decides (a `registrar` function beside `status`). The app's
        capture asks it and never computes a term itself.
      - **What it replaces:** the 1 January / 1 June flip as the only rule, and D8's "Banner's term
        list when the spike shows one". The term list was read for codes only.
    - **G4, closed:** by R0 (the D3 amendment) and task B4, which replaces the provisional call list,
      the parse and the fixture.
    - **G5, confirmed:** the registrar holds a course's class and lab per kind (Amendment 6), and a
      Google class confirmed before the fetch stays beside it (Amendment 6's accepted residual). L5
      inherits the rule for syllabus rows.
16. **The per-source table in `commitments.rs`, now (ruling 4; close-out task B3).** Today the
    registrar's prefix tests are spread over closures in `refresh_series`, `proposals`,
    `precedence`, `instances_map` and `SERIES_KEY_PREFIXES`. They become one table with **two key
    columns**, because two different strings select a source today (corrected in review round 2):
    a series' **calendar key** (`Series.calendar`: `registrar:`, `google:`, anything else) selects
    precedence, keep-alive, instances and the per-kind hold; a note's **`source_uid`** prefix
    (`registrar:`, `gcal-series:`, `ics-series:`) selects whether a change is watched and how it
    lands. A lookup names which key it uses; nothing matches a `source_uid` against a calendar
    prefix or the reverse. (Corrected at B3f against the code: the per-kind hold reads both keys,
    a series by its calendar and a confirmed note or a proposal card by its `source_uid`, each in
    its own column of the same row.)
    - **Columns:** calendar prefix; `source_uid` prefix; precedence tier; keep-alive rule; whether
      its series carry instances; whether it holds other sources' proposals per kind; how a change
      to a confirmed note lands (applied and listed, a `commitment-check` card, or not watched).
    - **Rows, as the branch behaves after A2:**

      | Calendar prefix | `source_uid` prefix | Tier | Keep-alive | Instances | Holds per kind | A change |
      |---|---|---|---|---|---|---|
      | `registrar:` | `registrar:` | 0 | in play by `until` (Amendment 1); a fetch that omits a row ends it at once (D5); out of play, read in 14 days | no | yes | applied and listed (8) |
      | `google:` | `gcal-series:` | 1 | configured, or read in 14 days | yes | no | carded |
      | any other | `ics-series:` | 2 | configured, or read in 14 days | yes | no | carded |
      | any other | any other `source_uid` | 2 | as its calendar | as its calendar | no | **not watched**: no card |

      The last row keeps `SERIES_KEY_PREFIXES`' allow-list: a confirmed note whose `source_uid` is
      outside it (an events one-off, a `card:` note, a hand-made note) files no change card today,
      and the table must not start filing one. A test pins it: a confirmed note keyed outside the
      three prefixes, with a series whose times moved, gets no `commitment-check` card. B3 checks the
      keep-alive and instances cells against the code it refactors and corrects this table in its
      commit if a cell differs; the table may not change behaviour to match this text. (B3f: the
      registrar's keep-alive cell gained the at-once end and the 14-day fallback the code has; the
      instances cells matched.)
    - **What L5 adds:** one `syllabus:` row (tier 1, `google:` moving to 2 and the rest to 3) and its
      two keep-alive arms.
    - **No output change:** the table changes no output on its own. The eight frozen references,
      the three surface references and `rank`'s output over `vault-full` stay byte-identical.
17. **Privacy (ruling 5).** This branch does not edit `site/privacy.html` (or `engine/tests/site.rs`).
    - **The sentence.** §4's words ("…your calendar link, your course list and, at schools Knowlu
      supports, your class schedule") ride lane L4's honest page and its `PRIVACY_VERSION` move.
      "At schools Knowlu supports" is true whether or not the button ships.
    - **The release guard.** §4's "the page must change in the same release that ships the button"
      becomes this guard: no release is tagged from a `main` carrying the registrar button until
      the page carries the sentence.
    - **Data minimization.** The answer the app carries holds instructors' names and email
      addresses (D3 amendment, P7). They are never parsed, logged or kept, and the temp file is
      deleted after the run (D4).
18. **The friends release (ruling 8).** The friends MVP carries the registrar button only if both
    hold before L10's tag:
    - B9's live proof has passed;
    - L4's page carries 17's sentence.

    Otherwise the UA curated row ships with `registrar: None` (data, not a build flag), so the
    button does not appear, and the entry returns in the next 0.1.x. Decision 2 at the end asks
    whether B9a's registered set (P6) is a third condition.
    - **Who checks, and who edits (review round 1).** Before L10's tag, the controller checks both
      conditions (and Decision 2's, if ruled in) against the ledger and records a dated row. If any
      fails, `mechanical` makes the fallback commit (below), `reviewer` reviews it, and it lands on
      `main` before the tag. The close-out plan's §5.5 release guard carries this step; L10's
      integrator does not tag without the controller's row.
    - **The fallback commit.** The UA row's `registrar` in `app/src/scaffold.rs` becomes `None`; no
      other row or field changes. Tests on the branch pin UA's entry (in `app/tests/registrar.rs`,
      the UA-row test and the campus.yaml-read-as-a-number test both expect UA to have one), so the
      commit also changes exactly those assertions to the release's behaviour (UA has no registrar
      entry), with the reason in its message. That is a deliberate behaviour change, not an
      assertion edited to pass.
    - **Kept small in advance.** B4, which already rewrites the registrar's app tests for the new
      call list, keeps every assertion that pins UA's real entry in as few tests as it can, builds an
      invented `Registrar` value for the rest, and names the UA-pinning tests in its ledger row, so
      the fallback's list is known before L10. B4's key-set test (close-out plan §7 R3 (c)) is
      written as "every curated row's `registrar.school` is a key of `SCHOOLS`", not set equality,
      so the fallback leaves it green.
19. **What R0 changes downstream.**
    - **The stop did not fire.** The rows call carries per-meeting start and end dates, so
      Amendment 5's re-plan trigger did not fire.
    - **No paging test.** The rows call is not paged (answer 7), so the close-out plan's "a paged
      answer below its stated total is exit 2" test is not needed. P1's empty-answer rule replaces
      it.
    - **The fixture.** `engine/tests/fixtures/registrar/banner-ua-registration.json` (hand-written,
      invented, not frozen) is rewritten to the recorded shape in task B4. It sits under
      `engine/tests/fixtures/**`, so it is a contract-list file even though it is not frozen:
      **`contract-engineer` writes it, and `contract-reviewer` reviews that diff** before any B4 test
      is written over it (convergence round 2). The rewrite is several screens, more than the main
      session edits. The review checks that the bytes keep the tree's `-text` encoding with no line
      ending re-encoded, that every row follows the recorded shape (the D3 amendment's answers) and not the new
      parser's output, and that every value is invented. It gains at least:
      - a row with a `CLAS` and an `EXAM` meeting;
      - a lab row on P5's rule;
      - an unknown `meetingType`;
      - a row under an invented non-registered `statusIndicator` (P6), with the invented registered
        code set in the test's own `SCHOOLS` entry;
      - a row whose only `CLAS` meeting has null `beginTime` and `endTime` (P4, P9's `unread`), and a
        row with one usable and one timeless `CLAS` meeting (P9's `partial`);
      - an empty-`registrations` answer (as a separate test input).

      Every expected value in the four shape-bearing test bodies stays.
    - **Order and owners (revised after review round 1).** The parse rules come before
      apply-and-list, so every apply-and-list test runs on the recorded shape. B4 moves ahead of B2
      in the close-out plan, in three steps:
      1. `contract-engineer` rewrites the fixture, and `contract-reviewer` reviews it (a contract-list
         diff); step 2 starts only on its PASS;
      2. `test-writer` writes the parse rules' failing tests: P1 to P6, P9's `seen` (each reading
         `whole`, `partial`, `unread`, `status` on its own row), and P3's finals rule (a final
         never adds a weekday to `meets`, never moves `first` or `until`, and is counted in
         `dropped.exam`);
      3. `implementer` makes `parse_banner`, the call list and the headers pass.

      B2 then writes apply-and-list's failing tests over that fixture, and B3 builds them. B4 owns
      what the parser returns, the `dropped` counts and P9's `seen` included; B4's tests pin that a
      row dropped under P3, P4 or P6 is still in `seen` with its reading (`unread`, `partial`,
      `status`). B3 owns the run, P9's merge carry, the report keys and the registrar block, and reads
      "missing" only from `seen`, never from the absence of a series. If Quinn picks Decision 1 (b), `parse_banner` returns the `EXAM` meetings
      (B4) and B3 adds `exams` to the report and the block; the "is listed" test joins B2 then, and
      not before.
20. **The tests review round 1 adds to B2** (all invented data, beside the close-out plan's B2 list):
    - **A held card is not lost.** With five `commitment-check` cards already first proposed today,
      a fetch whose registrar value moved for a hand-set field files no card, records the kept entry
      and lists it as waiting; the next day's `rank` files the card; a third run files no second one.
    - **The student's override is not proposed back.** A field the student set by hand to a value
      other than the registrar's, with the registrar's value unchanged since the previous fetch,
      gets no kept entry and no card, at the fetch or at `rank`.
    - **No prior row.** A hand-set field with no prior registrar row in the series file gets a kept
      entry listed only, and no card.
    - **Agreement clears it.** After the student accepts the card, the entry is neither listed nor
      offered; a later fetch whose value equals the note's removes it from the block. Second case
      (convergence round 3): with the card still pending, a fetch whose registrar value moves to the
      student's own value removes the entry and stamps the card `superseded`; approving it
      afterwards writes nothing (`settle_approved` returns `Superseded`), and the next `rank` files
      no card for the note's field.
    - **Change cards on day 1.** A kept-field card is filed on the vault's first day.
    - **The list survives a restart, all of it.** After a fetch that applies, ends and keeps, a new
      process's `commitments --json` overview shows the same `applied`, `ended`, `kept`, `status`
      and `unread` entries, read from the registrar block; each `applied` and `ended` entry has its
      `agent:knowlu.registrar` journal record.
    - **The block is kept.** A `rank` after the fetch rewrites the series file and leaves the
      registrar block byte-identical while the term is in play, and drops that term's entry once it
      ages out; a series file without the block reads as empty.
    - **Idempotence, read with 12.** A second identical fetch, run on the same day and again on a
      later day, adds no `applied` or `ended` entry, files no second card, and still lists the
      standing `kept`, `status` and `unread` entries; only `last_fetch` changes.
    - **A dropped course with a hand-set `until`** writes nothing, is listed "ended by myBama, kept
      yours", and gets one change card proposing the day before the first fetch that found it
      missing.
    - **The status rule (P6).** With a non-empty registered set, a confirmed note whose row comes back
      under another status is neither written nor ended and is listed under `status`; a row under
      such a status on a first fetch creates no note. With the empty set, every row counts.
    - **Over is not in play (15, G3).** A term 10 days past its last `CLAS` meeting is over for the
      button and still in play for keep-alive.

    Round 2 adds (all invented data):
    - **Ended once, across days (11).** Fetch on day 1 with no row for a confirmed note's CRN: its
      `until` is the day before day 1, one `ended` entry. Fetch the same answer on day 20: no write
      (no new journal record), no second `ended` entry, `until` still the day before day 1. The same
      with a hand-set `until`: one kept entry whose registrar value is the day before day 1, and
      exactly one `commitment-check` card over both fetches and the `rank`s between them.
    - **Ended once, with the block lost.** As above, but the series file's registrar block is
      removed between the fetches: day 20 still writes nothing, read from the journal's
      `agent:knowlu.registrar` `until` record (11's fallback).
    - **The CRN comes back.** After the day-1 end, a day-20 fetch that returns the CRN as a whole
      row moves `until` to the row's value, lists it under `applied`, and keeps the day-1 `ended`
      entry, now closed with `returned` day 20 (convergence round 1).
    - **A move across the drop (12; B3eR fix 2).** Day 1: the student sets `where` by hand to a value
      other than the registrar's, so no kept entry. Day 10: no row, so the note is ended. Day 20: the
      CRN comes back whole in another room. A kept entry for the new room, with its one card, never
      listed only. With the room unchanged on day 20, nothing is kept, listed or carded.
    - **Seen is not missing (P9).** A confirmed note whose row comes back with null `beginTime` and
      `endTime` (`unread`), one whose row has only an unknown `meetingType` (`unread`), and one whose
      row lost one of two `CLAS` meetings to P4 (`partial`): none is written, none is ended, each is
      listed under `unread` with its reason, and none gets a kept entry or a card.
    - **`was` is read at filing (12).** A kept entry recorded with the student's value A; the student
      edits the field to C; the next `rank` files the card with `was` C, and it is not superseded at
      that `rank`.
    - **Expired reopens (12, M-f).** A kept-field card that expires unanswered is filed again at the
      next `rank`, once; a rejected one is not.
    - **One filer (12).** With `SERIES_LOCK_FILE` held by the test, a fetch that has a kept entry to
      file does not write a card until the lock is released; a fetch and a `rank` on the same day
      file one card between them.
    - **The block's bytes (13).** A series file written twice from the same history is
      byte-identical; a vault with no registrar fetch has no `registrar` key, and its series file is
      byte-identical to the one `main` writes today.
    - **Not watched stays unwatched (16).** A confirmed note whose `source_uid` is outside
      `registrar:`, `gcal-series:` and `ics-series:`, with a matching series whose times moved, gets
      no `commitment-check` card.
    - **The next term (15, G3).** With a held fall `<year>40` term over, the engine's next-term
      function picks spring `<year+1>10`; with a held spring `<year>10` over, fall `<year>40`; with
      no held term, `term_for`'s month table decides. Invented years only.

    Convergence round 1 adds (all invented data):
    - **Dropped, back, dropped again (11).** Day 1: no row for a confirmed note's CRN, so `until` is
      the day before day 1 and one open `ended` entry is recorded. Day 20: the CRN comes back whole,
      `until` moves to the row's value, and the day-1 entry closes with `returned` day 20. Day 40:
      no row again. `until` is the day before day 40, written as `agent:knowlu.registrar` and
      listed under `ended`, and a second, open `ended` entry is recorded. A day-50 fetch with the
      same answer writes nothing and adds no third entry.
    - **The same with a hand-set `until`.** Day 1 lists "ended by myBama, kept yours" with one card.
      Day 20's whole come-back writes nothing and records a kept entry for the row's value. It still
      closes the day-1 entry. Day 40's miss is not already ended: a new open `ended` entry, a kept
      value of the day before day 40, and one new card for it, which is filed once over day 40, a
      day-50 fetch and the `rank`s between. Convergence round 3 adds: day 20 stamps the day-1 card
      (`until` = the day before day 1) `superseded`, and day 40 does the same to day 20's card; after
      each fetch at most one kept-field card for the note's `until` is pending or snoozed, and it is
      the one for the standing entry; approving a withdrawn card writes nothing, so the note's
      `until` never becomes the day before day 1 after the student re-registered.
    - **A come-back that is not whole.** After the day-1 end, a day-20 fetch that returns the CRN as
      `partial` (and, in a second case, `unread`) writes nothing, closes nothing and lists it under
      `unread`. A day-30 miss is still already ended: no write, no new entry.
    - **The merge keeps the last whole row (P9).** A note whose `meets` the student set by hand to S,
      with the registrar's W unchanged (no kept entry). Fetch 2 reads the row `partial`: the series
      file still holds W for it, byte for byte, apart from `last_seen`. Fetch 3 reads it whole as W:
      no kept entry, no card. The `unread` variant: fetch 2 reads it `unread`, its row stays in the
      file's series (not moved to `ended`), and a fetch 3 whose registrar value moved to X records a
      kept entry **with** its card, not listed-only. The `status` variant behaves as the `unread`
      one.
    - **A partial first read creates nothing (10).** On a first fetch, a matched row read `partial`
      creates no note, stores no series, and is listed under `unread` by its key. A later fetch that
      reads it whole creates the note `hard` as the student.

    Convergence round 3 adds (all invented data):
    - **A replaced value withdraws its card (12).** A confirmed note whose `meets` the student set
      by hand to S. Fetch 2 moves the registrar's value from A to B: kept entry B, card B pending.
      Fetch 3 moves it to C: kept entry C, card B stamped `superseded` at that fetch, card C filed.
      Approving card B after fetch 3 writes nothing. A fetch 4 back to B records entry B again and
      files a new card for it (a superseded card does not close the question).
    - **A block that is gone withdraws nothing (12).** With a pending kept-field card, a `rank` over a
      series file whose registrar block was removed leaves the card pending. A `gcal-series:` change
      card is untouched by the new test in every case.
    - **A `rank` does not withdraw a fresh card (12).** A `rank` whose `refresh_series` ran before a
      fetch wrote a new kept entry, and whose withdrawal calls run after that fetch filed the entry's
      card, leaves the card pending: the I5 call passes no block, and the second call re-reads the
      block under `SERIES_LOCK_FILE`. Convergence round 4 adds the replaced-value case. The rank's
      `refresh_series` reads kept entry B, with card B pending. A fetch then moves the registrar's
      value to C: entry C, card B `superseded`, card C filed. The rank leaves card C pending and
      files no new card B. Afterwards exactly one kept-field card for that field is pending (C's),
      and the next `rank` files nothing for it. B2 stages both interleavings deterministically,
      never by timing.
    - **A dropped elective is not carried (P9, P6).** With an invented non-empty registered set, an
      unmatched course's row (no vault course) is read whole on fetch 1 and becomes a proposal, and
      day 2's `rank` files its card. Fetch 2 returns the row under an invented non-registered
      `statusIndicator`: the series file no longer holds it under `series` (it is in `ended`), the
      pending card is stamped `superseded` by that fetch, the next `rank` proposes nothing for the
      key, and no note is created. The `partial` and `unread` variants behave the same. A fetch 3
      that reads the row whole again proposes it again (its `ended` entry goes), and the next `rank`
      files one new card for it inside the cap (the superseded card does not close the question).
    - **A confirmed note is still carried (P9).** The same three readings for a CRN whose key a
      confirmed note holds keep its previous row in `series`, byte for byte apart from `last_seen`,
      as "The merge keeps the last whole row" pins.

    Convergence round 5 adds (all invented data; B2 stages each interleaving deterministically,
    never by timing):
    - **A `rank` does not refile a dropped elective's card (P9, 12).** Day 2, with the setup of "A
      dropped elective is not carried": the unmatched course's proposal card X is pending. A rank's
      `refresh_series` returns a file holding X under `series`. A fetch then returns X's row under
      the non-registered `statusIndicator` (X moves to `ended`, card X is stamped `superseded`), and
      the rank runs on. It files no card for X and creates no note, and after it no card for X is
      pending. The `partial` and `unread` variants behave the same. Two controls, each with no card
      for X yet: with no fetch alongside, the rank files X's card as the branch does today; with
      only the re-read malformed (staged by the test), it files no `registrar:` proposal card and
      no kept-field card.
    - **Approving a dropped section's card writes nothing (P9, 12).** Card X pending; a fetch's
      series-file write moves X to `ended`; the card is approved and settled before that fetch's
      withdrawal pass. `settle_approved` returns `Superseded`, no note is created, the journal holds
      no `create` record for it, and the card is archived `superseded`. The same when the series
      file on disk no longer holds X's term but knows another calendar, and when the approved card
      is first settled by a later `rank`'s `process_approvals`. A fetch that then reads the row
      whole again leads the next `rank` to file one new card for X. This test never stamps a
      `registrar:` card whose key is under `series` on disk, a `registrar:` card over a series
      file that reads as empty, or a `gcal-series:` card whose key left its calendar.

    The close-out plan's "a paged answer below its stated total is exit 2" bullet is dropped (19).

## Decisions for Quinn (A2)

Asked one at a time, by the controller, recommendation first, before B2 starts. None is built
until Quinn answers; each answer is a dated row in the close-out ledger and, where it changes this
text, a dated line here.

1. **What a registrar final becomes** (D3 amendment, "`EXAM` meetings"). No ruling covers it.
   **Checked before asking (review round 2):** L5's amended spec (§9, "Exams") designs no registrar
   finals rule; it follows whatever this decision puts on `main`, with an arm for "dropped" and an
   arm for "a one-off `exam` commitment". So no lane owns registrar finals unless this answer names
   one. Before asking, the controller re-reads that bullet on the main checkout in case L5's running
   amendment has changed it, and adjusts this item if it has.
   - (a) **Skipped and counted, named deferred.** *Recommended:* the smallest build before the
     friends release; L5 handles it as written; nothing is reworked later. The cost, stated plainly:
     the friends MVP shows no registrar finals, and they have no owner until a `HANDOFF.md` §4 row
     (B6, `docs-keeper`) is picked up by a later lane.
   - (b) **Listed, never written, now**: an `exams` key, a block section and page rows. Not
     recommended: L5's §9 has no arm for it, so L5's planner would have to add one.
   - (c) **Written now** as a hard dated one-off `exam` note under R24, keyed
     `registrar:<school>:<term>-<crn>-exam`: the strongest answer to "what's next?", and L5's "yield"
     arm is ready for it. The cost: a second create path, moved-final and dropped-final cases in B2
     and B3, and a B9 recording that an `EXAM` meeting's `startDate` equals its `endDate`, all
     before L10's tag.
   - **Ruled 2026-10-19 (Quinn): (a).** Registrar `EXAM` meetings are skipped and counted, with a
     deferral row in `HANDOFF.md` §4 (B6); no lane owns registrar finals until a later lane takes
     the row.
2. **Does B9a's registered set join ruling 8's conditions for the friends release?** (D3 amendment,
   P6.) Without it, the button ships with every row counting as registered, and a course a tester
   drops mid-term may keep its hard weekly block all term, unlisted, if Banner still returns the
   row. *Recommended: yes.* B9 always sees the signed-in account's registered rows, so it can always
   record their codes; B9a is a one-line data edit with a test. With the set in place, the worst case
   is a dropped course listed under `status` and kept, which the tester sees. (Alternative: ship
   without it and state the residual in the release notes and `HANDOFF.md`.)
   **Ruled 2026-10-19 (Quinn): yes.** B9a's registered set joins ruling 8's conditions (a one-line
   data edit plus a test).
3. **The `CLAUDE.md` sentence (ruling 7) no longer says what the command does** (found in review
   round 2). Ruling 7 adds, as written: "`commitments --registrar` makes no network call and exits 0
   or 2; on 2 it writes nothing; it confirms matched rows as the student and applies later changes
   as `agent:knowlu.registrar`, journal first." Under A2 the command also files kept-field
   `commitment-check` cards (12) and ends dropped courses (11), and the existing `CLAUDE.md` line
   "`commitments` without `--confirm` always exits 0 and writes only the generated
   `state/calendar-series.json`" is untrue of `--registrar`, which writes notes and cards and can
   exit 2. Because the sentence was ruled word for word, changing it is Quinn's call.
   *Recommended:* B6 proposes, for Quinn's yes or no, (i) the existing line reads "`commitments`
   without `--confirm` or `--registrar`…", and (ii) ruling 7's sentence ends "…applies later
   changes and ends dropped courses as `agent:knowlu.registrar`, journal first, and files a
   `commitment-check` card, under the day's cap, for a field the student set by hand."
   (Alternative: keep ruling 7's sentence as written and let `docs/reference/engine-commands.md`
   carry the full contract.)
   **Ruled 2026-10-19 (Quinn): the recommendation, (i) and (ii).** `CLAUDE.md`'s line becomes
   "`commitments` without `--confirm` or `--registrar` always exits 0", and ruling 7's sentence gains
   "ends dropped courses as `agent:knowlu.registrar`… files a `commitment-check` card under the day's
   cap for a hand-set field". It lands with the close-out plan's B6.

Items 11, 12 and 13's ending, filing and storage rules, P6's and P9's rules and item 15's "over"
test are consequences of rulings 2, 3 and 8 made precise in review rounds 1 and 2 and convergence
rounds 1 and 3 (the open and closed `ended` entry, P9's merge carry and its narrowing to confirmed
notes, no create from a partial read, a kept-field card withdrawn with its entry); they are marked
**for review**, and `reviewer` (spec mode) checks them, not Quinn. Quinn signs the whole amendment
with these three answers.
