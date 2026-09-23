# The commitment model — piece 1 of "know what's next"

**Date:** 2026-09-23. **Status: DRAFT for Quinn's review.** Nothing here is built; the plan follows
this spec. **Authority:** `docs/specs/2026-09-09-knowlu-cloud-design.md` (signed; its §1 decisions
and the 2026-09-17 amendment bind this document), then
`docs/notes/2026-09-23-know-whats-next-direction.md` (the program; this is its piece 1), then
`CLAUDE.md`'s two overriding rules and engine invariants. Where this spec and the cloud design
disagree, the cloud design wins and this spec is wrong.

**Read with:** `docs/specs/2026-09-22-c1c-first-day-design.md` on branch `c1c-first-day` (D4: one
course code, one slug); `docs/plans/2026-09-23-engine-follow-ups-plan.md` on branch `j-followups`
(F2/F3: the `event-check` card this spec's card copies).

**Two senses of "commitment".** `config/planning.yaml`'s `recurring:` entries (an effort budget in
hours, no clock time — "German practice, 0.75h, Mon/Wed") already render as `commitments` in the
read model (`surface::Commitment`). They are untouched and keep that field name. This spec's
commitments are clock-time obligations — a class, a shift, a club meeting — held as notes in
`commitments/`. In code the new module is `engine/src/commitments.rs`; the read model's existing
field is not renamed (it is a contract with the console).

**The gap this closes (direction note §2).** A wizard-created vault's `config/week_template.yaml`
has an empty class list every day, and the LMS feed carries assignments, not class meetings. Today
nothing knows when a student is in class, so "never plan over a class" has no data and every
capacity number is too high.

---

## 1. Decisions

### 1.1 Quinn's (binding, 2026-09-23)

| # | Decision |
|---|---|
| Q1 | Tagline **"Knowlu: know what's next."** Every feature must improve the morning answer — what to work on today, and in what order. |
| Q2 | **Login-only rule.** Students bring only their logins. Knowlu infers everything else from what those logins expose and asks only to confirm. Typing is a last resort. |
| Q3 | **Commitments live in the vault** (option A). Each carries a kind and a level: **hard** (never overlapped), **soft** (a proposal may overlap it), **optional**. Google Calendar stays read-only (`calendar.readonly`). |
| Q4 | **Class source.** Onboarding shows a confirm list pre-filled from the student's Google Calendar weekly recurring series, matched to the courses the wizard already captured (c1c D4). Where Google has no match, **both**: a registrar login at supported schools, and otherwise one confirm card per unmatched course over the first week. |
| Q5 | Observed calendars (patterns only): classes are weekly series titled by course code ("CS 100", "PH 106", "CS 100 Lab") with the room in the description; the same calendar holds club and team meetings (weekly series with non-course titles), routines ("Wake Up", "Bedtime"), office hours, one-off events and Gmail-created events (`eventType: fromGmail`). |

### 1.2 The controller's, delegated by Quinn and adopted

| # | Decision | Where |
|---|---|---|
| C1 | One note per commitment in a new vault folder `commitments/`, written through the engine's `write` (journal first, opaque `id:`). Default levels: class, lab, work → hard; club, meeting → soft; office-hours → optional. The student changes a level by editing the note; judge-once protects every field a human set. | §2 |
| C2 | Routines are not overlap commitments. They set the day window, used wherever `week_template.yaml`'s `day_start`/`day_end` are used. Config files are never rewritten. | §6.3 |
| C3 | `WeekCalendar` reads confirmed hard and soft commitments' meeting spans **in addition to** `week_template.yaml`'s classes (a union), so ranking and capacity pick them up with no ranking change. A vault with no `commitments/` behaves byte-identically. | §6, §8 |
| C4 | Series data from Google reaches the device as a new JSON field beside the ICS in `/ingest-calendar`'s reply, capability-negotiated with `accepts`, and lands in a new generated state file — never in `state/calendar.md`, whose frozen format does not change. Description retention is minimised. | §4, §9 |
| C5 | A deterministic classifier (no model) turns series into proposals; one-offs and `fromGmail` events are never commitments; everything proposed is `status: proposed` until confirmed. | §3 |
| C6 | Three confirmation paths: the onboarding confirm screen (phase 2, not charged to the cap); a `kind: commitment-check` card per proposed series found later (charged to the 15-a-day cap, never re-asked); the per-course fallback card, the one place typing is allowed (phase 2). | §5 |
| C7 | The registrar login gets its own later spec. Here only the `registrar:` source shape, and that it writes proposals like any other source. | §3.1 |
| C8 | Piece 2's overlap API: a pure `conflicts(span) -> Vec<(commitment, level)>`. Piece 2's proposals never overlap hard, may overlap soft (the card says so), ignore optional. | §7 |
| C9 | Out of scope: write-back to Google, the calendar view (piece 5), preference learning (piece 4). | — |

### 1.3 The controller's refinements, each with its cost if wrong

