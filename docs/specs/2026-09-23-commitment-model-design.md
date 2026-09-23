# The commitment model — piece 1 of "know what's next"

**Date:** 2026-09-23. **Status: REVISED twice for Quinn's review** — the adversarial review's
findings, Quinn's two rulings of 2026-09-23 and the re-review's two Importants and nine Minors are
folded in (§12 lists every finding and what became of it).
Nothing here is built; the plan follows this spec. **Authority:**
`docs/specs/2026-09-09-knowlu-cloud-design.md` (signed; its §1 decisions and the 2026-09-17
amendment bind this document), then `docs/notes/2026-09-23-know-whats-next-direction.md` (the
program; this is its piece 1), then `CLAUDE.md`'s two overriding rules and engine invariants. Where
this spec and the cloud design disagree, the cloud design wins and this spec is wrong.

**Read with:** `docs/specs/2026-09-22-c1c-first-day-design.md` on branch `c1c-first-day` (D4: one
course code, one slug); `docs/plans/2026-09-23-engine-follow-ups-plan.md` on branch `j-followups`
(F2/F3: the `event-check` card this spec's card copies); `c3-sync:engine/src/sync.rs` (what leaves
the device, and the local-only sync cards of R-C3′-exec-18 that this spec's cards copy).

**Two senses of "commitment".** `config/planning.yaml`'s `recurring:` entries (an effort budget in
hours, no clock time — "German practice, 0.75h, Mon/Wed") already render as `commitments` in the
read model (`surface::Commitment`). They are untouched and keep that field name. This spec's
commitments are clock-time obligations — a class, a shift, a club meeting — held as notes in
`commitments/`. In code the new module is `engine/src/commitments.rs`; the read model's existing
field is not renamed (it is a contract with the console).

**The gap this closes (direction note §2).** A wizard-created vault's `config/week_template.yaml`
has an empty class list every day and a fixed 08:00–18:00 day, and the LMS feed carries
assignments, not class meetings. Today nothing knows when a student is in class or when their day
really starts and ends, so "never plan over a class" has no data and every capacity number is wrong.

**The shape in one paragraph.** Repeating events are read on the device and kept in a generated,
unsynced state file. A deterministic classifier turns them into *proposals*, which exist only in
that file and in local-only cards. Only what the student **confirms** becomes a note in
`commitments/` (and syncs like every note); what they decline leaves an anonymous marker holding
nothing but the source's opaque key. One exception, in phase 3 only: UA registrar rows that match a
vault course are written confirmed without a card (R24), because the student gave the registrar
login for exactly that and the registrar is the authority on enrolment. Confirmed hard and soft commitments are busy time for ranking
and capacity; one `planning-day` note holds the student's wake-to-bed window, which replaces the
template's fixed day.

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
| Q6 | **Planning day.** The student's routine window — wake to bed, 8am–10pm in the observed case — replaces the fixed 8am–6pm "usable" window. It is editable by the student at any time, and they can see how an edit changes the suggestions. An edit takes effect on the next rank, and the day view says what moved ("2 items moved to this evening"). |
| Q7 | **Registrar.** Before the pilot: the University of Alabama (myBama/Banner) registrar login only; other schools after. UA gets its own phase before the pilot: credentials on the device (Credential Manager, like zyBooks/VHL), fetch on the device, and the schedule becomes confirmed-or-proposed commitments with `source_uid: registrar:ua:<crn>`. This spec holds the design only, not the scraping. |

### 1.2 The controller's, delegated by Quinn and adopted

| # | Decision | Where |
|---|---|---|
| C1 | One note per **confirmed** commitment in a new vault folder `commitments/`, written through the engine's `write` (journal first, opaque `id:`). Default levels: class, lab, work → hard; club, meeting → soft; office-hours → optional. | §2 |
| C2 | Routines (wake, bed) are not overlap commitments. They propose the `planning-day` note (Q6), which sets each weekday's window wherever `week_template.yaml`'s `day_start`/`day_end` are used today. Config files are never rewritten. | §2.4, §6.3 |
| C3 | `WeekCalendar` reads confirmed hard and soft commitments' meeting spans **in addition to** `week_template.yaml`'s classes (a union), so ranking and capacity pick them up with no ranking change. A vault with no `commitments/` behaves byte-identically. | §6, §8 |
| C4 | Series data from Google reaches the device as a new JSON field beside the ICS in `/ingest-calendar`'s reply, capability-negotiated with `accepts`, and lands in a new generated, device-local state file — never in `state/calendar.md`, whose frozen format does not change. | §4 |
| C5 | A deterministic classifier (no model) turns series into proposals; one-offs, `fromGmail` events, biweekly and irregular series are never proposed. **When in doubt, do not propose**: a missed class costs one card; a false hard block costs a lost study slot every week. | §3 |
| C6 | Three confirmation paths: the onboarding confirm screen (phase 2, not charged to the cap); a local-only `kind: commitment-check` card per proposal found later (charged to the 15-a-day cap, never re-asked); the per-course fallback card, the one place typing is allowed (phase 2). | §5 |
| C7 | Registrar logins: UA only before the pilot (Q7), its own phase and plan; other schools later, each a code change. They produce proposals and confirmations through §3.4 like any source. | §3.1, §10 |
| C8 | Piece 2's overlap API: a pure `conflicts(span) -> Vec<(commitment, level)>`. Piece 2's proposals never overlap hard, may overlap soft (the card says so), ignore optional. | §7 |
| C9 | Out of scope: write-back to Google, the calendar view (piece 5), preference learning (piece 4). | — |

### 1.3 The controller's refinements, each with its reason and its cost if wrong

Rows marked **(rev.)** changed in the 2026-09-23 revision; **(new)** were added by it.

