# The commitment model, phase 3: the UA registrar

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