| # | Refinement (reason) | Cost if wrong |
|---|---|---|
| R1 | C1's `source` field is named **`source_uid`**, holding `gcal-series:<id>`, `ics-series:<uid>`, `registrar:<school>:<key>` or `card:<course-slug>`. A hand-written note has none. *Reason:* CLAUDE.md names `source_uid` as the vault's external key, and the dedupe helpers already read it. | A rename in one module and the notes it wrote; no user sees the field. |
| R2 | `meets:` is a **single-line flow sequence** (`[{days: [mon, wed], start: "12:00", end: "12:50"}]`). *Reason:* every note edit is single-line frontmatter surgery; a block list could not be amended. | None found; a block form would need a second emitter path. |
| R3 | A new id kind **`cmt`**, and `commitments` joins `ids::NOTE_FOLDERS` (id repair, backups and C3′ sync all key on it). | Id kinds are a vault contract; changing `cmt` later means re-iding every commitment. |
| R4 | A description is **never stored**. The function forwards at most 200 characters of it, only for recurring events with an empty location; the device reduces it to `where` (≤ 80 characters) and drops it. | A room the reduction misses shows no `where`; nothing ranks on `where`. |
| R5 | Series are read from **every calendar the student has selected** in Google (at most 10), not only `primary`. The `ics` field stays primary-only, so busy time does not change. *Reason:* a class schedule is often a separate subscribed calendar. | Extra API calls per fetch; a shared club calendar yields proposals the student declines. |
| R6 | A plain ICS feed (the secret iCal address, `cloud:personal`, or a direct URL) yields series too, from `RRULE:FREQ=WEEKLY` masters. A Google UID's `@google.com` suffix is stripped so both routes key the same series. | Students who only paste the secret address would otherwise get no pre-filled list at all. |
| R7 | A course-code-shaped title that matches **no vault course** is not a class; it is classified as a meeting. *Reason:* "Room 101" and "Bus 100" are code-shaped; Q4 says *matched to the courses the wizard captured*. | A real class missing from the LMS reads "a meeting?" and the student edits its kind. |
| R8 | Office-hours proposals get **no card**: an optional commitment changes nothing the morning answer shows. The phase-2 screen lists them. | An office-hours series stays `proposed` until the student confirms it on the screen or by hand. |
| R9 | `commitment-check` cards: at most **5 a day**, classes first; **no expiry**. | A slow onboarding without the phase-2 screen takes two or three days of cards. |
| R10 | The fallback card becomes eligible on **day 3** of the vault, paced 2 a day, not day 7. *Reason:* Q4 says "over the first week"; a week without class times is a week of wrong capacity. A pending fallback card is withdrawn if a series later matches the course. | One redundant card when a class shows up on Google late in week 1. |
| R11 | Routine window: `day_start` = the earliest confirmed wake routine's **end**; `day_end` = the latest confirmed bed routine's **start**. An inverted window is ignored with a warning. (See §11 Q1.) | Capacity changes on every day a student confirms a routine. |
| R12 | A change to a **confirmed** commitment's series (times, room, end) is an `amend` card; `commitments` joins `AMENDABLE_FOLDERS`. A series absent from Google for 14 days files an `amend` card proposing `until`. | Two more card shapes to maintain; the alternative is silent drift or silent edits. |
| R13 | **No new run-record step.** The pass's warnings join the existing `calendar` step's message. *Reason:* run records are a byte contract and `run-records-reference.json` is frozen. | No trend line of proposals; add a counted step later in its own commit. |
| R14 | A new engine command **`commitments`** (always exits 0) fetches, writes proposal notes (never cards) and prints them as JSON — the phase-2 screen's data source, testable now. | One more command in CLAUDE.md's list. |
| R15 | In the read model a commitment's span is a day block (`class` for class/lab, `busy` otherwise, labelled by its title), and a calendar event with the identical span that day is not drawn twice. No console change. | A console that wants a new block style for soft commitments needs a later `kind`. |
| R16 | Phase 1 executes **after `j-followups` merges**: it reuses F2's `what_and_when` time-range formatter and shares `cli.rs`, `approvals.rs`, `cloudmodel.rs` and `tests/cloud_contract.rs`. | Phase 1 waits on that branch. |
| R17 | An unknown `kind` loads with its written `level` (default soft), so `event`, `exam` and `task-block` from pieces 2–3 need no loader change. | A typo in `kind` still counts as busy time, and says so in a warning. |

---

## 2. The note

### 2.1 An example (invented values)

`commitments/cs-100.md`:

```yaml
---
id: cmt_3f9a1c2b7d
type: commitment
kind: class
level: hard
title: "CS 100"
course: cs-100
meets: [{days: [mon, wed, fri], start: "12:00", end: "12:50"}]
where: "Room 101"
from: 2026-08-19
until: 2026-12-04
source_uid: "gcal-series:4k2q9x7m1abc"
status: proposed
proposed_at: 2026-09-24
---

Found as a weekly series on your Google Calendar.
```

### 2.2 Field rules

| field | rule |
|---|---|
| `id` | `cmt_` + 10 hex, set by `write::create` like every note (R3). |
| `type` | `commitment`. `ids::kind_for` maps it to `cmt`; a note in `commitments/` without `type:` is also `cmt`. |
| `kind` | `class`, `lab`, `work`, `club`, `meeting`, `office-hours`, `routine`. Anything else loads under R17. |
| `level` | `hard`, `soft`, `optional`. Written at proposal from the kind's default (C1); absent → the kind's default; unknown → `soft` with a warning. **Ignored for `routine`**, which never takes part in overlap. |
| `routine` | only on `kind: routine`: `wake` or `bed` (§3.3 rule 1). A routine note without it is ignored with a warning. |
| `title` | the source's title, verbatim, ≤ 200 characters. The classifier never rewrites it. |
| `course` | a course slug; set for class and lab (always) and office-hours (when a code matched); absent otherwise. |
| `meets` | a flow sequence (R2) of `{days, start, end}`: `days` from `planning::DAY_KEYS` (`mon`…`sun`), `start` < `end`, both `"HH:MM"` 24-hour, **wall-clock in the vault's timezone** exactly like `week_template.yaml`. A meeting that crosses midnight is not representable and is not proposed. On a `confirmed` note an invalid entry is skipped with a warning, and a note with no valid entry is ignored with a warning; `proposed` notes are validated when written, and `declined` notes (which may carry no `meets` at all, §5.3) are not validated. |
| `where` | ≤ 80 characters, optional (R4). Display only; nothing ranks on it. |
| `from`, `until` | inclusive dates, both optional. Absent `from` = always started; absent `until` = open-ended. |
| `source_uid` | R1. Absent on a hand-written note. Two notes never share one: the proposer checks every note in `commitments/` before it creates. |
| `status` | `proposed` \| `confirmed` \| `declined`. **Only `confirmed` counts for anything** (capacity, overlap, the routine window). |
| `proposed_at` | the day the note was proposed; informational (the note is not an approval and is never charged to the cap). |

A student may create a note by hand with `status: confirmed` and no `source_uid`; id repair gives it
an `id`. Unknown extra fields are kept and ignored, as everywhere in the vault.