| # | Refinement (reason) | Cost if wrong |
|---|---|---|
| R1 | A note's source key is **`source_uid`**: `gcal-series:<id>`, `ics-series:<uid>`, `registrar:ua:<term>-<crn>` or `card:<course-slug>`. A hand-written note has none. *Reason:* CLAUDE.md names `source_uid` as the vault's external key. The registrar key carries the Banner term because a CRN is unique only within a term (Q7's `<crn>`, qualified). | A rename in one module and the notes it wrote; no user sees the field. |
| R2 | `meets:` (and the planning day's `window:`) is a **single-line flow sequence**. *Reason:* every note edit is single-line frontmatter surgery. The emitter (`yamlemit::safe_dump_flow`) sorts mapping keys and quotes times as PyYAML does, so its bytes are `{days: [mon, wed], end: '12:50', start: '12:00'}`; examples in this spec show the fields, not the exact bytes. | None found. |
| R3 | A new id kind **`cmt`**, and `commitments` joins `ids::NOTE_FOLDERS`. **(rev.)** Release gate: the phase 1s migration is live on prod before any build that carries `cmt` ships (§10, I9). | Id kinds are a vault contract; changing `cmt` later means re-iding every commitment. |
| R4 | **(rev.)** A description is **never stored**. The function forwards at most 200 characters of it, only for series with an empty location; the device keeps one line as `where` only if it looks like a place (§3.2 step 5) and drops the rest. | A room the reduction misses shows no `where`; nothing ranks on `where`. |
| R5 | **(rev.)** Series are read from **calendars the student owns**: `primary`, then other `calendarList` entries with `accessRole: owner` that are not hidden, in calendar-id order, at most 10. Subscribed and shared calendars are not read. *Reason:* another person's "Work" series or a club's public calendar must never become the student's hard busy time (review I1). | A class schedule kept on a subscribed calendar is not pre-filled; the student gets the fallback card, or adds that calendar's secret address as a feed (R6). |
| R6 | **(rev.)** A plain ICS feed yields series too, through **new** code in `calfeed::weekly_series` that honours `RECURRENCE-ID` overrides, `STATUS:CANCELLED`, the student's `PARTSTAT=DECLINED` and `TRANSP:TRANSPARENT` (§3.2). A Google UID's `@google.com` suffix is stripped so both routes key the same series. `parse_calendar_ics`'s busy time is not changed. | Students who only paste the secret address would otherwise get no pre-filled list. |
| R7 | **(rev.)** A series is a class only when its title **starts with one of this vault's own course codes**, however the vault's course notes and `course_map` spell them, followed by nothing or a known section word (§3.3). A fixed code pattern is only a fallback source of codes, with named blind spots. *Reason:* numbering differs by school (CS 1110, COMPSCI 61A, MATH 20A), and "Work on CS 100" or "CS 100 study group" is the student's own time, not a class (review C3, I2). | A class titled in a way no course note spells is not pre-filled; the fallback card asks. |
| R8 | Office-hours proposals get **no card**: an optional commitment changes nothing the morning answer shows. The phase-2 screen and the phase-2 "Your week" panel list them. | An office-hours series is never confirmed without the app. |
| R9 | **(rev.)** `commitment-check` cards: at most **5 a day**, classes first; **no expiry**; a pending card whose question went away is withdrawn (archived `superseded`) — for a proposal card, its series vanished from the series file; the rule is by card shape (§5.2). A withdrawn card does not close the question: if the series returns, it is asked again (re-review M-f). | A slow onboarding without the phase-2 screen takes two or three days of cards. |
| R10 | The fallback card becomes eligible on **day 3** of the vault, paced 2 a day. A pending fallback card is withdrawn if a class proposal for the course appears. | One redundant card when a class shows up on Google late in week 1. |
| R11 | **(rev., Q6)** The planning day is **one note**, `commitments/planning-day.md` (`kind: planning-day`), holding a per-weekday `window`. Routines only *propose* it. A weekday the note leaves out keeps the template's `day_start`/`day_end`. *Reason:* the ranking reads the vault, not app data, so the window must be in the vault to take effect on the next rank; a note (not `week_template.yaml`) because config files are never rewritten and a note edit is journaled and syncs. With no routine found, the phase-2 screen suggests 08:00–22:00 (the observed case) for the student to adjust. | A student with no confirmed window keeps 08:00–18:00 until they set one; a suggested 22:00 end over-plans the evenings of a student who stops earlier and does not adjust it. |
| R12 | **(rev., C1)** A change to a **confirmed** commitment (times, room, end date) is a `commitment-check` card with a `change:` field, settled by its own arm in `approvals.rs` — **not** the `amend` machinery, whose validator refuses null and sequence values (§5.4). `commitments` does **not** join `AMENDABLE_FOLDERS`. | One more settlement arm; the alternative (amend) can never apply these changes. |
| R13 | **(rev.)** **No new run-record step.** The pass's warnings join the existing `calendar` step's message. *Reason:* a new step would change every run's record shape and the console's Runs view for no reader; `run-records-reference.json` pins the serializer on fixed input, so this is a choice, not a constraint. | No trend line of proposals; add a counted step later in its own commit. |
| R14 | **(rev.)** A new engine command **`commitments`** (always exits 0) fetches, refreshes the series file and prints the current proposals as JSON — the phase-2 screen's data source. It writes **no note and no card**. | One more command in CLAUDE.md's list. |
| R15 | **(rev.)** In the read model a commitment's span is a day block (`class` for class/lab, `busy` otherwise, labelled by its title), clamped to that day's window and dropped if outside it; a calendar event with the identical span is not drawn twice. | A console that wants a new block style for soft commitments needs a later `kind`. |
| R16 | Phase 1 executes **after `j-followups` merges**: it reuses F2's time formatting (`eventemit::clock`, made `pub(crate)`) and shares `cli.rs`, `approvals.rs`, `cloudmodel.rs` and `tests/cloud_contract.rs`. | Phase 1 waits on that branch. |
| R17 | **(rev.)** Reserved kinds are declared now with default levels — `event` soft, `exam` hard, `task-block` soft — and load without a warning. Only a kind outside the full list (§2.2) loads as soft **with** a warning. *Reason:* a WARN on every run trains the reader to ignore warnings (review I7). | A typo in `kind` still counts as busy time, and says so. |
| R18 | **(new)** **Proposals are never notes.** They live in `state/calendar-series.json` (generated, device-local; C3′ sends only `state/journal/` records and notes under `NOTE_FOLDERS`) and in local-only cards. *Reason:* a proposal is an unconfirmed guess about someone's week — a therapy group, another person's shift — and must not reach our servers or the journal before the student says yes (review C2). | A second desktop can ask once about a proposal the first desktop has a card pending for. |
| R19 | **(new)** A **decline** writes an anonymous marker note: `type`, `status: declined`, `source_uid`, nothing else, at `commitments/declined-<10 hex of sha256(source_uid)>.md`. *Reason:* never-re-ask must hold across desktops, and the marker says nothing about the event. | A series declined through Google and later arriving from a registrar under another key is asked once more. |
| R20 | **(new; rev. by re-review N1)** `commitment-check` and `commitment-ask` cards are **local-only**, exactly like C3′'s sync cards: never pushed, nor any journal record about them. Their settlement's note write (a confirmed note, a decline marker) syncs as usual. **By construction:** the card kinds are one constant, `commitments::LOCAL_CARD_KINDS`; every card this piece files goes through one constructor that refuses any other kind; and `sync.rs` recognises a local card by that constant (the note's `kind`, or a `create` record's `new.kind`) beside its own `created_by: agent:knowlu.sync` test, so a new local kind is excluded the moment it is added to the list (§10 Phase 1s). | Phase 1s adds one predicate arm and one gate test to `sync.rs` (§10). |
| R21 | **(new)** Within the last fresh fetch's 28-day horizon, a confirmed `gcal-series:`/`ics-series:` commitment subtracts only its **actual instances** (moved instances where they moved, cancelled ones — holidays — not at all); beyond it, the weekly pattern. *Reason:* review I3. | `rank` reads one more generated file; a stale file falls back to the pattern. |
| R22 | **(new)** Google's "this and following" split (review I4): a confirmed note counts as **seen** when a fresh series has its `source_uid` **or** its signature; a successor series with the same signature is never proposed; a successor for the same course with different meets, starting as its predecessor ends, becomes **one** change card that also moves `source_uid`. | A split with a new title as well as new times is proposed as a new class, and the old one ends by its own change card: two cards. |
| R23 | **(new, Q6; rev. by re-review N2)** "What moved" is computed by **`surface`**, the console's live read model, purely and without writing: it designates today under the window the day started with and under the note's current window, and diffs the two plans (§6.4). `rank` only records the day's starting window in `state/plan.json`, and only when a `planning-day` note exists. *Reason:* `surface` recomputes the day on every refresh, so a window edit reshapes the console's today view at once; a `moved` computed by the next `rank` would describe, hours later, a change the student had already seen. | One more generated file; a vault without the note is unchanged. |
| R24 | **(new, Q7)** UA registrar rows whose course matches a vault course are written **confirmed** (the registrar is the authority on enrolment, and the student gave the login for exactly this); other rows become ordinary proposals; rows with no meeting time (online, TBA) produce nothing. | A dropped course still on the registrar stays busy time until the next fetch says otherwise. |

---

## 2. The notes

`commitments/` holds three shapes of note: a **confirmed commitment**, a **decline marker** and the
one **planning day**. No note in it is ever `proposed` (R18).

### 2.1 A confirmed commitment (invented values; fields, not exact bytes — R2)

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
status: confirmed
confirmed_at: 2026-09-24
---

Found as a weekly series on your Google Calendar.
```

### 2.2 Field rules

| field | rule |
|---|---|
| `id` | `cmt_` + 10 hex, set by `write::create` like every note (R3). |
| `type` | `commitment`. `ids::kind_for` maps it to `cmt`; a note in `commitments/` without `type:` is also `cmt`. |
| `kind` | `class`, `lab`, `work`, `club`, `meeting`, `office-hours`, `planning-day`, and the reserved `event`, `exam`, `task-block` (R17). Anything else loads as soft with a warning. |
| `level` | `hard`, `soft`, `optional`. Written at confirmation from the kind's default (C1, R17) or the level the student chose; absent → the kind's default; unknown → `soft` with a warning. Not used by `planning-day`. |
| `title` | the source's title, verbatim, ≤ 200 characters. The classifier never rewrites it. |
| `course` | a course slug; set for class and lab (always) and office-hours (when a code matched); absent otherwise. |
| `meets` | a flow sequence (R2) of `{days, start, end}`: `days` from `planning::DAY_KEYS` (`mon`…`sun`), `start` < `end`, both `"HH:MM"` 24-hour, **wall-clock in the vault's timezone** exactly like `week_template.yaml`. A meeting that crosses midnight is not representable and is not proposed. An invalid entry is skipped with a warning; a note with no valid entry is ignored with a warning. |
| `where` | ≤ 80 characters, optional (R4). Display only; nothing ranks on it. |
| `from`, `until` | inclusive dates, both optional. Absent `from` = always started; absent `until` = open-ended. |
| `source_uid` | R1. Absent on a hand-written note. The settlement checks every note in `commitments/` before it creates; a duplicate that arrives anyway (two desktops, a race) is handled by §2.5. |
| `status` | `confirmed` \| `declined`. **Only `confirmed` counts for anything** (capacity, overlap, the window). A note with any other status — including a hand-written `proposed` — is ignored with a warning. |
| `confirmed_at` | the day the note was confirmed; informational. |

A student may create a note by hand with `status: confirmed` and no `source_uid`; id repair gives it
an `id`. Unknown extra fields are kept and ignored, as everywhere in the vault.

**File name.** `commitments/<ingest::slugify(title)>.md` ("CS 100" → `cs-100.md`, "CS 100 Lab" →
`cs-100-lab.md`; `slugify` never returns an empty string — it returns `item`). A collision takes
`-2`, `-3`, as `approvals::materialize` does. The name is never an identity; `id` and `source_uid`
are.

### 2.3 A decline marker (R19)

```yaml
---
id: cmt_8c1d0e5a44
type: commitment
status: declined
source_uid: "gcal-series:4k2q9x7m1abc"
---
```

At `commitments/declined-<first 10 hex of sha256(source_uid)>.md`. No title, kind, meets, where or
body: the marker exists only so that no desktop asks about that series again. The loader reads only
its `source_uid`; it is exempt from §2.2's other rules. One marker key is not a source's: the
literal `source_uid: "window"` records that the student rejected a window proposal, and suppresses
every later one (§3.5).

### 2.4 The planning day (Q6, R11)

`commitments/planning-day.md`:

```yaml
---
id: cmt_51b7aa90c3
type: commitment
kind: planning-day
status: confirmed
window: [{days: [mon, tue, wed, thu, fri], start: "08:00", end: "22:00"}, {days: [sat, sun], start: "10:00", end: "22:00"}]
confirmed_at: 2026-09-24
---

The part of each day Knowlu plans in: from when you are up to when you stop.
```

- `window` has the shape of `meets`; each weekday appears in at most one entry (a later entry's
  duplicate day is skipped with a warning). A weekday not listed keeps `week_template.yaml`'s
  `day_start`/`day_end`. `end` is at most `"23:59"`: a window past midnight is not representable
  (a student who works past midnight gets `23:59`; named, not solved).
- Exactly one is used: if several `planning-day` notes are confirmed, the lowest `id` wins and the
  rest are named in one warning.
- The student edits it at any time — by hand now, by the phase-2 editor later (§6.4) — through the
  engine's `write` like any note. No agent ever writes `window` on a note that exists: routines
  found later never override the student's window (§3.5).
- The note syncs like every note. That is the student's own setting, confirmed by them.

### 2.5 Who writes what

- **At confirmation** (a card's settlement, the phase-2 screen, the registrar pass for R24 rows):
  `write::create` of the whole note, actor `agent:commitments` for a settlement (the student's
  decision is the journal record on the card) or the console's human context for the screen.
- **After confirmation** no agent writes the note except by the settlement of an approved change
  card (§5.4). `kind`, `level`, `title`, `course` and `status` are never written by an agent after
  creation; `journal.human_set` protects anything the student edited.
- **Duplicates.** If two confirmed notes share a `source_uid` (two desktops confirming before
  either syncs, or a race between the `commitments` command and a slot), `commitments::load` keeps
  the lowest `id`, ignores the rest and names them in one warning, and the settlement never creates
  a third. Nothing is deleted automatically; the doubled span would subtract nothing extra anyway
  (§6.1).

---

## 3. Sources, the series file and the classifier

### 3.1 Sources

| `source_uid` | from | phase |
|---|---|---|
| `gcal-series:<recurringEventId>` | the Google grant, through `/ingest-calendar?name=google&accepts=series` (§4.1), owned calendars only (R5) | 1 |
| `gcal-series:<id>` / `ics-series:<UID>` | any ICS feed the vault reads (the secret iCal address as `cloud:personal`, or a direct URL): a master `VEVENT` with an `RRULE`. A UID ending in `@google.com` has the suffix stripped and is keyed `gcal-series:`, so one series seen through both routes is one key (R6). | 1 |
| `registrar:ua:<term>-<crn>` | the UA registrar (Q7, R24), fetched on the device | 3 (pre-pilot) |
| `card:<course-slug>` | the answer to a fallback card (§5.3); written confirmed | 2 |
| *(none)* | a note the student wrote | any |

### 3.2 Normalising a series

Every source is reduced to one record, so the classifier has one input shape:

```
Series { source_uid, calendar, title, where, event_type, rule: {freq, interval, until, count},
         has_master, instances: [(date, start, end)], meets: [(days, start, end)], first, until,
         last_seen }
```

1. **Instances**, in the vault's timezone (`config/ingest.yaml`'s `timezone:`, as `calfeed` does),
   over the horizon `[today, today + 28 days)`.
   - *Google:* the reply's `instances` (§4.1). Cancelled instances are not in them (Google omits
     them from `singleEvents=true` without `showDeleted`); moved ones appear at their new time.
   - *ICS:* **new logic in `calfeed::weekly_series`**, not the existing busy-time path. Masters and
     overrides are grouped by `UID`; the master is expanded with `calfeed`'s recurrence helpers,
     `EXDATE`s removed; an override `VEVENT` (one carrying `RECURRENCE-ID`) replaces the instance
     it names, or removes it if it has `STATUS:CANCELLED`. A master with `STATUS:CANCELLED`, with
     `TRANSP:TRANSPARENT`, or with an `ATTENDEE` line for the feed owner carrying
     `PARTSTAT=DECLINED` yields no series. (The feed owner is the address a calendar-level
     `X-WR-CALNAME` names, when it is email-shaped — Google names a primary calendar so — and is
     **never** inferred from `ORGANIZER` lines, since a feed whose invites all come from one
     person would make that person the owner (P4 review I1, controller ruling). Without it the
     owner is unknown, and any `DECLINED` attendee line, on the master or an override, makes the
     series ineligible — when in doubt, do not propose; the cost is that some invite-based
     series reach the student as a card later instead of a proposal now.) Each rule has its own
     test.
   - `parse_calendar_ics`'s busy time keeps today's override-blind behaviour: it feeds the frozen
     `calendar-snapshot-gcal.md` reference, and a doubled busy span subtracts nothing extra.
2. **Rule.** From the master's `RRULE`: `FREQ`, `INTERVAL` (absent = 1), `UNTIL` (the raw value,
   converted on the device to a date in the vault's timezone — `UNTIL=20261205T055959Z` is
   4 December in Chicago), `COUNT`. An `RDATE` on the master makes the series irregular.
   `has_master` is false when no master was seen (Google: the masters call failed or missed it).
3. **Meets.** Count each `(weekday, start, end)` triple among the instances; keep triples seen at
   least twice; group kept triples by `(start, end)` into `{days, start, end}` entries, days in
   `DAY_KEYS` order, entries ordered by `(first day, start)`.
4. **`first`** = the master's start date. **`until`** = the `UNTIL` date; else, with `COUNT`, the
   date of the rule's last occurrence, expanded on the device from the master's start; else none.
5. **`where`** = the location, trimmed; else the first description line that **looks like a
   place**: at most 80 characters, no `://` or `www.`, none of `zoom`, `teams`, `meet.google`,
   `webex`, `pwd`, `passcode`, `password`, `pin`, `meeting id` (case-insensitive), and no run of
   six or more digits. Otherwise no `where`. The description goes no further (R4).
6. **`calendar`** = the opaque calendar key the reply names (Google, §4.1) or the feed name (ICS).
   **`last_seen`** = today for every series a fresh, complete read of its calendar returned.

### 3.3 The series file — `state/calendar-series.json`

Generated state, like `state/calendar.md`: not a note, never journaled, **never synced** (C3′ sends
only `state/journal/` records and notes under `NOTE_FOLDERS`), rebuilt from the sources. It is the
only place a proposal exists before a card asks about it (R18). It holds **every** recurring
series a fresh read returned, eligible or not: eligibility and classification are decided later,
each run, by §3.4 (re-review M-g).

```json
{"calendars": {"google:3b9e0c1d2a4f5e60": "2026-09-24", "personal": "2026-09-24"}, "ended": {},
 "series": [{"calendar": "google:3b9e0c1d2a4f5e60", "event_type": "default", "first": "2026-08-19",
  "instances": [["2026-09-24", "12:00", "12:50"]], "last_seen": "2026-09-24",
  "meets": [{"days": ["mon", "wed", "fri"], "end": "12:50", "start": "12:00"}],
  "rule": {"count": null, "freq": "WEEKLY", "interval": 1, "until": "2026-12-04"},
  "source_uid": "gcal-series:4k2q9x7m1abc", "title": "CS 100", "until": "2026-12-04",
  "where": "Room 101"}]}
```

- `calendars` maps each calendar key to the date it was last read **freshly and completely**.
  Series sorted by `source_uid`; keys sorted, as `ledger::dumps_value` always sorts them; a trailing
  newline. The same data is the same bytes.
- **Per-calendar fallback.** A calendar read fresh and complete this run replaces its series; one
  it no longer returns is kept with its old `last_seen` until that is 14 days old, then dropped
  (§5.4 files the end card first). A calendar not read this run — failed, partial (§4.1), hidden,
  over the cap, or a Google reply that carried no `series` field — keeps its series and its date
  untouched, so a series is never "absent" because its calendar was not looked at (review I6).
- **Fetched with zero series is a fresh read** (re-review M-e). An ICS feed whose text was fetched
  and parsed as a calendar (`BEGIN:VCALENDAR` present) but holds no recurring master is read
  fresh and complete: its date moves to today and its old series start to age out, so a student
  who deleted every class from that calendar sees them end. Only a failed fetch, or text that is not
  a calendar, is "not read". A Google calendar listed in `calendars_read` with no items is the same
  fresh, empty read.
- **A removed feed** (a `calendars:` entry deleted from `config/ingest.yaml`) keeps its series for
  14 days after its last read date, then they are dropped; no end card is filed for them, since the
  student removed the source, not the class.
- **Ended series are retained** (plan review I1). A series dropped because its calendar was read
  fresh and it went unseen for 14 days — not one dropped with a removed feed, and not a key still
  held under another calendar — moves to `ended[source_uid] = {calendar, dropped, last_instance,
  until}` (its last instance date the file held, and its last-known `until`). The entry goes 28
  days after `dropped`, or at once if the key comes back. §5.4's **ended** rule reads it, so the end
  card can be computed after the series itself left, and filed on a later run if the cap was full.
- **One key, two calendars** (plan review I2). R6 keys a Google series and its ICS twin alike, so
  one `source_uid` may be held under a `google:` calendar and an ICS feed. Both records are kept
  (series sorted by `source_uid`, then calendar), each aging with its own calendar; every reader
  takes **one per key** — a `google:` calendar first, then the lower calendar key — so one key is
  one set of instances, one proposal and one card. A key "left the file" only when no calendar
  holds it.
- **Written only when some calendar produced a fresh result and the bytes differ.** A vault whose
  feeds are all unconfigured or unreachable — every oracle fixture — never gets the file. A
  **missing** file is silent and empty; an unreadable or malformed one is empty with one warning
  and is rewritten on the next fresh fetch.

### 3.4 The classifier

Pure: `classify(series, courses, planning) -> Option<Class>`, where `Class` is `(kind, course)` or
a routine side (`wake`/`bed`). **Eligible** only if all hold:

- `event_type` is `default` or absent (ICS) — never `fromGmail`, `outOfOffice`, `focusTime`,
  `workingLocation`;
- `has_master`, `rule.freq` is `WEEKLY`, `rule.interval` is 1, no `RDATE`, and the rule is one
  `calfeed`'s recurrence helpers support: a rule `occurrence_starts` refuses as
  `UnsupportedRule` (a key outside `RRULE_ALLOWED_KEYS`, such as `BYMONTHDAY` or `BYSETPOS`) makes
  the series ineligible (re-review, C4 residual). For a Google series the device applies the same
  test to the master's `RRULE` line;
- the instances are timed, not all-day, and none crosses midnight (except for rule 1 below);
- at least two instances in the 28-day horizon, and for every kept triple, consecutive instances
  are a multiple of 7 days apart with at least one pair exactly 7 apart (a holiday gap is allowed;
  an every-other-week series is not);
- `meets` is non-empty, and `until`, if any, is not before today.

Everything else — one-offs, biweekly and irregular series, a series whose master is missing,
anything the student declined — is **never** proposed (C5). The first rule that matches wins;
matching is on the title with surrounding punctuation trimmed, case-insensitive unless stated.

0. **Not proposed at all.** The title contains a word or phrase that marks the student's own work
   time — `study`, `studying`, `homework`, `hw`, `review`, `prep`, `tutoring`, `focus`, `work on`,
   `working on`, `work session`, `work block`, `work time` — or equals the name of a
   `config/planning.yaml` `recurring:` entry (already budgeted in hours; counting it again as busy
   time would double it). These are piece 3's `task-block` territory. **Ruling (fix round 1, I3):**
   "Work session", "Work block", "Work time", "Work on …" and "Working on …" are the student's own
   study time, in any spacing and any case — never kind `work`, and never proposed as a commitment.
   Rule 4's `work` is reserved for a job shift: a bare "Work", "Shift", "Work @ <place>" or
   "<Employer> shift". When unsure, not `work`.
1. **Routine.** The whole title is one of `wake`, `wake up`, `get up`, `alarm` → wake side; or
   `bed`, `bedtime`, `go to bed`, `sleep`, `lights out` → bed side. Whole-title only, so "Sleep
   study" is not a routine. A `sleep` series that crosses midnight is both: its start is the bed
   side for its start day, its end the wake side for the next day. Routines feed §3.5's window
   proposal, never a commitment.
2. **Office hours.** `\boffice hours?\b`, or the token `OH` in capitals (case-sensitive, so "oh"
   never matches) → `office-hours`, with `course` when rule 3's code test also finds one.
3. **Course.** The title **starts with** a code in the vault's code table (below), and what follows
   it is empty, or a section word — `lab`, `laboratory`, `lecture`, `lec`, `recitation`, `rec`,
   `discussion`, `disc`, `seminar`, `section`, `sec`, `studio`, `class` — optionally with a section
   number, or words that all appear in that course note's own `name`/`title` ("CS 100 – Intro to
   Computer Science"). Separators between code and remainder are space, `-`, `–`, `:`, `(`, `.`.
   `lab`/`laboratory` → `lab`; otherwise `class`; `course` = the course's slug. A title that starts
   with a known code but continues with anything else ("CS 100 TA hours") is not a class and falls
   through to rules 4–6. A section word's own trailing token counts only when it is a section
   number, not a second word: it holds a digit, or is a single letter — "CS 100 Lab 01" and "CS 100
   Sec 1" are a class, "CS 100 Lab Hours" and "CS 100 Class Party" are not. **Ruling (fix round 1,
   I4):** a remainder that is a bare section designator, with no leading section word, is also a
   class — `-001`, ` 001`, `.001`, `001 LEC`, `LEC`, `SEC 1`. A lab designator such as `LAB`, `L01`
   or `-L01` makes it a lab instead.
4. **Work.** The title starts with the word `work`, or has the word `shift` → `work`.
5. **Club.** The title has the word `club`, `society`, `team`, `practice`, `rehearsal` or `chapter`
   → `club`.
6. **Otherwise** → `meeting`.

**The vault's code table** (C3, R7) is read, never fetched, in this order, first mapping wins:

1. every `courses/*.md` note, in **file-name order**: its `code`, `title` and `name` fields and its
   slug (`cs-100` → `CS100`), each reduced to a code by the **leading-code reading** —
   `^[A-Za-z]{2,8}[ ._-]?[0-9]{1,4}[A-Za-z]?\b` at the start of the string — and, as a fallback
   for LMS names like `202640-BUI-100-101`, by c1c D4's reading anywhere in the string;
2. every `config/ingest.yaml` `course_map` key that reduces to a code, mapped to its slug.

Codes are compared in **compact form**: capitals, separators removed (`CS 100`, `cs-100`, `CS100`
→ `CS100`; `COMPSCI 61A` → `COMPSCI61A`; `CS 1110` → `CS1110`). A title's leading code is reduced
the same way. A compact code claimed by two different courses is dropped with one warning — when in
doubt, do not propose. A vault with no courses has no classes until its courses exist. **Blind
spots, named:** codes with no letters (`01:198:111`), codes with two separators inside (`CS-UY
1114`), and titles that put the code after words ("Intro to CS (CS 100)"). Those classes are not
pre-filled; the fallback card (§5.3) or the registrar asks instead. Tests pin a four-digit school
(`CS 1110`), a long department (`COMPSCI 61A`), a two-digit number (`MATH 20A`), and that "Work on
CS 100", "CS 100 study group" and "Study for PH 106" are never proposed.

### 3.5 Proposals

`commitments::proposals(series_file, notes, courses, planning, template, today) -> Vec<Proposal>`,
pure, in `source_uid` order. A series becomes a proposal only if it is eligible and classified,
and:

- **no note** in `commitments/` has its `source_uid` — a confirmed note or a decline marker (the
  never-re-ask rule for proposals);
- **no confirmed note has its signature** `(kind, course or lower-cased title, meets)` — the same
  class from a second route, a registrar row, a hand-written note, or a "this and following"
  successor (R22);
- it is not `office-hours` when the caller is the card emitter (R8; the screen lists them).

Proposals carry what a confirmed note needs: `kind`, default `level`, `title`, `course`, `meets`,
`where`, `from` (= `first`), `until`, `source_uid`.

**The window proposal.** When the vault has no `planning-day` note and no `window` decline marker
(§2.3), the eligible wake and bed series propose one: for each
weekday, `start` = the latest wake-side time that day (a wake event's end), `end` = the earliest
bed-side time (a bed event's start); a weekday with only one side takes the other from the
template; a weekday whose result is inverted or shorter than `min_block_minutes`, or has neither
side, is left out. It is one proposal whose `source_uid`s are the routine series' keys. A vault
that already has a `planning-day` note never gets another proposal: the student's window stands.
**After a rejection** (re-review M-i): rejecting a window card writes the one `window` marker, and
no window is proposed again, whatever routine series appear later — the proposal is suppressed as a
whole, never rebuilt from the series the student has not yet seen. The student sets the window in
the phase-2 editor instead. *Reason:* the student answered "not these hours", not "not these
series", and a second window card built from one new alarm would ask the same question again.

Nothing in §3 writes a note, a card or a journal record. Proposals are recomputed each run.

---

## 4. The transport change

### 4.1 The cloud: `/ingest-calendar` learns series

`GET /ingest-calendar?name=google&accepts=series`. The function is a `GET`, so the capability list
is a query parameter rather than stream J's body field: `accepts` is a comma-separated list, unknown
words are ignored, and **without `series` in it the reply is byte-for-byte today's** (`{ics,
source}`). With it, and only for `name=google`, the reply gains one field:

```json
{"ics": "BEGIN:VCALENDAR…", "source": "google_calendar",
 "series": {"calendars_read": ["google:3b9e0c1d2a4f5e60"],
  "items": [{"calendar": "google:3b9e0c1d2a4f5e60", "id": "4k2q9x7m1abc", "title": "CS 100",
             "location": "", "description": "Room 101", "event_type": "default",
             "first": "2026-08-19T12:00:00-05:00",
             "recurrence": ["RRULE:FREQ=WEEKLY;BYDAY=MO,WE,FR;UNTIL=20261205T055959Z"],
             "instances": [{"start": "2026-09-23T17:00:00Z", "end": "2026-09-23T17:50:00Z"}]}]}}
```

- **Which calendars (R5).** From `calendarList`: `primary`, then entries with `accessRole: owner`
  and not `hidden`, ordered primary first and then by calendar id; the first 10. Each is named in
  the reply only by `google:` + the first 16 hex of `sha256(account_id + "\n" + calendar id)` — an
  identity for "the same calendar as last time", and nothing else.
- **Which events.** Per calendar, `events.list` with `singleEvents=true` over a window from 24
  hours before now to 28 days after (plan review M7: an instance held earlier today is still seen,
  in any timezone; the device keeps `[today, today + 28)`, §3.2), **following `nextPageToken`** up
  to 5 pages; keep items that carry a `recurringEventId`,
  have a `dateTime` start, are not `status: cancelled`, and whose `self: true` attendee, if any, has
  not declined. `eventType` passes through (`default` when Google omits it); the device, not the
  function, decides eligibility (§3.4), so the rule lives in one place.
- **Masters.** Per calendar, `events.list` with `singleEvents=false` over the same window (same
  pagination) gives each series' master: its `recurrence` lines, **filtered to `RRULE`, `EXDATE`
  and `RDATE`**, verbatim, and its start as `first`. `UNTIL` and `COUNT` are interpreted on the
  device (§3.2), in the vault's timezone. A series whose master was not returned is sent without
  `recurrence` and is ineligible.
- **Complete or not at all.** A calendar goes into `calendars_read` only if every page of both
  calls was read and none of its series was cut by a cap. A calendar that ran past 5 pages, or
  whose series would pass the 100-series cap (applied in calendar order, then series id order),
  is left out of `calendars_read` **and its items are not sent** — its series on the device stay
  as they were (§3.3). A series with more than 40 instances in the window is dropped (it cannot be
  weekly-eligible).
- **Bounds.** `title` and `location` cut to 200 characters; **`description` sent only when
  `location` is empty, cut to 200 characters** (R4). Nothing is written to a table or a log; error
  lines name only exception classes, as today.
- **A time budget.** Series gathering (up to ~40 Google calls) runs under its own 5-second
  wall-clock budget after `ics` is built. Past it, or on any failure in it, `series` is **omitted**
  and `ics` is returned exactly as today — a series problem never costs the day's busy time.
- **The `ics` field does not change**: still `primary` only, still `toIcs`'s five properties. A
  test pins `toIcs` output byte-for-byte against today's for the same input.
- **`name=personal`** gets no `series` field: its ICS already carries the masters, overrides and
  RRULEs, and the device derives series from it (§3.2).

### 4.2 The engine

- `cloudmodel::fetch_calendar(client, name)` sends `&accepts=series` and returns `(ics,
  Option<serde_json::Value>)`; the absence of `series` is `None`. `engine/tests/cloud_contract.rs`'s
  request-line assertion is updated to the new line in the same commit, and a second assertion
  pins that a reply without `series` parses.
- **The stash is explicit** (review I8). `cli::Fetchers` gains `series: Option<&'a SeriesStash>`,
  where `SeriesStash` is a `RefCell<BTreeMap<String, StashEntry>>` keyed by the feed's **URL** as
  the calendar closure receives it (`cloud:google`, a direct `https://` address), and `StashEntry`
  is the Google `series` value or the fetched ICS text. `cli::run` owns one stash and its calendar
  closure fills it; `run_with`, the seam every test drives, takes it through `Fetchers`, so tests
  inject series without a network. `Fetchers` derives `Default`; the one struct literal outside the
  engine, `app/tests/scaffold.rs:99`, gains `..Default::default()` (listed in §10). A feed that
  fails stashes nothing. URLs map to feed names through `calfeed::calendar_entries`.
- The calendar closure keeps its `Fn(&str) -> Result<String, String>` shape, so
  `calfeed::load_calendar_events` and every oracle test are untouched.
- After `load_calendar_events`, `commitments::refresh_series(vault, stash, tz, today)` normalises
  (§3.2) — Google values directly, ICS text through the new pure `calfeed::weekly_series` — and
  updates `state/calendar-series.json` (§3.3). Then the card passes of §5 run.

### 4.3 Compatibility

| engine | function | result |
|---|---|---|
| old | new | no `accepts` → today's reply; the old engine never learns series. |
| new | old | `series` absent → `None` → that feed's series keep their previous state; no proposals, no error. |
| new, no account | — | direct ICS feeds still yield series (R6); `cloud:` feeds degrade to the snapshot as today. |
| new | new, Google not connected | the existing 409 → "using snapshot"; series unchanged. |
| old engine, a vault a new engine wrote | — | `commitments/` is an unknown folder: ignored by ranking, id repair and backups; capacity is what it was before this spec. |
| old engine on a second desktop, after C3′ | — | its sync refuses pulled `cmt_` ids and `commitments/` paths (`is_id`, `is_note_path`) and does not get them back after it updates. The updated desktop then re-proposes those series (no local note has their key), the student approves once more, and §2.5's duplicate rule absorbs the pair when both sync. Bounded by the release gate (R3) and auto-update; accepted and named. |

---

## 5. Confirmation paths and the cap

### 5.1 The onboarding confirm screen (phase 2)

Lands after c1b, c1c and C3′ merge (they own `app/static/*` and `app/src/onboarding.rs`).

- Shown right after the calendar connection step, before the first slot. It runs
  `knowlu-engine commitments --vault <v> --json` (R14): fetch, refresh the series file, print the
  proposals — **no note is written**. A fetch that yields nothing shows only *Your day* (below).
- **Grouped:** *Your classes* — **one row per series**, labelled with the course code, days and
  times in the §5.2 format, so a lecture and its lab are two rows; *Your week* (work, clubs,
  meetings); *Your day* — the planning window (Q6), pre-filled per weekday from the routines when
  there are any, otherwise from 08:00–22:00 (the observed case), with the times editable by pickers;
  *Office hours*.
- **Each row has three states:** *mine*, *not mine*, and unanswered. **Pre-set to mine:** every
  class, lab and work row. **Left unanswered:** clubs, meetings, office hours. *Not mine* is only
  ever the student's explicit choice (re-review M-b). One control per row changes the level (hard /
  soft / optional). Nothing asks for typing.
- A course the wizard captured with no class row is listed under *Your classes* as "no class times
  found — Knowlu will ask this week" (§5.3), or at UA offers the registrar login (phase 3).
- **Finish** confirms every *mine* row (a confirmed note, with its chosen level), declines every
  *not mine* row (a decline marker, R19), writes nothing for an unanswered row, and writes the
  `planning-day` note. An unanswered club or meeting reaches the student later as an ordinary §5.2
  card (after the classes, inside the 5-a-day pace); an unanswered office-hours row stays listed
  in the *Your week* panel (R8). *Reason:* a default must never decline anything permanently; a
  student who clicks Finish without reading has declined nothing. The writes go through
  the engine, which re-derives each proposal from the series file by its `source_uid` — the app
  passes keys, levels and the window, never event data. The console's human context; **no card, so
  nothing is charged to the cap.** Leaving without Finish writes nothing; the proposals then reach
  the student as §5.2 cards. Whether this is a mode of the `commitments` command or a new app
  command is the phase-2 plan's call; either way the Tauri command count is recounted.

### 5.2 The `commitment-check` card (phase 1)

One card per proposal not yet answered (§3.5), built on F2's `event-check` pattern. **Local-only
(R20).**

- **Emitter.** `commitments::emit_checks(vault, proposals, today, budget, ctx, journal)`, run by
  `rank` after `refresh_series`. It asks about a proposal only if it is not `office-hours` (R8) and
  no card in `approvals/` or `archive/` has its `source_uid` — any card, answered, pending or
  snoozed, closes the question for good. **A withdrawn card (archived `superseded`) does not**
  (re-review M-f): it was never answered, so a series that vanished and later returns is asked
  again. The churn is bounded: a card is withdrawn only after its series has been gone from the
  file for 14 days (§3.3).
- **One constructor.** Every card this piece files — proposal, window, change, and phase 2's
  `commitment-ask` — is written by one function, `commitments::file_card`, which refuses a `kind`
  outside `LOCAL_CARD_KINDS = ["commitment-ask", "commitment-check"]`. That constant is what C3′'s
  sync reads to keep the cards local (R20, §10 Phase 1s).
- **Cap.** `allowance = min(budget, 5 − commitment-check cards whose first_proposed_at is today)`
  (R9). `budget` is `daily_approval_budget − count_proposals_created(vault, today)` after the events
  pass has taken its share, so `defer_over_budget` never has overflow to snooze. Order: change
  cards (§5.4), then `class`, `lab`, `work`, the window, `club`, `meeting`, any other kind; within a
  kind by the first meeting's `(day, start)`, then `source_uid`.
- **Title** (structured fields only, never a model): `{title≤40} · {days} {range} · {question}`.
  `days` joins the first `meets` entry's days as `Mon/Wed/Fri` (three or more consecutive days
  collapse to `Mon–Fri`); `range` uses F2's clock format (`12–12:50pm`, `1–2:45pm`); a second
  `meets` entry adds ` +1 more time`. `question` by kind: `a class?`, `a lab?`, `work?`, `a club?`,
  `a meeting?`. The window: `Your day · Mon–Fri 8am–10pm · plan in this window?`. Example:
  `CS 100 · Mon/Wed/Fri 12–12:50pm · a class?`.