**File name.** `commitments/<ingest::slugify(title)>.md` ("CS 100" → `cs-100.md`, "CS 100 Lab" →
`cs-100-lab.md`); an empty slug becomes `commitment`. A collision takes `-2`, `-3`, as
`approvals::materialize` does. The name is never an identity; `id` and `source_uid` are.

### 2.3 Who writes which field

- **At proposal** (`agent:commitments`): every field above except `status` beyond `proposed`.
- **While `proposed`**, the proposer may refresh `meets`, `where`, `until` and `from` from the
  source, each only if `journal.human_set(id, field)` is empty — judge once, re-propose freely.
- **Once `confirmed` or `declined`**, the proposer never writes the note again. A changed source
  becomes an `amend` card (R12, §5.5).
- **`kind`, `level`, `title`, `course`, `status`** are never rewritten by any agent after creation.
  `status` moves only by a card's settlement, the phase-2 screen, or the student's own edit.

---

## 3. Sources and the classifier

### 3.1 Sources

| `source_uid` | from | phase |
|---|---|---|
| `gcal-series:<recurringEventId>` | the Google grant, through `/ingest-calendar?name=google&accepts=series` (§4.1) | 1 |
| `gcal-series:<id>` / `ics-series:<UID>` | any ICS feed the vault reads — the secret iCal address (`cloud:personal` or a direct URL): a `VEVENT` with `RRULE:FREQ=WEEKLY` and no `RECURRENCE-ID`. A UID ending in `@google.com` has the suffix stripped and is keyed `gcal-series:`, so the same series seen through both routes is one note (R6). | 1 |
| `registrar:<school>:<key>` | a registrar login at a supported school, e.g. `registrar:ua:202640-CS-100-001`. The registrar spec decides `<key>`; it must be stable across a term. Its proposals go through §3.4 exactly like any series, with `kind` and `course` set by the registrar rows rather than the classifier. | 3 |
| `card:<course-slug>` | the answer to a fallback card (§5.3); written `confirmed`. | 2 |
| *(none)* | a note the student wrote | any |

### 3.2 Normalising a series

Every source is reduced to one record before classification, so the classifier has one input shape:

```
Series { source_uid, title, where, event_type, meets: [(days, start, end)], first, until,
         next_14d, last_seen }
```

1. **Instances.** Google: the reply's `instances` (§4.1), converted to the vault's timezone (the
   `timezone:` of `config/ingest.yaml`, as `calfeed` does). ICS: the master expanded by `calfeed`'s
   existing recurrence code over the same 28-day horizon, EXDATEs honoured, overrides
   (`RECURRENCE-ID`) replacing their instance.
2. **Meets.** Count each `(weekday, start, end)` triple among the instances; keep the triples seen
   **at least twice** in the horizon (a single moved instance is an exception, not a meeting), then
   group kept triples by `(start, end)` into `{days, start, end}` entries, days in `DAY_KEYS` order,
   entries ordered by `(first day, start)`.
3. **`next_14d`** = instances starting in `[today, today + 14 days)`.
4. **`first`** = the master's start date when known, else the earliest instance. **`until`** = the
   RRULE's `UNTIL` as a date; a `COUNT`-bounded or open rule gives none (R12 catches the end).
5. **`where`** = the location, trimmed; else the first non-empty line of the description; cut to 80
   characters at a character boundary. The description goes no further (R4).
6. **`last_seen`** = today for every series a fresh fetch returned.

### 3.3 The classifier

Pure: `classify(series, courses) -> Option<(kind, course, routine)>`. **Eligible** only if all hold:
`event_type` is `default` or absent (ICS); the instances are timed, not all-day; `next_14d >= 2`;
`meets` is non-empty. Everything else — one-offs, `fromGmail`, `outOfOffice`, `focusTime`,
`workingLocation`, biweekly series, anything the student declined in Google (§4.1) — is **never** a
commitment. The first rule that matches wins; all matching is case-insensitive on the title with
surrounding punctuation trimmed.

1. **Routine.** The whole title is one of `wake`, `wake up`, `get up`, `alarm` → `routine: wake`;
   or `bed`, `bedtime`, `go to bed`, `sleep`, `lights out` → `routine: bed`. Whole-title only, so
   "Sleep study" is not a routine.
2. **Office hours.** `\boffice hours?\b` or a standalone `OH` → `office-hours`, with `course` when
   rule 3's code match also succeeds.
3. **Course.** The first course-code-shaped token, `\b([A-Za-z]{2,4})[ -]?(\d{3}[A-Za-z]?)\b`,
   normalised to `"<DEPT> <NUM>"` in capitals ("CS 100", "CS100" and "CS-100" are all `CS 100`),
   that equals a **vault course code** → `lab` if the rest of the title has the word `lab` or
   `laboratory`, else `class`; `course` = that course's slug. Any other suffix ("Recitation", "PYB")
   stays `class` of the same course. A code that matches no vault course falls through (R7).
4. **Work.** The title starts with the word `work` not followed by `on`, or has the word `shift` →
   `work`.
5. **Club.** The title has the word `club`, `society`, `team`, `practice`, `rehearsal` or `chapter`
   → `club`.
6. **Otherwise** → `meeting`.

**Vault course codes** are read, never fetched: each `courses/*.md` note's `title`, then `name`,
then `code`, each passed through the same token rule (the c1c D4 reading — a code in an LMS name
like `202640-BUI-100-101` is `BUI 100`), mapped to the note's `slug:` or file stem; then every
`config/ingest.yaml` `course_map` fragment that normalises to a code, mapped to its slug. The first
mapping for a code wins, in that order. A vault with no courses has no classes, only meetings and
routines, until its courses exist.

### 3.4 Writing proposals

`commitments::propose(vault, series, courses, today, ctx, journal) -> (created, warnings)`, pure
apart from its writes, in `source_uid` order:

- **Skip** a series if any note in `commitments/` has its `source_uid` — whatever that note's status.
  A declined series is never proposed again; that is the never-re-ask rule for proposals.
- **Skip** it if a note already has the same **signature** — `(kind, course or lower-cased title,
  meets)` — whatever its source. This catches the same class arriving from two routes (Google and a
  registrar) or matching a hand-written note.
- Otherwise `write::create` the note of §2 with `status: proposed`, actor `agent:commitments`, the
  run's `via` and `run_id`. The body is one sentence naming the source ("Found as a weekly series on
  your Google Calendar." / "…in your calendar feed." / "…in your registrar schedule.").
- A **still-proposed** note whose series changed has `meets`/`where`/`from`/`until` refreshed by
  `write_literals`, field by field, skipping any field a human set (§2.3).

Proposals are notes, not approvals: they are never charged to the 15-a-day cap. Only the card that
asks about one is (§5.2).

---

## 4. The transport change

### 4.1 The cloud: `/ingest-calendar` learns series

`GET /ingest-calendar?name=google&accepts=series`. The function is a `GET`, so the capability list
is a query parameter rather than stream J's body field: `accepts` is a comma-separated list, unknown
words are ignored, and **without `series` in it the reply is byte-for-byte today's** (`{ics,
source}`). With it, and only for `name=google`, the reply gains one field:

```json
{"ics": "BEGIN:VCALENDAR…", "source": "google_calendar",
 "series": [{"id": "4k2q9x7m1abc", "title": "CS 100", "location": "", "description": "Room 101",
             "event_type": "default", "until": "2026-12-04",
             "instances": [{"start": "2026-09-23T17:00:00Z", "end": "2026-09-23T17:50:00Z"}]}]}
```

- **Which events.** From every calendar in the student's `calendarList` with `selected: true`, at
  most 10 (R5): `events.list` with `singleEvents=true` over the same 28-day window, keeping only
  items that carry a `recurringEventId`, have a `dateTime` start (timed), and whose `attendees`
  entry with `self: true`, if any, is not `responseStatus: declined`. `eventType` is passed through
  (`default` when Google omits it); the device, not the function, decides eligibility (§3.3), so the
  rule lives in one place.
- **`until`.** One `events.list` with `singleEvents=false` per calendar returns the recurring
  masters; the `UNTIL` of a master's `RRULE` gives `until` as a date. No `UNTIL` → `null`.
- **Bounds.** At most 100 series and 40 instances per series; `title` and `location` cut to 200
  characters; **`description` is sent only when `location` is empty, cut to 200 characters** (R4).
  Nothing is written to a table or a log; the function's error lines name only exception classes,
  as today.
- **Failure.** If anything in the series gathering fails, `series` is **omitted** and `ics` is
  returned exactly as today — a series problem never costs the day's busy time.
- **The `ics` field does not change**: still `primary` only, still `toIcs`'s five properties. A
  test pins `toIcs` output byte-for-byte against today's for the same input.
- **`name=personal`** gets no `series` field: its ICS already carries RRULEs and the device derives
  series from it (§3.1).

### 4.2 The engine

- `cloudmodel::fetch_calendar(client, name)` sends `&accepts=series` and returns `(ics,
  Option<Vec<serde_json::Value>>)`; the absence of `series` is `None`. `engine/tests/cloud_contract.rs`'s
  request-line assertion (`GET /functions/v1/ingest-calendar?name=google HTTP/1.1`) is updated to the
  new line in the same commit, and a second assertion pins that a reply without `series` parses.
- `cli.rs`'s `calendar` closure (the one `Fetchers.calendar` is built from) keeps its
  `Fn(&str) -> Result<String, String>` shape, so `calfeed::load_calendar_events` and every oracle
  test are untouched. It **stashes**, per feed, what the series step needs: the Google `series`
  array for a `cloud:google` feed, the fetched ICS text for any other. A feed that fails stashes
  nothing.
- After `load_calendar_events`, `commitments::refresh_series(vault, stash, tz, today)` normalises
  (§3.2) — Google arrays directly, ICS text through a **new** pure `calfeed::weekly_series(text, tz,
  today)` that reuses the recurrence helpers and leaves `parse_calendar_ics` and `write_snapshot`
  alone — and updates `state/calendar-series.json`.

### 4.3 `state/calendar-series.json`

Generated state, like `state/calendar.md`; not a note, never synced, rebuilt from the sources.

```json
{"feeds": {"google": [{"source_uid": "gcal-series:4k2q9x7m1abc", "title": "CS 100",
  "where": "Room 101", "event_type": "default",
  "meets": [{"days": ["mon", "wed", "fri"], "start": "12:00", "end": "12:50"}],
  "first": "2026-08-19", "until": "2026-12-04", "next_14d": 6, "last_seen": "2026-09-24"}]}}