- **Frontmatter:** `type: approval`, `kind: commitment-check`, `title`, `status: pending`,
  `source_uid` (the proposal's; for the window, `window:` + its routine keys joined by `,`),
  `commitment:` — the proposal as one single-line flow mapping (`kind`, `level`, `title`, `course`,
  `meets`, `where`, `from`, `until`; for the window, `window`) — `proposed_at` and
  `first_proposed_at` (both today), `expires: null`, `snooze_until: null`, `created_by:
  agent:commitments`. Written with `write::create`; journal actor `agent:commitments`. Path
  `approvals/commitment-check-<slugify(title)>.md`, `-2` on collision.
- **Body.** First paragraph (the card's `why`): `**Is this part of your week?** Knowlu found it
  repeating on your calendar.` Then what approving does, by level — hard: "Approve and Knowlu never
  plans anything over it."; soft: "Approve and Knowlu counts it as busy; an event suggestion may
  overlap it, and will say so."; optional: "Optional: Knowlu won't plan around it." Then `where`
  when present, and `Reject and it's ignored. Either way you won't be asked again.` The window
  card instead says "Approve to plan inside these hours. Reject to keep your usual hours." and
  then "Either way you won't be asked again.": rejecting it keeps the usual hours, so "ignored"
  would be wrong (controller ruling, P11 fix round 1; for Quinn to revisit).
- **Settlement**, a new arm in `approvals::transition_note` beside F3's `event-check` arm:
  - `approved` → if a confirmed note already has the card's `source_uid` or signature (or, for the
    window, a `planning-day` note exists), the card is archived `refused` with one warning and
    nothing is written. Otherwise `write::create` the confirmed note from `commitment:` (§2.1; the
    window as §2.4), `confirmed_at` today, then the card is stamped `executed` and archived.
  - `rejected` → `write::create` the decline marker (for the window, the one `window` marker of
    §2.3), unless one exists; then the generic archive.
  - `pending` / `snoozed` → unchanged machinery. There is no expiry (R9).
  - **Withdrawn** (M14), by card shape (plan review C1): a pending card is archived `superseded`
    with no write only when its question went away — a **proposal** card when its key is under no
    calendar of the series file, a note has its key, or a confirmed note has its signature; a
    **window** card when a `planning-day` note or the `window` marker exists; a **change** card
    (§5.4) when its target note is gone or the note no longer holds `was`. Each condition also stops
    the card being proposed again, so no card is withdrawn and re-filed run after run. While a
    window card is not `superseded`, no other window card is filed, whatever its routine keys.
- **The day's count.** Cards filed after `process_approvals` are counted into this run's
  `Approvals: N pending` line, as F2 does for its event-checks: in `cli.rs`, by adding the number
  filed to `approvals.pending` (`j-followups:cli.rs:417` does `approvals.pending += checks`).
  `render.rs` is not edited (re-review M-a).
- **No app change in phase 1.** `decide` writes the card's status and runs `process_approvals` in
  the same process (F3 decision 3); the console renders any `kind` with Approve, Reject and Snooze.
  Choosing a level on the card, or correcting a kind after the fact, is the phase-2 *Your week*
  panel (§10), not typing (review I10).
- **An old engine** meeting an approved `commitment-check` warns `unknown kind` and leaves it;
  reachable only across version skew; accepted and named.

### 5.3 The per-course fallback card (phase 2)

"When does BUI 100 meet?" — the one place the student types. Local-only (R20).

- **Eligible:** a course in `courses/` with no confirmed class note, no pending class proposal and
  no `card:<slug>` decline marker, from **day 3** of the vault (the date of its earliest
  `state/journal/` file), at most **2 a day**, charged to the 15 like any card (R10). At UA, a vault
  with the registrar connected waits for the registrar pass instead.
- **Card:** `kind: commitment-ask`, `course: <slug>`, title `When does BUI 100 meet?`, body naming
  what the answer does and that the student can reject if the course has no meetings (an online
  course). Rejecting writes the decline marker for `card:<slug>`, which closes the question.
- **The answer:** a small form — day toggles Mon…Sun, start and end pickers, "add another time" —
  writes `answer_meets: [{days: [...], start: "HH:MM", end: "HH:MM"}]` onto the card and approves
  it. The settlement validates it as §2.2 validates `meets` and creates `commitments/<slug>.md`
  (`kind: class`, `level: hard`, `course`, `meets`, `source_uid: card:<slug>`, `status:
  confirmed`). An invalid answer returns the card to `pending` with a warning.
- **Withdrawn** (archived `superseded`) if a class proposal for the course appears while it is
  pending; that proposal's own card asks instead.
- The engine half ships with the form, never alone: a card the console cannot answer would teach
  the student to reject.

### 5.4 Changes to confirmed commitments (R12, R22)

The `amend` machinery is not used: `approvals::validate_amendment` refuses a null `from` ("from/to
must both be present and non-null") and any sequence ("must be a scalar"), and those refusals are
recoverable, so an amend of `until` or `meets` would return to `pending` for ever (review C1).
`commitments` does not join `AMENDABLE_FOLDERS`, and `AMENDABLE_FIELDS` is unchanged.

- **Detection** (in `rank`, after `refresh_series`), only for a confirmed note whose `source_uid`
  is a `gcal-series:`/`ics-series:` key and whose calendar was read fresh this run:
  - **changed** — its series differs from the note in `meets`, in a non-empty `where`, or in
    `until` (the series now ends, or ends earlier);
  - **ended** — its series was dropped from the file (14 days unseen while its calendar was read)
    and so is in the file's `ended` map (§3.3), and no fresh series has the note's signature (R22)
    → propose `until` = the last instance date the file held, else its last-known `until` (neither:
    no card); nothing when the note's `until` already ends it by then;
    an open-ended series (no UNTIL, no COUNT) that merely stops appearing never ends a **soft or
    optional** note (club, meeting, office hours) — a club dormant over the summer is not lost; the
    student can delete the note — while hard ones (class, lab, work) keep this rule (controller
    ruling, P12 fix round 1, m1);
  - **succeeded** (R22) — an eligible class or lab series for the same course, with different
    meets, whose first instance falls on or after the note's series' last instance, while that
    series is ending → propose `meets` (and `where`) of the new series **and** `source_uid` = the
    new key, in one card. That new series is not a proposal of its own while the card is pending,
    once it is approved, or once it is rejected (below).
- **The card** is a `commitment-check` with `target: commitments/<file>.md`, `change:` — the new
  values of the changed fields only, one single-line flow mapping — and `was:` — the note's
  current values of the same fields, absent written as `null`. Title: `CS 100 now meets Tue/Thu
  9:30–10:45am · update?` / `CS 100 ends Dec 4 · update?`. `source_uid` is the note's. Local-only
  (R20); charged to the cap and filed before new proposals' cards (§5.2).
- **Settlement** (the same arm): `approved` → for each field in `change`, the note's current value
  (absent = null) and the card's `was` value are compared as parsed, normalised values (read the
  way `load` reads them) — and any mismatch means the student changed the note since: the card is
  archived `superseded` (not `refused`, so a successor is never held forever) with a warning and
  nothing is written. If the note already holds every value in `change` (a re-run after a crash),
  the card is stamped `executed` and nothing is written. Otherwise each field
  is written with `write::write_literals` (a sequence through `to_literal` of the parsed value, so
  it reads back as the sequence `commitments::load` accepts), the card is stamped `executed` and
  archived. `rejected` → archived, no write to the note. No field is ever proposed as `null`.
- **A rejected successor** (re-review M-d): when the rejected card's `change` carries a new
  `source_uid`, the settlement also writes the decline marker for that new key (unless one
  exists), so the successor series is not proposed again as a new class. The old note is then
  handled like any other: when its own series has been gone 14 days, the **ended** rule files one
  end card. That is a different question ("CS 100 ends Dec 4?"), asked once.
- **Never re-asked:** a card with the same `target` and the same canonical `change` text in
  `approvals/` or `archive/` suppresses another; a different change is a new question.
- **Tests:** an `until` change onto a note with no `until` applies; a `meets` change applies and
  reads back as a sequence `load` accepts; a stale `was` is superseded and archived; a split series
  with the same meets files nothing; a split with new meets files exactly one card.

### 5.5 Never re-asked

| situation | what closes it |
|---|---|
| a series was confirmed or declined | a note with its `source_uid` — the confirmed note or the decline marker (§3.5) |
| the same meeting arrives from a second route | the signature match against confirmed notes (§3.5) |
| a proposal has had a card | any card with its `source_uid`, in `approvals/` or `archive/`, except one withdrawn as `superseded` (§5.2) |
| a successor change was rejected | the decline marker for the successor's key (§5.4) |
| a window proposal was rejected | the `window` marker (§2.3, §3.5) |
| a course was asked when it meets | its `card:<slug>` note, confirmed or a decline marker |
| a confirmed commitment changed | a card with the same `target` and `change` (§5.4), except one withdrawn as `superseded` (the note was edited since; asked once more with the new `was`) |
| the student has a planning day | the `planning-day` note: no window is proposed again |

Cards are local-only (R20), so on a second desktop the rows closed by a card hold only once the
first desktop's answer — a note — has synced. Until then that desktop may ask once; the settlement's
duplicate check (§5.2) makes the second answer a no-op.

---

## 6. How ranking and capacity use it

### 6.1 One constructor for the vault's calendar

Today every production path builds its calendar the same way —
`WeekCalendar::from_file(&vault.join("config").join("week_template.yaml"), events)` in
`cli::run_with` (the `rank` step) and in `surface::load` (the read model). Both become
`WeekCalendar::for_vault(vault, events)`: `from_file`, then
`with_commitments(commitments::load(vault))`, then `with_instances(series file)` (R21). The
test-only `from_file` calls on fixtures in `ranking.rs`, `scheduling.rs`, `render.rs` and
`surface.rs` stay as they are.

`WeekCalendar` keeps the template's `classes`, `day_start` and `day_end` exactly as today and gains:

- `commitment_spans: Vec<Span>` — `Span { day: DayKey, start, end, from: Option<Date>, until:
  Option<Date>, title, kind, source_uid }` — from **confirmed** notes whose level is `hard` or
  `soft` (optional notes, decline markers and the planning day contribute nothing here);
- `instances: BTreeMap<String, (Date, Date, Vec<(Date, Time, Time)>)>` — per `source_uid`, the
  fresh-read horizon `[read date, read date + 28)` of its calendar and its actual instances (R21);
- `window: [Option<(Time, Time)>; 7]` — the planning day per weekday (§6.3).

Then:

- **`window(day)`** returns the planning day's `(start, end)` for `day_key(day)`, or the template's
  `(day_start, day_end)`. Every use of `day_start`/`day_end` on a date goes through it —
  `template_blocks` and `surface::the_day` (lines 892–893 today).
- **`template_blocks(day)`** takes its busy list as the template's spans for `day_key(day)`
  **plus** each commitment span active that day: when `day` is inside the span's source horizon in
  `instances`, the span's actual instances on that date (none on a cancelled date, the moved time on
  a moved one); otherwise the weekly span on its weekday when `from ≤ day ≤ until`. The sort, the
  cursor walk and the clamp to the window's end are unchanged, so overlapping or duplicate spans
  behave exactly as overlapping template classes already do.
- **`free_blocks`, `capacity`, `template_capacity`** follow from `template_blocks` with no change,
  so `rank`, `designate_today`, `start_by`, `slack_days`, the must-do partition and the gauge all
  see commitments and the window with **no ranking change** (C3).
- A class that is both a confirmed commitment and a Google event in `state/calendar.md` is
  subtracted twice; subtracting an already-busy span removes nothing, so capacity is right. The
  visible shift is in `render::capacity_breakdown`'s wording, where that hour moves from "calendar"
  to "template" — correct, since it is now part of the timetable.

### 6.2 The read model

`surface::the_day` draws classes as the gaps between `template_blocks`. With commitments in that
busy list, a club would be drawn as `class`. So (R15): the gap walk runs over a new
`template_only_blocks(day)` (the template's classes alone within `window(day)` — today's exact
computation when there is no planning day), and each commitment span active that day is added as
its own block — `kind: "class"` for class and lab, `"busy"` otherwise, `label` = the note's title —
**clamped to `window(day)`, and not drawn if wholly outside it**. A `busy` block from `events_on(day)`
whose start and end equal a commitment block's is dropped, so a class on both Google and in
`commitments/` is drawn once. Both kinds already exist, so the console renders them unchanged;
`docs/surface/anatomy.md` gains one paragraph on where these blocks come from and on `moved`
(§6.4).

### 6.3 The planning day (Q6, C2, R11)

`with_commitments` reads the one confirmed `planning-day` note (§2.4) into `window`. A weekday the
note lists uses its `start`–`end`; any other weekday keeps the template's `day_start`/`day_end`. An
entry with `start >= end` or an unparseable time is skipped with the warning `planning day
<days>: <start>–<end> ignored; using week_template`. `config/week_template.yaml` is never
written. Routines reach the window only through the note (§3.5): no confirmed note, no change. The
window is per weekday, so a 7:30 weekday wake-up does not also start the student's weekend.

### 6.4 What moved (Q6, R23)

An edit to the window shows its effect **at once** in the console, and the today view says what
moved. Q6 asked for "the next rank"; the console does better than that, because `surface` already
recomputes the day on every refresh (`surface::load` calls `designate_today_explained` live), and
`moved` is computed in the same place (re-review N2). `today.md`, which only `rank` writes, follows
at the next slot as it always has.

- **`state/plan.json`**, generated and device-local, is written by `rank` **only when a confirmed
  `planning-day` note exists** (so a vault without one — every fixture — is byte-identical). It
  holds one thing: the window **the day started with**, `{"date": <today>, "start": "HH:MM",
  "end": "HH:MM"}` through `ledger::dumps_value` — today's effective window, one span, since only
  today is diffed.
- **When `rank` writes it.** If the file's `date` is today, `rank` leaves it alone: the baseline
  stays the day's first window however often the student edits. If the file's `date` is another
  day, `rank` records today's current window (`window(today)`). If the file is **missing**, `rank`
  records the **template's** `(day_start, day_end)` for today — the window the day was planned in
  before any planning-day note existed. `rank` computes no diff and writes no `moved`.
- **What `surface` does, purely and without writing.** The baseline is `state/plan.json`'s window
  when its `date` is today; with a note and no file (or an unreadable one), the template's window
  for today; with a file dated another day (today's first `rank` has not run yet), the current
  window, so nothing is reported; with no note, there is no baseline and no `moved`. When the baseline differs from
  `window(today)`, `surface::load` runs `designate_today_explained` a second time, over a
  `WeekCalendar` identical but for today's window, from the **same** ranked list, and compares the
  two plans by task id. A take's **part of day** is the start of its free block
  (`free_blocks(today)[block_index]`): before 12:00 *morning*, before 17:00 *afternoon*, otherwise
  *evening*. A task counts as **moved to** its new part when its part changed or it is new in the
  current plan; one present only in the baseline plan counts as **no longer fits today**. Both plans
  come from the same ranked list, so the diff is exactly the window's effect.
- `moved` = `{"to": {"morning": n, "afternoon": n, "evening": n}, "dropped": n, "text": "2 items
  moved to this evening"}`, `text` built by the engine from the counts (largest group first, ties in
  morning→evening order; `dropped` adds "; 1 no longer fits today"), or **omitted** when the
  windows are equal or the plans are.
- **How long it shows.** From the edit until the day ends: the baseline is the day's first
  window, so a second edit is reported against the morning's plan, not the first edit, and an edit
  undone reports nothing. The next day's first `rank` records that day's window and the line is
  gone.
- **What the first window reports.** On the day a planning-day note first appears (a window card
  approved, or a note written by hand), no `state/plan.json` exists yet, so the baseline is the
  template's 08:00–18:00. Approving a Mon–Fri 08:00–22:00 window on a weekday therefore shows, at the
  next refresh, the evening the new window opened — "3 items moved to this evening" — and the
  first `rank` after it records 08:00–18:00 for today, so the line stays for the rest of that day.
  On a desktop that syncs the note from another one, its first day with the note says the same
  once; accepted and named.
- **The read model** gains an optional `moved` on the today view, omitted when there is none — so
  `surface-today-{s1,s1-migrated,full}.json` do not change (their vaults have no note).
- **Preview.** `surface --view today --window '<flow sequence>'` computes the day under a proposed
  window and its `moved` against the **current** window (not the baseline), and writes nothing. It
  is the phase-2 editor's data source ("see how an edit changes the suggestions"); the editor itself
  — pickers, the preview, the edit through `set_fields` — is **app phase** (phase 2).
- `today.md` gains nothing: its format is the golden references' contract; the line is the
  console's.

### 6.5 Warnings and determinism

`commitments::load` never fails: a missing folder is empty, an unreadable note or invalid field is
skipped with a warning, notes are read in file-name order. Its warnings, the series pass's and the
card passes' join the `calendar` step's message in the run record (R13). Nothing in §6 reads the
clock, the network or a model; `rank_cannot_reach_a_judgment_endpoint`, which today scans
`cli.rs`, is extended to `commitments.rs` and the `commitments` command's arm in `main.rs` (M5).

---

## 7. The overlap API for piece 2

In `engine/src/commitments.rs`, pure, no I/O:

```rust
pub enum Level { Hard, Soft, Optional }

/// Every confirmed commitment whose meeting overlaps [start, end) — half-open, local wall-clock,
/// in the vault's timezone, honouring actual instances inside the fresh horizon (R21). Sorted by
/// (level: hard first, meeting start, title, id). The planning day is not a commitment here.
pub fn conflicts<'a>(set: &'a Commitments, start: DateTime, end: DateTime)
    -> Vec<(&'a Commitment, Level)>;

pub enum Fit { Clear, OverlapsSoft(Vec<String>), OverlapsHard(Vec<String>) }

/// The rule piece 2 applies: any hard conflict → OverlapsHard (never propose); else any soft →
/// OverlapsSoft (propose; the card names them); optional conflicts are ignored → Clear.
pub fn fit(conflicts: &[(&Commitment, Level)]) -> Fit;
```

- `Commitments` is the `load(vault)` result plus the series file's instances — the same data
  `WeekCalendar::for_vault` uses, so the overlap rule and the capacity rule can never disagree.
- A span that crosses midnight is split at midnight and each part is checked against its own date.
- `conflicts` returns optional commitments too, so a caller can say "during office hours"; `fit`
  ignores them.
- Tests pin: a hard class overlapping by one minute is `OverlapsHard`; touching end-to-start is
  `Clear`; a soft club gives `OverlapsSoft(["Chess Club"])`; decline markers never conflict; a span
  outside `from`/`until` is `Clear`; a cancelled instance inside the horizon is `Clear`; a
  midnight-crossing span meets the next day's commitment.

---

## 8. What cannot change

- **The eight frozen Python references** (CLAUDE.md rule 2). `golden-today-s1.md` and
  `golden-today-full.md`: the fixture vaults have no `commitments/`, no `courses/` and no reachable
  feed, so the calendar, window, capacity and page are computed exactly as today, and `today.md`
  gains no line (§6.4). `calendar-snapshot-gcal.md`: `write_snapshot`, `read_snapshot` and
  `parse_calendar_ics` are not edited; series use new code and a different file.
  `vault-full/state/events.md`, the parsed references, `run-records-reference.json` (no new step,
  R13) and `pyyaml-safe-dump-reference.json` (`meets`, `window`, `commitment`, `change` and `was`
  go through the existing flow emitter) are untouched by construction.
- **The three read-model references** `surface-today-{s1,s1-migrated,full}.json`: no commitment
  blocks, no planning day and no `state/plan.json` exist for them; `template_only_blocks` is
  today's computation; `moved` is omitted. They are not regenerated.
- **Byte contracts.** No new `journal::VIAS` entry (`agent:commitments` is an actor, and
  `provenance::is_agent` is a `starts_with` test). Every JSON line goes through
  `ledger::dumps_value`. `/ingest-calendar` without `accepts=series` replies byte-for-byte as
  today, and `toIcs` is pinned.
- **Old engines** — §4.3.
- **A vault with no `commitments/`.** By construction `commitment_spans` is empty and `window`
  falls back to the template, so `template_blocks` builds the identical busy list; the
  de-duplication in §6.2 fires only beside a commitment block; with no configured, reachable feed
  no series file is written; with no planning-day note no plan file is written. Tests prove it:
  1. `for_vault_equals_from_file_without_commitments` — for each of `vault-s1`,
     `vault-s1-migrated` and `vault-full`, over the 35 days from 2026-08-24, `for_vault` and
     `from_file` give equal `template_blocks`, `free_blocks`, `capacity`, `template_capacity` and
     `window(day)`.
  2. `template_only_blocks_equals_template_blocks_without_commitments` — the same fixtures and
     days (M7).
  3. `decline_markers_change_nothing` — the first comparison on a scratch vault whose
     `commitments/` holds only decline markers.
  4. `cargo test --test oracle --test surface_oracle` green with `git diff --exit-code
     engine/tests/fixtures` clean.
  5. `rank_on_vault_full_writes_no_new_state` — a copy of `vault-full` ranked with the fixture's
     feed configuration gains no `state/calendar-series.json`, no `state/plan.json`, no
     `commitments/` and no card, and its `calendar` step message equals today's (M7).
- **`ids::NOTE_FOLDERS` grows to seven.** Id repair and backups iterate it and already skip a
  missing folder; `note_folders_is_a_subset_of_backup_folders` keeps checking the derivation, and
  `BACKUP_FOLDERS` grows to ten.

---

## 9. Privacy

- **What leaves the device:** nothing new. The request gains `accepts=series`. Proposals, the
  series file and the cards never leave (R18, R20).
- **What our function reads from Google:** the recurring events on the calendars the student
  **owns** (R5) — title, times, location, description, event type, recurrence rule and the
  student's own response — all within `calendar.readonly`, which the account already holds. It is
  read in memory for the request, sent to the device, and **not stored or logged**; error lines
  name exception classes only.
- **What reaches the device:** per series, the title, times, location, event type, recurrence
  lines, and — only when the location is empty — at most 200 characters of description (R4).
- **What the device keeps and never sends to our servers:** `state/calendar-series.json` — the
  title, a place-like `where` and the times of **every** repeating event a fresh read returned from
  the student's own calendars, eligible or not, including ones they never confirm — and the
  local-only cards. **The description is never written anywhere.** These stay on the student's
  computer **and in the student's own backups**: `backup::mirror` copies `state/` and `approvals/`
  (`BACKUP_FOLDERS`) to the backup target the student chose, and the legacy `history.rs` stages
  both for a git remote the student configured. Neither is our server (re-review M-g).
- **What syncs** (after C3′, under the 2026-09-17 amendment's ruling 2 that the account holds the
  vault), and nothing else from this piece: **confirmed** commitment notes (title, kind, level,
  course, times, `where`, dates, source key); **decline markers** (an opaque source key and
  nothing more); the `planning-day` note (the student's hours); and the journal records of those
  writes. A therapy group, a shift on someone else's calendar or a declined club never reaches our
  servers. All of it is readable by the service, exported with the account and deleted with it.
- **The registrar (phase 3, UA):** the login stays in Credential Manager on the device and the
  fetch runs on the device; only rows that become confirmed notes sync, as above.
- **No model.** Nothing in this piece sends calendar or registrar data to an inference provider;
  the classifier is deterministic and runs on the device.
- **Limited Use.** Calendar data is used only for the user-facing feature — planning around the
  student's week — which is prominent (every card says it came from the calendar); only what the
  student confirms is transferred, and only to our own service; no human reads it; nothing trains
  on it; export and deletion cover it because it is notes. Data minimisation holds because
  unconfirmed events never leave the device.
- **`site/privacy.html` needs lines before shipping — a release gate, not a follow-up.** Today's
  page (on `main` and on `c3-sync`) says Calendar events go to inference providers for judgments
  and does not say that repeating events are read to learn the week. Two lines, worded for Quinn
  and the lawyer's read (per ruling 6), in substance:
  1. with **phase 1's first release**: *when you connect Google Calendar, Knowlu reads the
     repeating events on your own calendars to learn your week; it asks before keeping any, the
     ones you don't confirm stay on your computer and in your own backups, and an event's
     description is never stored;*
  2. with **whichever of phase 1 and C3′ ships second**: *the ones you confirm, and your
     wake-to-bed hours, are kept as notes in your folder, which your account keeps in step.*
  Phase 3 needs its own line about the registrar login, and it collides with the page's current
  "No campus password" paragraph (§11 Q1).
- **The consent screen.** The `calendar.readonly` justification could name "learns the student's
  class and work schedule from repeating events". Changing it may restart Google's verification,
  so whether and when is Quinn's (§11 Q2).

---

## 10. Phases

### Phase 1 — engine and cloud (planned now; executes after `j-followups` merges, R16)

Delivers: the three note shapes and the loader; series transport, the series file and the ICS
series logic; the classifier and proposals; local-only `commitment-check` cards (proposals, the
window, changes) and their settlement; capacity, the planning day, `state/plan.json` and `moved`;
the read model's blocks and preview; `conflicts`/`fit`; the `commitments` command. The only
`app/**` edit is one test line.

| file | change |
|---|---|
| `engine/src/commitments.rs` | **new** — `Commitment`, `Commitments`, `Level`, `load`, `Series`, `refresh_series`, `classify`, the code table, `proposals`, `LOCAL_CARD_KINDS` and `file_card`, `emit_checks`, change detection, the settlement helpers, the plan baseline and diff, `conflicts`, `fit`; the tripwire test `sync_keeps_every_local_card_kind_local` (Phase 1s) |
| `engine/src/lib.rs` | one `pub mod commitments;` |
| `engine/src/ids.rs` | `NOTE_FOLDERS` + `commitments`; `KINDS`, `ID_RE` + `cmt`; `kind_for` maps `type: commitment` |
| `engine/src/backup.rs` | `BACKUP_FOLDERS` derivation grows by one |
| `engine/src/weekcal.rs` | `for_vault`, `with_commitments`, `with_instances`, `window(day)`, `commitment_spans`, `template_only_blocks` |
| `engine/src/calfeed.rs` | **new** pure `weekly_series` (overrides, cancellations, declines, transparency, rule fields); nothing existing edited but `occurrence_starts` made `pub(crate)` |
| `engine/src/cloudmodel.rs` | `fetch_calendar` sends `accepts=series`, returns the optional value |
| `engine/src/cli.rs` | `Fetchers.series` + `#[derive(Default)]`, `SeriesStash`, the stashing closure; `refresh_series`, change detection and `emit_checks` after `load_calendar_events`; `for_vault`; the `state/plan.json` baseline (no diff); cards filed added to `approvals.pending` (M8, re-review M-a); warnings into the `calendar` step |
| `engine/src/main.rs` | `commitments --vault <v> [--today] [--json]` (always exits 0, writes no note); `surface --window` |
| `engine/src/approvals.rs` | the `commitment-check` arm: create, decline marker, change, withdraw; **not** `AMENDABLE_FOLDERS` |
| `engine/src/eventemit.rs` | `clock` made `pub(crate)` for the card titles (M4) |
| `engine/src/surface.rs` | `for_vault` in `load`; `window(day)` in `the_day`; commitment blocks, clamp, duplicate drop; `moved`, computed live against the baseline (§6.4, re-review N2); the `--window` preview |
| `engine/src/yamlemit.rs` | only if `safe_dump_flow` cannot already emit `meets`, `commitment` and `change` on one line (the plan checks first) |
| `engine/tests/cloud_contract.rs` | the new request line; a reply without `series`; `rank_cannot_reach_a_judgment_endpoint` extended to `commitments.rs` and the command (M5) |
| `app/tests/scaffold.rs` | `..Default::default()` in the one `Fetchers` literal (review I8) |
| `cloud/supabase/functions/ingest-calendar/{handler.ts,handler_test.ts,index.ts}` | `accepts`, `series`, owned calendars, pagination, masters, completeness, caps, time budget, failure omission, `toIcs` pin |
| `docs/surface/anatomy.md` | commitment blocks, `moved`, the `commitment-check` card |
| `CLAUDE.md` | `commitments/` in the vault folder list; the `commitments` command; `surface --window` |

**Phase 1s — sync (with C3′: after it merges, or inside it if phase 1 lands first).** A migration
adding `commitments` to `sync_notes_path_check`; `NOTE_PATH_RE` in
`cloud/supabase/functions/_shared/sync_rows.ts` and its test; `sync.rs`'s local-card predicate
(R20, re-review N1); privacy line 2 (§9).

**The local-card predicate, by construction.** Today `SyncCards::find` recognises a local card
only by `created_by: agent:knowlu.sync` (on disk) or a `create` record under that actor, and
`sync_card_note` gates on the actor's literal. A `commitment-check` card is written by
`agent:commitments` — the same actor as the confirmed notes, which **must** sync — so no
actor-based test can separate them. Phase 1s therefore makes the card's **kind** the second key:

- `sync_card_note` becomes `local_card_note(text) -> Option<Option<String>>`: `Some` when the
  frontmatter's `created_by` is `ACTOR` **or** its `kind` is in `crate::commitments::LOCAL_CARD_KINDS`
  (the substring pre-test widens to `ACTOR` or `kind: commitment-`). It is used in all three places
  the old function was: the `approvals/`/`archive/` scan, and the whole-folder check in the note
  loop.
- The journal pass collects the `id` of every `create` record whose `new.kind` is in
  `LOCAL_CARD_KINDS`, beside the two actor tests, **before** ids and paths are gathered, so every
  later record about the card — `decide`'s status `set`, a snooze, the archive `move`, the
  settlement's `executed`/`refused`/`superseded` stamp — is covered by id, and every path it has
  had by path, exactly as a sync card's are.
- The constant lives in phase 1's `commitments.rs`; `sync.rs` names it and never copies it. A kind
  added to the list (phase 2's `commitment-ask` is in it from the start) is excluded with no edit to
  `sync.rs`. And phase 1's `file_card` refuses any kind outside the list, so no card of this piece
  can exist that the predicate does not see.

**Gates** — three, each failing the build of whichever of phase 1 and C3′ merges second until 1s is
in it:

1. `is_note_path_and_the_servers_regex_agree` (`engine/tests/sync_contract.rs`, C3′'s): the folder
   regex must match `NOTE_FOLDERS` (a refused `commitments/` path would wedge every push,
   R-C3′-exec-12). **As C3′ wrote it, it asserts a hard-coded six-folder literal and cannot fail**
   (plan review C2); phase 1s rewrites it to derive from `NOTE_FOLDERS`. Until then, gate 3's
   second test is what forces the regex and the migration.
2. **`build_push_sends_no_local_card_nor_any_record_about_one`** (`engine/src/sync.rs` `mod tests`,
   phase 1s, beside the regex gate in the hand-off's list). A scratch vault holds, for each kind in
   `LOCAL_CARD_KINDS`, a **pending** card in `approvals/`, an **approved-and-settled** card and a
   **withdrawn** card in `archive/`, each written through `write::create` and settled through
   `process_approvals`, so the journal holds each card's `create`, status `set`, `move` and stamp
   records; plus the confirmed note and the decline marker those settlements wrote. It asserts
   that `build_push` from a fresh cursor sends **no** note row under any card path, and **no**
   record whose `id` is a card's or whose `path`, `old` or `new` is a card path; and
   that it **does** send the confirmed note, the marker, and each one's `create` record. The test
   iterates `LOCAL_CARD_KINDS`, so a kind added later is covered with no edit.
3. **`sync_keeps_every_local_card_kind_local`** (`engine/src/commitments.rs` `mod tests`, phase 1,
   the tripwire): if `engine/src/sync.rs` exists in the crate (`CARGO_MANIFEST_DIR`), its text must
   name `commitments::LOCAL_CARD_KINDS` and contain the test named in 2. It passes while phase 1
   lands before C3′ (no `sync.rs`), and fails the first build in which both exist until the
   predicate and gate 2 are in — so the second merge cannot pass on the regex alone. Beside it,
   `the_servers_note_path_rules_name_every_note_folder` requires `sync_rows.ts` and the latest
   `*sync_note_path_check*.sql` migration, whenever they exist, to carry `NOTE_FOLDERS`' folder
   group — so the second merge cannot pass without the regex and the migration either.

And the migration is live on prod before any release carrying `cmt` ships (R3).

### Phase 2 — the app (after c1b, c1c and C3′ merge)

The onboarding confirm screen (§5.1); a **Your week** panel in the console — every confirmed
commitment with its kind and level as controls, office-hours proposals to confirm, and the planning
day editor with the `surface --window` preview and the today view's `moved` line (Q6); the fallback
card's form and its engine half (§5.3). Files: `app/static/index.html`, `console.js`,
`console.css`, `app/src/onboarding.rs`, `app/src/commands.rs` and `main.rs` (commands, recounted),
`app/tests/{onboarding,commands,static_assets}.rs`, `scripts/wizard-check.py`, and
`engine/src/commitments.rs` / `approvals.rs` / `main.rs` for `commitment-ask` and the screen's
writes. **`app/src/commands.rs::EDITABLE`** (line 119 on `main`) holds none of `window`, `level`
or `kind`, and `set_fields` has no path for a sequence literal: the panel and the editor need both
— the three fields added to `EDITABLE`, and a sequence value validated as §2.2 validates `meets`
and written through `write::to_literal` — each with its test in `app/tests/commands.rs`
(re-review M-c).

### Phase 3 — the UA registrar (before the pilot; Q7)

Its own plan; this is the design it implements. Other schools come after the pilot, each a code
change.

- **Credentials on the device.** The app stores the myBama login in Credential Manager as
  `knowlu/<profile_id>/registrar-ua` (`app/src/credentials.rs`); the vault names it as a
  `credential_target`, exactly as zyBooks and VHL are named, and the engine reads it through
  `wincred.rs`. It never leaves the device.
- **Fetch on the device**, as a third portal inside the `coursework` step — the slot's portal step,
  which already owns this credential pattern, always exits 0, and treats an empty parse as a
  failure, never an empty semester. At most once a day; a failure is a named warning.
- **Output.** Each schedule row with a meeting time becomes a series record keyed
  `registrar:ua:<term>-<crn>` (R1), kind `lab` or `class` from the row's schedule type, `course`
  from the vault's code table (§3.4). Rows that match a vault course are written **confirmed**
  (R24) — the connection screen lists them first; the others become ordinary proposals and cards;
  rows with no meeting time produce nothing. A later fetch that differs files a §5.4 change card; a
  row that disappears (a dropped course) files an end card.
- **Everything downstream** — capacity, overlap, never-re-ask, the signature match against Google
  series — is this spec's, unchanged.
- **Not here:** the login sequence, MFA and parsing — the plan's, after §11 Q1 is answered.

### Conflict check (`git diff --name-only main...<branch>`, 2026-09-23)

| branch | files it shares with this spec's phases | risk |
|---|---|---|
| `j-followups` | **already changed:** `cli.rs`, `cloudmodel.rs`, `eventemit.rs`, `lib.rs`, `tests/cloud_contract.rs`, `CLAUDE.md`; its plan adds `approvals.rs` (F2/F3) | **High if run in parallel** — the same functions in `cli.rs` and `approvals.rs`. Resolved by R16: phase 1 starts after it merges. |
| `c3-sync` | `lib.rs`, `CLAUDE.md` (textual); **semantic:** `sync.rs` pushes every `NOTE_FOLDERS` path and its server check allows six folders; `write.rs`, which phase 1 calls; `site/privacy.html` | Phase 1s and its gates (above). Named in both branches' hand-off. |
| `c1c-first-day` | `CLAUDE.md`, `site/privacy.html`, `app/tests/scaffold.rs` (phase 1's one line); **semantic:** `coursework.rs` and `app/src/scaffold.rs` set the course-note shape §3.4's code table reads | Low: the code table accepts both the old and the D4 shapes; the test line rebases trivially. |
| `c1b-sign-in` | `CLAUDE.md`; `site/privacy.html` (the §9 lines) | Low. |

No branch touches `weekcal.rs`, `surface.rs`, `ids.rs`, `backup.rs`, `calfeed.rs`, `main.rs`,
`render.rs` or `ingest-calendar/`. Phase 1 no longer edits `render.rs` (re-review M-a).

---

## 11. Open questions for Quinn

1. **The UA login and the privacy page.** Q7 puts the myBama password in Credential Manager.
   `site/privacy.html` promises today that "Knowlu never asks for your university sign-in, and
   there is nowhere in the app to type one" — sign-in happens on the school's own page in a window
   Knowlu opens, and the session is thrown away. myBama is the same campus sign-in (and likely
   behind Duo, which an unattended fetch cannot pass). *Recommendation:* keep the promise — fetch
   the schedule through the school's own sign-in window, as the LMS link already does, once per
   term and on demand, storing nothing; if you want the stored login instead, the page's paragraph
   changes before phase 3 ships, with the lawyer's read.
2. **The consent-screen justification.** Adding "learns the student's class and work schedule from
   repeating events" to the `calendar.readonly` justification may restart Google's verification.
   *Recommendation:* change it at the next submission you were making anyway, not on its own;
   the scope itself does not change.

---

## 12. Review (2026-09-23)

The adversarial review (`.superpowers/sdd/2026-09-23-commitment-model/spec-review.md`, on
af4f138) found 4 Critical, 10 Important and 17 Minor issues. Each is listed with what this revision
did. "Fixed" means the spec text now says what the finding asked, or an equivalent that the row
names. The claims were checked against the code: `approvals::validate_amendment` (null and
sequence refusals), `calfeed.rs` (no `RECURRENCE-ID`, `STATUS`, `PARTSTAT` or `TRANSP` handling),
`ingest::slugify` (returns `item`), `cli::Fetchers` (two fields, no `Default`), and
`c3-sync:engine/src/sync.rs` (pushes every journal record and every note under `NOTE_FOLDERS`,
nothing else from `state/`; sync cards stay local).

| # | Finding | Disposition |
|---|---|---|
| C1 | R12's amend cards can never be applied | **Fixed differently:** no amend machinery. Changes are `commitment-check` cards with `change:`/`was:`, settled by their own arm with canonical-text staleness checks and `write_literals` (§5.4, R12). `AMENDABLE_FOLDERS` unchanged. |
| C2 | Proposals and declines are synced data | **Fixed:** proposals exist only in the unsynced series file and local-only cards (R18, R20); only confirmed notes, anonymous decline markers (R19) and the planning day sync. §9 rewritten; two privacy lines made release gates. |
| C3 | Course-code shape assumes one school | **Fixed:** the vault's own code table in compact form, D4 as fallback, blind spots named, tests for four-digit, long-department and two-digit schools (§3.4, R7). |
| C4 | ICS override handling claimed but absent | **Fixed:** `weekly_series` is new logic for overrides, cancellations, declines and transparency, each tested; busy time keeps today's behaviour, with the reason (§3.2 step 1, R6). |
| I1 | Other people's calendars | **Fixed:** owned, non-hidden calendars only, ordered, capped at 10 (R5, §4.1). |
| I2 | Time-blocking and study groups become classes | **Fixed:** code must start the title with only a section word or the course's name after it; study/work-block titles never proposed; one screen row per series (§3.4, §5.1). |
| I3 | Biweekly, count-ended and cancelled series | **Fixed:** recurrence lines sent; interval 1 and 7-day spacing required; `COUNT` gives `until`; actual instances subtract inside the horizon (§3.2, §3.4, R21). |
| I4 | "This and following" split | **Fixed:** signature-seen rule, successor change card moving `source_uid` (R22, §5.4). |
| I5 | `where` from a meeting link | **Fixed:** place-like lines only (§3.2 step 5). |
| I6 | Flicker from caps, pagination, hidden calendars | **Fixed:** deterministic caps, pagination, `calendars_read`, per-calendar freshness, removed-feed rule (§3.3, §4.1). |
| I7 | Warnings on every run from reserved kinds | **Fixed:** reserved kinds declared with levels (R17). |
| I8 | Stash plumbing | **Fixed:** explicit `Fetchers.series`, URL-keyed, `Default`, the app test line listed (§4.2, §10). |
| I9 | Sync wedge and old-engine skew | **Fixed:** named gate test, prod-migration release gate, old-engine behaviour stated (§4.3, §10, R3). |
| I10 | Post-onboarding changes need typing | **Fixed as phase-2 scope:** the *Your week* panel (§10); phase 1 cards offer approve/reject only. |
| M1 | Key order vs `dumps_value` | Fixed (§3.3). |
| M2 | Example bytes | Fixed (R2; examples say they are not byte-exact). |
| M3 | `UNTIL` to date | Fixed: raw lines sent, converted on the device (§3.2 step 2). |
| M4 | Conflict table gaps | Fixed (§10: `eventemit.rs`, `write.rs`, `coursework.rs`, `scaffold.rs`). |
| M5 | Model-reach test scope | Fixed (§6.5, §10). |
| M6 | R13's reason | Fixed (R13). |
| M7 | Missing file silent; two more tests | Fixed (§3.3, §8 tests 2 and 5). |
| M8 | Approvals count | Fixed: counted, as F2 does (§5.2; in `cli.rs` since the re-review's M-a). |
| M9 | `OH`, "practice", course order | Fixed: `OH` case-sensitive; `planning.yaml` names excluded; file-name order (§3.4). |
| M10 | Routines: one window for all days, midnight sleep, pre-check | Fixed by Q6's design: per-weekday window, midnight `sleep` handled, the window is proposed and editable (§2.4, §3.4, §6.3). |
| M11 | Clamping | Fixed (§6.2, R15). |
| M12 | `next_14d` before term | Fixed: 28-day horizon (§3.4). |
| M13 | Duplicate `source_uid` | Fixed: load keeps lowest id, settlement never adds a third (§2.5). |
| M14 | Vanished proposals still carded | Fixed: withdrawn `superseded` (§5.2, R9). |
| M15 | `slugify` never empty | Fixed (§2.2). |
| M16 | Privacy line gate; consent screen is Quinn's | Fixed: release gates (§9); consent screen to §11 Q2. |
| M17 | Series time budget | Fixed: 5-second budget, omit on overrun (§4.1). |

**Quinn's rulings of 2026-09-23** replaced the former §11: Q6 (the planning day) settles the old
question 1 — R11 is rewritten around one editable note, with `state/plan.json` and `moved` for
"what moved" (§2.4, §6.3, §6.4); Q7 (UA registrar before the pilot) settles the old question 2 —
phase 3 is now pre-pilot and UA-only (§10).

### 12.1 Re-review (2026-09-23, on ad44347)

The re-review (`.superpowers/sdd/2026-09-23-commitment-model/spec-rereview.md`) found all four
Criticals and all ten Importants addressed, and raised two new Importants and nine Minors. Each is
listed with what this revision did.

| # | Finding | Disposition |
|---|---|---|
| N1 | The sync gate does not enforce R20: the regex gate alone passes, and `SyncCards` sees only `agent:knowlu.sync` cards | **Fixed:** exclusion by card **kind**, from one constant `commitments::LOCAL_CARD_KINDS` that `sync.rs` names; every card goes through `file_card`, which refuses other kinds; three gates — the regex gate, the behavioural `build_push_sends_no_local_card_nor_any_record_about_one` (pending, settled and withdrawn cards of every kind; records and paths; the confirmed note and marker still sent) and the tripwire `sync_keeps_every_local_card_kind_local` (R20, §5.2, §10 Phase 1s). |
| N2 | "What moved" lags a change the console already shows; the first window reports nothing | **Fixed:** `surface` computes `moved` live against the day's starting window; `rank` only records that baseline; with a note and no `plan.json`, the template window is the baseline, so the first window reports what it opened for the rest of that day (R23, §6.4). |
| M-a | F2's count lives in `cli.rs`, not `render.rs` | **Fixed:** the row moved to `cli.rs`; `render.rs` is not edited (§5.2, §10). |
| M-b | Finish declines every unchecked row for good | **Fixed:** three row states; only an explicit *not mine* writes a marker; unanswered clubs and meetings reach the student as cards, office hours stay in the panel (§5.1). |
| M-c | `EDITABLE` lacks `window`, `level`, `kind` and a sequence path | **Fixed:** named in phase 2 with its tests (§10 Phase 2). |
| M-d | A rejected successor is asked again as a new class | **Fixed:** rejecting it writes the successor key's decline marker; the old note ends through its own end card, a different question (§5.4, §5.5). |
| M-e | An ICS feed that genuinely lost every series never ends them | **Fixed:** fetched-and-parsed with zero series is a fresh read; only a failed fetch or non-calendar text is "not read" (§3.3). |
| M-f | A withdrawn card closes the question for good | **Fixed:** `superseded` cards are exempt; the 14-day retention bounds the churn (R9, §5.2, §5.5). |
| M-g | The series file holds every series; "never sends" needs a qualifier | **Fixed:** both stated; the privacy text says "your computer and your own backups", naming `backup::mirror` and `history.rs` (§3.3, §9). |
| M-h | The shape paragraph omits R24's exception | **Fixed:** the exception is stated there (shape paragraph). |
| M-i | A window proposal after a decline | **Fixed:** one `window` marker suppresses every later window proposal as a whole (§2.3, §3.5, §5.2). |
| (C4 residual) | A rule the recurrence helpers do not support | **Fixed:** a rule `occurrence_starts` refuses makes the series ineligible (§3.4). |

§11 Q1 (the myBama login and the privacy page) stays open for Quinn; it affects only phase 3.