```

- One array per feed name, series sorted by `source_uid`, keys in the order shown; serialised with
  `ledger::dumps_value` plus a trailing newline, so the same data is the same bytes.
- **Per-feed fallback, like the snapshot:** a feed fetched fresh this run replaces its array, but a
  series it no longer returns is kept with its old `last_seen` until that is 14 days old, then
  dropped (R12 files its `until` card first). A feed that failed, or served no `series`, keeps its
  array untouched.
- **Written only when some feed produced a fresh result and the bytes differ.** A vault whose
  feeds are all unconfigured or unreachable — every oracle fixture — never gets the file.
- Unreadable or malformed → treated as empty, one warning, rewritten on the next fresh fetch.

### 4.4 Compatibility

| engine | function | result |
|---|---|---|
| old | new | no `accepts` → today's reply; the old engine never learns series. |
| new | old | `series` absent → `None` → that feed's series keep their previous state; no proposals, no error. |
| new, no account | — | direct ICS feeds still yield series (R6); `cloud:` feeds degrade to the snapshot as today. |
| new | new, Google not connected | the existing 409 → "using snapshot"; series unchanged. |
| old engine reading a vault a new engine wrote | — | `commitments/` is an unknown folder to it: ignored by ranking, by id repair and by backups. Capacity is simply what it was before this spec. C3′ multi-desktop version skew is the only way to reach this, and the app and engine ship as a pair. |

---

## 5. Confirmation paths and the cap

### 5.1 The onboarding confirm screen (phase 2)

Lands after c1b and c1c merge (both own `app/static/*` and `app/src/onboarding.rs`). Behaviour:

- Shown right after the calendar connection step, before the first slot. It runs
  `knowlu-engine commitments --vault <v> --json` (R14), which fetches, writes the proposal notes and
  prints them; the screen renders that list. A fetch that yields nothing skips the screen with one
  line ("Nothing repeating on your calendar yet — Knowlu will ask as it finds your classes").
- **Grouped by what matters:** *Your classes* (class and lab, one row per course, each labelled with
  the course code, days and times in the §5.2 format), then *Your week* (work, clubs, meetings),
  then *Your day* (wake and bed routines), then *Office hours*.
- **Pre-checked:** every class, lab and work row, and the routines. **Unchecked:** clubs, meetings,
  office hours. One control per row changes the level (hard / soft / optional) where the kind allows
  it; nothing on the screen asks for typing.
- A course the wizard captured with no class row is listed under *Your classes* as "no class times
  found — Knowlu will ask this week" (the fallback card, §5.3), or, at a school with a registrar
  source, offers that login (phase 3).
- **Finish** writes, through the engine's `write` with the console's human context, `status:
  confirmed` on every checked row (and its `level` if changed) and `status: declined` on every
  unchecked one. These are ordinary journaled note edits: **no approval is created, so nothing is
  charged to the cap.** Leaving the screen without Finish writes nothing; the proposals then reach
  the student as §5.2 cards.
- Which app command performs the writes (the existing `set_fields` or one new command) is the
  phase-2 plan's call; either way the engine's `write` does the work, and the Tauri command count
  in CLAUDE.md is recounted.

### 5.2 The `commitment-check` card (phase 1)

For every `proposed` commitment found after onboarding (or left unanswered by it), one card, built
on F2's `event-check` pattern:

- **Emitter.** `commitments::emit_checks(vault, today, budget, ctx, journal)`, called by `rank` after
  the proposal pass. It asks about a note only if its status is `proposed`, its kind is not
  `office-hours` (R8), and no card in `approvals/` or `archive/` has `target:` equal to that note's
  path or `source_uid` equal to its `source_uid` — any card, answered, pending or snoozed, closes
  the question for good.
- **Cap.** `allowance = min(budget, 5 − commitment cards whose first_proposed_at is today)` (R9) —
  "commitment cards" being `commitment-check` cards plus `amend` cards whose `target` is under
  `commitments/` (§5.5), in `approvals/` or `archive/`. `budget` is `daily_approval_budget −
  count_proposals_created(vault, today)` after the events pass has taken its share. The emitter sizes itself, so `defer_over_budget` never has overflow to
  snooze. Order: `class`, `lab`, `work`, `routine`, `club`, `meeting`, then any other kind; within a
  kind by the first meeting's `(day, start)`, then `source_uid`.
- **Title** (structured fields only, never a model): `{title≤40} · {days} {range} · {question}`.
  `days` joins the first `meets` entry's days as `Mon/Wed/Fri` (a run of three or more consecutive
  days collapses to `Mon–Fri`); `range` is F2's `what_and_when` range (`12–12:50pm`, `1–2:45pm`);
  a second `meets` entry adds ` +1 more time`. `question` by kind: `a class?`, `a lab?`, `work?`,
  `a club?`, `a meeting?`, and for routines `your wake-up time?` / `your bedtime?`. Example:
  `CS 100 · Mon/Wed/Fri 12–12:50pm · a class?`.
- **Frontmatter:** `type: approval`, `kind: commitment-check`, `title`, `status: pending`,
  `target: commitments/<file>.md`, `source_uid` (the note's, when it has one), `proposed_at` and
  `first_proposed_at` (both today), `expires: null`, `snooze_until: null`, `created_by:
  commitments`. Written with `write::create`; the journal actor is `agent:commitments`.
- **Body.** First paragraph (the card's `why` in the console): `**Is this part of your week?**
  Knowlu found it repeating on your calendar.` Then one line of what approving does, by level —
  hard: "Approve and Knowlu never plans anything over it."; soft: "Approve and Knowlu counts it as
  busy; an event suggestion may overlap it, and will say so."; routine: "Approve and Knowlu plans
  your days from 7:30am" / "…until 11pm" (the time from the note). Then `where` when present, and
  `Reject and it's ignored. Either way you won't be asked again. You can change its level later.`
  Path: `approvals/commitment-check-<note stem>.md`, `-2` on collision.
- **Settlement** in `approvals::transition_note`, beside F3's `event-check` arm:
  - `approved` → `write_literals(target, status: confirmed)`, then the card is stamped `executed`
    with `executed_at` and archived — the `amend` arm's tail.
  - `rejected` → `write_literals(target, status: declined)` before the generic archive.
  - `pending` / `snoozed` → unchanged machinery. There is no expiry (R9).
  - A target that is gone or no longer `proposed` (the student edited it by hand) → the card is
    archived as `refused` with no write, and one warning.
- **No app change.** `decide` already writes the card's status with the console's context and runs
  `process_approvals` in the same process (F3 decision 3); the console renders any `kind` with
  Approve, Reject and Snooze. The note's `status` is written by the approvals pass; the human's
  decision is the journal record on the card, as with every other kind.
- **An old engine** meeting an approved `commitment-check` warns `unknown kind` and leaves it; a
  rejected one is archived without writing `declined`, so the note stays `proposed` and, the card
  being archived, is never asked again. Reachable only across version skew; accepted and named.

### 5.3 The per-course fallback card (phase 2)

"When does BUI 100 meet?" — the one place the student types.

- **Eligible:** a course in `courses/` with no `class` commitment in any status other than
  `declined`, from **day 3** of the vault (the date of its earliest `state/journal/` file), at most
  **2 a day**, each charged to the 15 like any card (R10). Registrar schools try the registrar first.
- **Card:** `kind: commitment-ask`, `course: <slug>`, title `When does BUI 100 meet?`, body naming
  what the answer does and that the student can reject if the course has no meetings (an online
  course). Rejecting writes a `class` note with `status: declined` and `source_uid:
  card:<slug>`, which closes the question.
- **Collecting the answer** needs the app: the card renders a small form — day toggles
  Mon…Sun, a start and an end time picker, "add another time" — whose submit writes the answer onto
  the card as one flow field, `answer_meets: [{days: [...], start: "HH:MM", end: "HH:MM"}]`, then
  approves it. The approvals pass validates `answer_meets` exactly as §2.2 validates `meets` and
  creates `commitments/<slug>.md` with `kind: class`, `level: hard`, `course`, `meets`,
  `source_uid: card:<slug>`, `status: confirmed`. An invalid answer returns the card to `pending`
  with a warning, like a recoverable amend refusal.
- **Withdrawn:** if a series matching the course is proposed while the card is pending, the card is
  archived as `superseded` and the series' own card asks instead.
- Phase 2 because the form is `app/static` work. The engine half (emitter, settlement, withdrawal)
  ships with it, never alone: a card the console cannot answer would teach the student to reject.

### 5.4 Never re-asked

| situation | what closes it |
|---|---|
| a series already has a note | `source_uid` match, any status (§3.4) |
| the same meeting arrives from a second source | the signature match (§3.4) |
| a proposal has had a card | any card with that `target` or `source_uid`, in `approvals/` or `archive/` |
| a course was asked when it meets | its `card:<slug>` note, confirmed or declined |
| a confirmed commitment's series changed | one `amend` card per change; an `amend` already pending for the same target and field suppresses another (§5.5) |

### 5.5 Changes to confirmed commitments (R12)

When a fresh series differs from its **confirmed** note in `meets`, `where` or `until`, `rank` files
one `kind: amend` card per changed field with `target:` the note — the existing amend machinery,
with `commitments` added to `AMENDABLE_FOLDERS` and `meets`, `where`, `until` amendable **in that
folder only** (a per-folder allowance beside `AMENDABLE_FIELDS`, so a task's fields do not widen).
The amend's `from` is compared as the field's single-line text, exactly as the note holds it. Its
title follows §5.2 (`CS 100 now meets Tue/Thu 9:30–10:45am`). A series
dropped from `state/calendar-series.json` after 14 unseen days files an amend proposing `until:
<last_seen>`. Amend cards are charged to the cap as today, share the 5-a-day ceiling of §5.2, and are filed before that day's new `commitment-check` cards.

---

## 6. How ranking and capacity use it

### 6.1 One constructor for the vault's calendar

Today every production path builds its calendar the same way —
`WeekCalendar::from_file(&vault.join("config").join("week_template.yaml"), events)` in
`cli::run_with` (the `rank` step) and in `surface::load` (the read model). Both become
`WeekCalendar::for_vault(vault, events)`, which is `from_file` followed by
`with_commitments(commitments::load(vault))`. The test-only `from_file` calls on fixtures in
`ranking.rs`, `scheduling.rs`, `render.rs` and `surface.rs` stay as they are.

`WeekCalendar` keeps the template's `classes` exactly as today and gains one field,
`commitment_spans: Vec<Span>` where `Span { day: DayKey, start: Time, end: Time, from:
Option<Date>, until: Option<Date>, title, kind }`, filled only from **confirmed** notes whose level
is `hard` or `soft` (optional and routine notes contribute nothing here). Then:

- **`template_blocks(day)`** takes its busy list as the template's spans for `day_key(day)` **plus**
  every commitment span on that weekday whose `from ≤ day ≤ until`. The rest of the function — the
  sort, the cursor walk, the clamp to `day_end` — is unchanged, so overlapping or duplicate spans
  behave exactly as overlapping template classes already do.
- **`free_blocks`, `capacity`, `template_capacity`** follow from `template_blocks` with no change,
  so `rank`, `designate_today`, `start_by`, `slack_days`, the must-do partition and the gauge all
  see commitments with **no ranking change** (C3).
- A class that is both a confirmed commitment and a Google event in `state/calendar.md` is
  subtracted twice; subtraction of an already-busy span removes nothing, so capacity is right. The
  only visible shift is in `render::capacity_breakdown`'s wording, where that hour moves from
  "calendar" to "template" — correct, since it is now part of the timetable.

### 6.2 The read model

`surface::the_day` draws classes as the gaps between `template_blocks`. With commitments in that
busy list, a club would be drawn as `class`. So (R15): the gap walk runs over a new
`template_only_blocks(day)` (the template's classes alone, today's exact computation), and each
commitment span active that day is added as its own block — `kind: "class"` for class and lab,
`"busy"` otherwise, `label` = the note's title. A `busy` block from `events_on(day)` whose start and
end equal a commitment block's is dropped, so a class on both Google and in `commitments/` is drawn
once. Both kinds already exist, so the console renders them unchanged; `docs/surface/anatomy.md`
gains one paragraph saying where these blocks come from.

### 6.3 The routine window (C2, R11)

`with_commitments` also reads confirmed `kind: routine` notes. If any `routine: wake` note is
confirmed, `day_start` becomes the earliest `end` among their meetings; if any `routine: bed` note
is confirmed, `day_end` becomes the latest `start` among theirs. Each side is independent; a side
with no routine keeps the template's value. If the result has `day_start >= day_end`, both sides
keep the template's values and `commitments::load` returns the warning `routine window inverted
(<start>–<end>); using week_template`. `config/week_template.yaml` is never written.

### 6.4 Warnings and determinism

`commitments::load` never fails: a missing folder is empty, an unreadable note or invalid field is
skipped with a warning, notes are read in file-name order. Its warnings, the proposal pass's and the
card emitter's join the `calendar` step's message in the run record (R13). Nothing in this section
reads the clock, the network or a model — `rank never calls a model` holds, and
`rank_cannot_reach_a_judgment_endpoint` is unaffected.

---

## 7. The overlap API for piece 2

In `engine/src/commitments.rs`, pure, no I/O:

```rust
pub enum Level { Hard, Soft, Optional }

/// Every confirmed, non-routine commitment whose meeting overlaps [start, end) — half-open, local
/// wall-clock, in the vault's timezone. Sorted by (level: hard first, meeting start, title, id).
pub fn conflicts<'a>(commitments: &'a [Commitment], start: DateTime, end: DateTime)
    -> Vec<(&'a Commitment, Level)>;

pub enum Fit { Clear, OverlapsSoft(Vec<String>), OverlapsHard(Vec<String>) }

/// The rule piece 2 applies: any hard conflict → OverlapsHard (never propose); else any soft →
/// OverlapsSoft (propose; the card names them); optional conflicts are ignored → Clear.
pub fn fit(conflicts: &[(&Commitment, Level)]) -> Fit;
```

- A span that crosses midnight is split at midnight and each part is checked against its own
  weekday. `from`/`until` are checked against the date of each part.
- `conflicts` returns optional commitments too, so a caller that wants to say "during office hours"
  can; `fit` is where they are ignored.
- The input is the same `commitments::load(vault)` result `WeekCalendar::for_vault` uses, so the
  overlap rule and the capacity rule can never disagree about what is confirmed.
- Tests pin: a hard class overlapping by one minute is `OverlapsHard`; touching end-to-start is
  `Clear`; a soft club gives `OverlapsSoft(["Chess Club"])`; `proposed` and `declined` notes never
  conflict; a span outside `from`/`until` is `Clear`; a midnight-crossing span meets the next
  day's commitment.

---

## 8. What cannot change

- **The eight frozen Python references** (CLAUDE.md rule 2). `golden-today-s1.md` and
  `golden-today-full.md`: the fixture vaults have no `commitments/`, so the calendar, capacity and
  page are computed exactly as today. `calendar-snapshot-gcal.md`: `write_snapshot`,
  `read_snapshot` and `parse_calendar_ics` are not edited; series go to a different file.
  `vault-full/state/events.md`, the parsed references, `run-records-reference.json` (no new step,
  R13) and `pyyaml-safe-dump-reference.json` (`meets` goes through the existing flow emitter) are
  untouched by construction.
- **The three read-model references** `surface-today-{s1,s1-migrated,full}.json`: no commitment
  blocks exist in them, and the gap walk over `template_only_blocks` is today's computation. They are
  not regenerated.
- **Byte contracts.** No new `journal::VIAS` entry (`agent:commitments` is an actor, and
  `provenance::is_agent` is a `starts_with` test). Every JSON line goes through
  `ledger::dumps_value`. `/ingest-calendar` without `accepts=series` replies byte-for-byte as
  today, and `toIcs` is pinned.
- **Old engines** — §4.4.
- **A vault with no `commitments/`.** By construction `commitment_spans` is empty, so
  `template_blocks` builds the identical busy list; the routine window applies only when a
  confirmed routine exists; the de-duplication in §6.2 fires only beside a commitment block; and
  with no configured, reachable feed no series file is written. Four tests prove it:
  1. `for_vault_equals_from_file_without_commitments` — for each of `vault-s1`,
     `vault-s1-migrated` and `vault-full`, over the 35 days from 2026-08-24, `for_vault` and
     `from_file` give equal `template_blocks`, `free_blocks`, `capacity`, `template_capacity`,
     `day_start` and `day_end`.
  2. `proposed_and_declined_commitments_change_nothing` — the same comparison on a scratch vault
     whose `commitments/` holds only `proposed` and `declined` notes (routines included).
  3. `cargo test --test oracle --test surface_oracle` green with `git diff --exit-code
     engine/tests/fixtures` clean.
  4. `rank_on_vault_full_writes_no_series_file_and_no_commitments` — a copy of `vault-full` ranked
     with the fixture's feed configuration gains neither `state/calendar-series.json` nor
     `commitments/`.
- **`ids::NOTE_FOLDERS` grows to seven.** Id repair and backups iterate it and already skip a
  missing folder; `note_folders_is_a_subset_of_backup_folders` keeps checking the derivation, and
  `BACKUP_FOLDERS` grows to ten.

---

## 9. Privacy

- **What leaves the device:** nothing new. The request gains `accepts=series`.
- **What our function reads from Google:** the recurring events on the calendars the student has
  selected — title, times, location, description, event type and the student's own response —
  all within `calendar.readonly`, which the account already holds. It is read in memory for the
  request, sent to the device, and **not stored or logged**; error lines name exception classes
  only.
- **What reaches the device:** per series, the title, times, location, event type, `until` and — only
  when the location is empty — at most 200 characters of description (R4).
- **What the device keeps:** `state/calendar-series.json` (title, `where`, meeting times) and the
  commitment notes. **The description is never written anywhere**; `where` is at most 80
  characters.
- **What our servers keep:** under the 2026-09-17 amendment (ruling 2) the account holds the vault.
  `commitments/` notes and `commitment-check` cards therefore sync like every note: a confirmed
  class's title, times and room are stored server-side, readable by the service, and deleted with
  the account. `state/` never syncs, so the series file does not.
- **No model.** Nothing in this piece sends calendar data to an inference provider; the classifier
  is deterministic and runs on the device.
- **Limited Use.** Calendar data is used only for the user-facing feature — planning around the
  student's week — which is prominent (every card says it came from the calendar); it is transferred
  nowhere beyond our own service; no human reads it; nothing trains on it; export and deletion cover
  it because it is notes.
- **`site/privacy.html` needs a line.** Today it says what the calendar links are and that
  Calendar events go to the inference providers for judgments; it does not say that repeating
  events become notes the account keeps. The line (its wording is C3′'s privacy task, read by
  Quinn and the lawyer, per ruling 6) should say, in substance: *when you connect Google Calendar,
  Knowlu reads your repeating events to learn your week; the ones you confirm are kept as notes in
  your folder, which your account keeps in step; an event's description is never stored.* It lands
  in phase 1s (§10), because `site/privacy.html` is edited on `c1b-sign-in`, `c1c-first-day` and
  `c3-sync`.
- **Quinn-owned, no decision needed:** the `calendar.readonly` scope justification on the Google
  consent screen gains "learns the student's class and work schedule from repeating events" at its
  next submission.

---

## 10. Phases

### Phase 1 — engine and cloud (planned now; executes after `j-followups` merges, R16)

Delivers: the note format and loader, series transport and normalisation, the classifier, proposal
notes, `commitment-check` cards and their settlement, amend cards for changed series, capacity and
the read model, the routine window, `conflicts`/`fit`, and the `commitments` command. No
`app/**` file.

| file | change |
|---|---|
| `engine/src/commitments.rs` | **new** — `Commitment`, `Level`, `load`, `Series`, `refresh_series`, `classify`, `propose`, `emit_checks`, `conflicts`, `fit` |
| `engine/src/lib.rs` | one `pub mod commitments;` |
| `engine/src/ids.rs` | `NOTE_FOLDERS` + `commitments`; `KINDS`, `ID_RE` + `cmt`; `kind_for` maps `type: commitment` |
| `engine/src/backup.rs` | `BACKUP_FOLDERS` derivation grows by one |
| `engine/src/weekcal.rs` | `for_vault`, `with_commitments`, `commitment_spans`, `template_only_blocks`, the routine window |
| `engine/src/calfeed.rs` | **new** pure `weekly_series`; nothing existing edited |
| `engine/src/cloudmodel.rs` | `fetch_calendar` sends `accepts=series`, returns the optional array |
| `engine/src/cli.rs` | the stashing closure; `refresh_series`, `propose`, `emit_checks` and the R12 amend pass after `load_calendar_events`; `for_vault`; warnings into the `calendar` step |
| `engine/src/main.rs` | the `commitments --vault <v> [--today] [--json]` command (always exits 0) |
| `engine/src/approvals.rs` | `commitment-check` settlement; `commitments` in `AMENDABLE_FOLDERS` with its per-folder fields |
| `engine/src/surface.rs` | `for_vault` in `load`; commitment blocks and the duplicate drop in `the_day` |
| `engine/src/yamlemit.rs` | only if `safe_dump_flow` cannot already emit `meets` on one line (the plan checks first) |
| `engine/tests/cloud_contract.rs` | the new request line; a reply without `series` |
| `cloud/supabase/functions/ingest-calendar/{handler.ts,handler_test.ts,index.ts}` | `accepts`, `series`, `calendarList`, masters, bounds, failure omission, `toIcs` pin |
| `docs/surface/anatomy.md` | one paragraph (§6.2) and the `commitment-check` card |
| `CLAUDE.md` | `commitments/` in the vault folder list; the `commitments` command |

**Phase 1s — sync (executes after C3′ merges, or inside it if phase 1 lands first):** a new
migration adding `commitments` to `sync_notes_path_check`; `NOTE_PATH_RE` in
`cloud/supabase/functions/_shared/sync_rows.ts` and its test; the `site/privacy.html` line (§9).

### Phase 2 — the app (after c1b, c1c and C3′ merge)

The onboarding confirm screen (§5.1), the fallback card's form and its engine half (§5.3). Files:
`app/static/index.html`, `console.js`, `console.css`, `app/src/onboarding.rs`, possibly
`app/src/commands.rs` and `main.rs` (a command, recounted), `app/tests/{onboarding,static_assets}.rs`,
`scripts/wizard-check.py`, and `engine/src/commitments.rs` / `approvals.rs` for `commitment-ask`.

### Phase 3 — registrar logins

Its own spec (C7): the school list, the scraping, credentials in Credential Manager as
`knowlu/<profile_id>/registrar-<school>` read through the vault's `credential_target` like zyBooks
and VHL, and — after the 2026-09-17 amendment — the fetch sequence server-side through C5's relay.
It produces `registrar:` proposals through §3.4 and nothing else in this spec changes.

### Conflict check (`git diff --name-only main...<branch>`, 2026-09-23)

| branch | files it shares with phase 1 | risk |
|---|---|---|
| `j-followups` | `cloudmodel.rs`, `lib.rs`, `tests/cloud_contract.rs`, `CLAUDE.md`; its plan adds `cli.rs`, `approvals.rs`, `eventemit.rs` (F2/F3) | **High if run in parallel** — the same functions in `cli.rs` and `approvals.rs`. Resolved by R16: phase 1 starts after it merges. |
| `c3-sync` | `lib.rs`, `CLAUDE.md` (textual, one line each); **semantic:** its `sync.rs` pushes every `ids::NOTE_FOLDERS` path and its server check allows only six folders | Whichever merges second carries phase 1s' migration; until then a `commitments/` path would be refused by sync. Named in both branches' hand-off. |
| `c1b-sign-in` | `CLAUDE.md`; `site/privacy.html` (phase 1s only) | Low. |
| `c1c-first-day` | `CLAUDE.md`. Its D4 changes course-note titles, which §3.3 reads; the reading accepts both the old and the D4 shapes | Low. |

No branch touches `weekcal.rs`, `surface.rs`, `ids.rs`, `backup.rs`, `calfeed.rs`, `main.rs` or
`ingest-calendar/`.

---

## 11. Open questions for Quinn

1. **Does a confirmed wake and bed routine widen the planning day?** As written (R11), confirming
   "Wake Up 7:00–7:30" and "Bedtime 11pm" makes Knowlu plan from 7:30am to 11pm, so capacity grows
   into the evenings; the other reading is that routines may only narrow the template's
   8am–6pm window, never widen it. *Recommendation: widen.* Students do their work in the evening,
   the wizard's 8–6 default undercounts every student who does, and the card states the times
   before the student approves.
2. **Is the registrar login (phase 3) needed before the pilot?** *Recommendation: after.* Google
   series plus the fallback card give every student a path, and a registrar integration is
   school-by-school scraping with its own legal read under the relay-fetch amendment.
