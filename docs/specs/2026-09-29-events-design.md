# Events (MVP): required events enter the plan, with accept and decline

**Date:** 2026-09-29. **Status: Signed by Quinn, 2026-09-29 (every recommendation accepted).**
Nothing here is built. §12's questions are decided as recommended: Q1 (c), Q1a (i), Q1b (i), Q2 (a),
Q3 (a), Q4 (a), Q5 (c). The plan follows this spec.

**Authority.** `docs/specs/2026-09-09-knowlu-cloud-design.md` and its Amendment 2026-09-29 (signed).
Ruling 10 puts "events (required events become tasks; accept and decline)" in the **MVP**. Then
`VISION.md`, `CLAUDE.md`'s two overriding rules, and the commitment-model specs
(`2026-09-23-commitment-model-design.md`, `2026-09-24-commitment-model-phase2-design.md`), whose
phases 1–2 are on main. If this spec disagrees with the cloud design, the cloud design wins.

**Read with:**
- `docs/plans/2026-09-23-engine-follow-ups-plan.md` (F1–F3: the `unsure` card this spec extends).
- `docs/notes/2026-09-23-know-whats-next-direction.md` §3–§5.
- `docs/notes/2026-09-29-vision-program.md` (parity row P4).
- `docs/notes/2026-09-22-cost-matrices.md` (`obligation → drop` costs 3).
- `docs/surface/anatomy.md` §3.9 and §3.11.

**Base.** Main at `97dc27b`. It already holds #12 (the `unsure` word, the `event-check` card,
series inheritance, label telemetry) and the commitment model's phases 1–2 (#14, #16). The
`j-events` branch (`fcad6d4`, unmerged) is cloud-side only: the `event-4` decomposition and the B1
harness under `scripts/experiments/e1-decomposition/`. This build does not depend on it (§10).

---

## 0. The student-facing goal

A student asks Knowlu **"what's next?"** A required campus event (an advising session for their
major, a mandatory orientation, a lab safety briefing) is part of the answer. Today Knowlu judges it
`obligation` and lists it under Coming up, and that is all. Nothing asks the student about it,
nothing puts it on the day it happens, and nothing reserves the time. An optional event (a talk, a
career fair) is worse off in the app: it lands in a daily digest the app never shows as a card, and
the digest's expiry quietly declines it (§1, F1).

After this spec:
- Every judged event the student should decide on becomes **one card**: Accept or Decline.
- A required event the student accepts goes **into the plan**. It is on Today's schedule on its day,
  it takes its hours out of capacity, and Knowlu never proposes work over it. An all-day or
  multi-day event has no hours to take, so it goes in the day's all-day lane instead. That fallback
  is Q1b, decided (i) at signing, so the promise holds for every shape (§4.2).
- A declined event leaves Coming up and is never asked about again, across its whole series.
- An unanswered card is not a decline. It expires and says nothing, and the event stays in Coming up.

This answers VISION's test on two counts. The day's answer is **more complete**, because a required
event is in it. It is **more honest**, because capacity no longer counts hours the student will
spend at a fair.

## 1. What is on main today (verified 2026-09-29)

W1's research (`research-events.md`) was checked against main at `97dc27b`. Its file and line
claims were written against older bases. The corrected facts:

| Claim | On main |
|---|---|
| The event verdict words | `obligation`, `opportunity`, `drop`, `unsure` (`eventledger.rs:85`); the answer words are `obligation`, `drop` (`:90`). #12 is **merged**, not pending. |
| Judging | `events::judge_roster` (`events.rs:393`) → `CloudModel::judge_event` (`cloudmodel.rs:513`); a refused reply is recorded `unsure`. The model runs only in `judge`. |
| What an obligation does | Nothing but Coming up: `eventroster::relevant_events` (`eventroster.rs:59`, words at `:38`). The digest selects `opportunity` only (`eventemit.rs:129`), so an obligation is never proposed. |
| The events pass in `rank` | `cli.rs:439-522`: series inheritance (`:466`), the digest (`:485`), the `unsure` cards (`:497`), Coming up (`:508`). |
| The `unsure` card | `eventemit::emit_event_checks` (`eventemit.rs:700`), at most 3 a day (`:356`), grouped by series (20 listed), never asked twice. Settled by `approvals::settle_event_check` (`approvals.rs:1683`). |
| Label telemetry | `enrich::labels_to_report` (`enrich.rs:1180`): an `event-check` answer is a `verdict` row; any other judged card rejected is a `decision` row (`:1199-1206`). Never an email card. |
| Commitments | `commitments/` notes with a kind and a level. `event` is a reserved kind, soft by default (`commitments.rs:66-67`). `create_confirmed` (`:3871`) writes one. `detect_changes` watches only series keys (`:3493`), so a note keyed by an event uid is never re-proposed. |

**Four findings the research did not have:**

- **F1. The digest cannot be acted on in the app, and its expiry declines everything in it.** The
  read model never makes a digest a card (`surface.rs:1323-1326`). The deck shows only
  "N events in today's digest" (`console.js:287-290`). The Obsidian checkboxes it was built for are
  gone. The digest expires the day after its earliest event (`eventemit.rs:268`, `:320`). On expiry,
  `expand_digest(approve = false)` writes a `declined` line for **every** uid in it
  (`approvals.rs:1350-1353`, `:1143-1145`). Those events then leave Coming up, including the ones
  weeks away. The digest also spends up to its full weight of the 15-a-day budget
  (`approvals.rs:855`) before the `unsure` cards are sized (`cli.rs:503`).
- **F2. An approved digest has no executor.** It writes `kind: calendar-event` notes with
  `status: approved` (`approvals.rs:1015`). Knowlu only counts them as `awaiting_calendar`
  (`:1407-1408`). The executor was quinn-ops's cloud routine, which did not come across.
- **F3. The digest is in a frozen reference.** `golden-today-full.md:5` reads
  `**Events: 3 in today's digest**`, rendered by `rank` over `vault-full`, whose ledger holds three
  `opportunity` verdicts and no `obligation`. Rule 2 means the default digest path must stay
  byte-identical for a vault like that one (§3, D6).
- **F4. The event ledger is device-local.** `state/events-seen.md` is under `state/` and outside
  `ids::NOTE_FOLDERS` (`sync.rs:347-367`), so it never syncs. Settled cards in `archive/` do sync.
  W1b showed that an `event-check` answer survives on another desktop only because every card
  carries `series_uid`. This spec makes the archived card the durable record of every answer (D9).

The parity shape of quinn-ops is still visible in the code. `Task::conflicts_with`
(`models.rs:176-179`) was "written by the routine onto an obligation whose event collides with a
class". The routine made a task from each obligation. That routine is the piece that did not come
across.

## 2. Stage and scope

**Stage: MVP** (ruling 10). It is proven on the founder's scratch profile and a staging test
account, with no second person.

**In scope:**
- one Accept/Decline card per judged `obligation` and `opportunity` event (series grouped);
- Accept puts the event into the plan (§4; Q1 decides the shape);
- Decline removes the event from Coming up for good, across its series;
- the `unsure` card's Approve does the same as Accept, so a student is never asked twice about one
  event;
- the digest retired for vaults that opt in, which every new vault does (D6, Q3);
- the deck's buttons read **Accept** and **Decline** on event cards.

**Out of scope, each with its home:**
- the audience and level filter (direction note, stage 2): its own spec after the MVP;
- learned relevance (stage 3): Beyond, with the learning phase;
- event feeds for campuses other than Alabama, and onboarding that infers a feed: the Pilot's
  onboarding work (research Q5, trimmed: it changes no line of this build);
- write-back to Google Calendar: VISION's dedicated calendar, Beyond;
- events found in email: the Gmail spec;
- a conflict line on the card ("overlaps CS 100"): cut from the MVP (M2 spec Q7 (b), Appendix A (b),
  signed 2026-09-29); the dropped-event audit list: M2 (Q7 (d)); the 14-day expiry of opportunity
  proposals (P6 (c)) is not M2's: this spec's D3 delivers it;
- two-desktop behaviour: Launch (§9).

## 3. Decisions

The first four are §12's questions, shown here with their recommended answers, which Quinn accepted
at signing on 2026-09-29. D5–D12 are the planner's, each with its reason
and what it costs if wrong.

| # | Decision | Reason | Cost if wrong |
|---|---|---|---|
| D1 (Q1) | **Accept writes a one-off commitment**, `kind: event`: `hard` for an obligation, `soft` for an opportunity. It also writes a **"Register" task** when the event requires registration, due at the registration deadline, or undated when the feed gives none (Q1a). An all-day or multi-day event writes no commitment and is drawn in the all-day lane instead. A past-midnight event writes a commitment that ends at 23:59 (Q1b, §4.2). | An event happens at a time; it is not work to do before a due date. As a task with effort, a 3-hour fair would take 3 hours of capacity on the days *before* it. As a commitment, it takes the hours on its day, shows on the schedule and blocks proposals (VISION commitment 4). | Ruling 10 says "tasks". If Quinn means a task, §4.2's alternative applies instead, and capacity counts the event's hours before it. |
| D2 (Q2) | **Obligations are asked, never auto-created.** | The feed is not an authority on what this student must attend; the model inferred it. VISION commitment 5: "Knowlu noticed → it proposes and waits". | A student who ignores the card has no commitment. The event still shows in Coming up, because expiry is not a decline (D8). |
| D3 (Q3) | **One card per opportunity replaces the digest.** Unanswered, it expires after 14 days or at the event, whichever is first. | The digest cannot be acted on in the app, and it silently declines (F1). | More cards. They are capped (D7). |
| D4 (Q4) | **An answer covers the whole series**: every instance the card lists, and later instances as they appear. The carry never writes a uid that any note has ever carried, deleted notes included, and every carried note is listed in the delta (§4.5). | A declined recurring opportunity stays declined across the series (VISION, Learning). A weekly required meeting accepted once should not be asked about every week. The student answered, so Knowlu acts, lists what it did, and a delete undoes it for good (VISION commitment 5). | A student who wants only some instances deletes the rest one at a time from the schedule, and they stay deleted. A deleted instance comes back only if the student re-creates it by hand. There is no per-instance undo card in the MVP. |
| D5 | **A new card kind, `event-accept`**, a sibling of `event-check`, filed by the same code path in `rank`. | It extends #12's card. `event-check` keeps its meaning ("does this apply to you?") and its label rows. | One more kind in `approvals.rs`'s dispatch. |
| D6 | **The opportunity switch is `event_cards: true` in `config/events.yaml`.** Without it, `rank` files the digest as it does today. The campus presets (`app/assets/campus/*.yaml`) carry the key, so every vault the wizard creates has it. Obligation cards need no switch. | `vault-full` has no such key and no obligation verdict, so `golden-today-full.md` stays byte-identical (F3, rule 2). The loader ignores unknown keys (`events.rs:239`), so an older engine reads the file unchanged. | A vault created before this change keeps the digest until its `events.yaml` gains one line. The only such vaults are the founder's. |
| D7 | **Caps**: at most 3 obligation cards and 3 opportunity cards first proposed per day. This is beside `event-check`'s 3, and every card is charged to the 15-a-day budget. Order: obligations, then `unsure` checks, then opportunities, each soonest-first. Opportunities are ordered by strength, then registration deadline, then start (the digest's `sort_key`, `eventemit.rs:150`). | A required event matters most (cost 3 when missed). 47 of 50 labelled campus events were opportunities, so a flood is the real risk. | A busy week holds opportunity cards back a day or two. They stay eligible until the event's horizon passes. |
| D8 | **Expiry writes nothing.** An expired `event-accept` card archives as `expired`. Its instances are never asked about again, but its series' next instance may be (the `event-check` rule, ruling G1). The event stays in Coming up. | Silence is not consent and not refusal. The digest's decline-on-expiry is F1's defect. | An ignored required event never enters the plan. It is still listed. |
| D9 | **The archived card is the record of an answer.** At each `rank`, a `rejected` `event-accept` card in `archive/` whose uids have no `declined` line in this device's ledger gets those lines. An `executed` card needs nothing: its notes sync as notes, and writing them again from the card would race sync into a duplicate. `event-check` already derives its answers from `archive/`. | The ledger never syncs; archived cards do (F4). A restore or a second desktop reads the same answers. | One extra read of `archive/` per `rank`, which the `event-check` emitter already makes. |
| D10 | **The card carries its own payload** (`instances:`, §5.1), so Accept settles inside `decide`, and the schedule shows the event before the next slot runs. | `process_approvals` runs in the app with no feed at hand (`commands.rs:281-292`). | The card is larger. It is vault text that already syncs. |
| D11 | **Accepting an `unsure` card is an Accept.** When the card carries `instances:`, its Approve writes the human answer (as today) and then D1's notes. Cards filed before this change carry none and keep today's behaviour. | A student who said "this applies to me" should not then be asked "will you go?". | None found. |
| D12 | **No new telemetry shape.** A declined `event-accept` card is already a `decision` row (`proposed → rejected`) because it carries `judgment_kind: event` (`enrich.rs:1203`). An accepted card is right by absence, as today. | It keeps the privacy page true as it stands (§7). | An opportunity declined for taste is reported as a rejected decision. That is what it is. |

## 4. What each answer does

### 4.1 The card

`rank` files one `event-accept` card for an event, or a series, when all of these hold:
- its ledger verdict is `obligation`, or `opportunity` under D6's switch;
- nobody has answered it, and it is neither declined nor proposed;
- it is inside its horizon. That is `eventemit::horizon_start` (`eventemit.rs:59`): the propose
  horizon, or three days before a registration deadline, whichever is earlier;
- no approval names its uid, as `source_uid` or in `events:`, in `approvals/` or `archive/`, of
  either event kind;
- its series has no live event card and no answered one. "Answered" is the union of
  `eventemit::settled_series` (answered `event-check` cards) and `eventcarry::answered_series`
  (answered `event-accept` cards, §6.1). This is the same union `judge_roster` skips. The emitter
  calls `answered_series` and never re-derives it, so one `rank` cannot both carry a series and
  ask about it.

These are `emit_event_checks`' tests (`eventemit.rs:566`, `:700`), widened to both kinds. A card
lists up to 20 instances of one series; the soonest is its primary.

**The title** is `what_and_when` (`eventemit.rs:436`), prefixed by the kind:
`Required · Career fair · Thu 1 Oct 10am–3pm`, or `Worth a look · Engineering talk · Tue 6 Oct 5–6pm`.
A series card adds ` · +N more`.

**The first paragraph is the card's `why`**, which the deck shows (`surface.rs:1335`):
`**Knowlu thinks this is required of you.** <the judgment's why>`, or
`**This may interest you.** <why>`. After it come the facts line (when, where, who), the URL, the
series' other dates, and one closing sentence saying what each answer does. No Obsidian buttons.

### 4.2 Accept

For each instance on the card that no note yet names by `source_uid`, the settlement writes:

1. **A commitment** (D1) in `commitments/`, through `commitments::create_confirmed` (`write::create`,
   journal first, id `cmt_…`):
   - `kind: event`;
   - `level: hard` (obligation) or `soft` (opportunity);
   - `title`: the event's, at most 200 characters;
   - `meets: [{days: [<weekday>], start, end}]`, wall-clock in the vault's timezone;
   - `from` and `until`: both the event's date;
   - `where`: the location, when short enough (≤ 80 characters);
   - `source_uid`: the event uid;
   - `status: confirmed`.

   It is written when the event starts and ends on one day with `start < end`. The other shapes
   follow Q1b's recommended answer:
   - **Past midnight** (it ends on the next day, before its start time): a commitment on the start
     day that ends at 23:59. Its body line gives the true end ("Ends at 1am the next day."). The part
     after midnight is outside any planning window that ends by midnight.
   - **All-day or multi-day**: no commitment, because the event has no hours to take. The answer
     stands. `surface::the_day` draws the event in the all-day lane on each of its days, and that
     lane never subtracts capacity (anatomy §3.7). It reads the event from the archived `executed`
     card's `instances:`, and `surface` still never writes. Coming up marks it "Accepted".
2. **A "Register" task**, only when the feed marks the event as requiring registration. It goes in
   `tasks/` through `write::create`:
   - `title: "Register: <title>"`;
   - `due`: the registration deadline at 23:59. When the feed gives no deadline, `due: null`, the
     value `create_task` writes for an undated task (`commands.rs:247`). Knowlu never invents a due
     date (VISION commitment 5, ruling 3). The event's start is an upper bound, not the deadline:
     registration can close or fill days before. The task's body says "Registration closes by
     <start> at the latest; check the event page for the real deadline." with the URL. Until the
     "needs a date" marker exists, the read model shows it under "No due date" (`surface.rs:679`).
     There it sorts last (`scheduling.rs:102`), which is Q1a's cost;
   - `effort_hours: 0.25`;
   - `importance: 3`;
   - `domain: school`, the value `create_task` writes (`commands.rs:246`);
   - `status: active`;
   - `created_by: events`;
   - `source_uid: "register:<event uid>"`.

   A series writes one task, for its primary instance only.

Then the card is stamped `executed` and archived, exactly as `event-check`'s arm does
(`approvals.rs:1460-1470`). A failed write leaves the card `approved`, and the next pass retries.
The settlement skips any instance that a note has ever carried (§5.3's set, which includes
`archive/`). So a retry never duplicates a note and never brings back one the student deleted.

**Actor.** `create_confirmed` writes as `agent:commitments` (`commitments.rs:3879`). The task is
written under the context `process_approvals` runs with: `agent:approvals` through
`executor_ctx()` in the app (`commands.rs:171`), or the `rank` run's context in a slot. The
student's own record is the `status: approved` set on the card, made by the console's human actor.
So judge-once holds: a later edit the student makes to either note is a human set, and no agent
re-sets it.

**The note's body.** `create_confirmed` picks its one-line body by key prefix, and an event uid would
get "Found as a weekly series on your calendar." (`commitments.rs:3922-3928`). A `kind: event` note
gets its own line instead: "You accepted this from your campus events."

**The alternative, if Q1 is answered "a task".** Step 1 becomes a task: the event title,
`due` = the event's start, `effort_hours` = its length (clamped to 0.25–4), `source_uid` = the event
uid. The commitment is dropped. Everything else in this spec stands. The cost is the one D1 names.

### 4.3 Decline

One `declined` line per listed uid (`eventledger::record_declined`, `eventledger.rs:679`). The
events leave Coming up at the next read (`eventroster.rs:71`), and no event emitter asks about them
again. The card is archived `rejected`. D4 carries the decline to later instances of the series at
each `rank` (§4.5).

### 4.4 Snooze and expiry

Snooze is the deck's, unchanged. `expires`:
- an obligation card: its primary's start date;
- an opportunity card: the earlier of that date and `first_proposed_at + 14`.

On expiry, `process_approvals`' generic path archives the card as `expired`. D8: no ledger line, no
note.

### 4.5 The series carry (D4)

At each `rank`, before any event card is filed, the carry reads the answered `event-accept` series
through `eventcarry::answered_series`: `rejected` and `executed` cards in `archive/`, each by
`series_uid` (§6.1 gives its edge rules). It then acts on every
instance of those series in the roster that the card did not list:
- **Declined series:** one `declined` line per new instance (`record_declined`).
- **Accepted series:** §4.2's notes for each new instance whose start is today or later, **unless
  that uid is in the ever-written set** (§5.3). The set reads `commitments/`, `tasks/` and
  `archive/`. A note the student deletes goes to `archive/` with its frontmatter intact
  (`write::delete`, `write.rs:512-537`), so a deleted instance stays deleted: deleting is the undo,
  and it sticks.

`inherit_series_answers` (`eventemit.rs:518`) is unchanged. It still carries `event-check`
answers, and the new carry lives beside it in its own module (§6.1).

**Listed, not silent.** The student answered for the series, so Knowlu acts (VISION commitment 5).
Every carried note is a journal `create` record in the `rank` run. The read model's delta
(anatomy §3.2) groups those records and expands to one record per note, so each carried commitment
is listed under what changed, with its title and run id. No new surface is added, and **no task
edits `surface::delta`**.

The delta's window decides whether the carry shows. The code answers this (`surface.rs:1699-1721`
on 97dc27b):
- **With a `seen_at`**, the carry is listed. The console passes its persisted stamp
  (`ConsoleState::seen_at`, `app/src/state.rs:145`), and `delta` keeps every record later than it.
  That includes the carrying run's own `create`s.
- **With no `seen_at`**, the carry is not listed. `delta` falls back to the newest run's `end` and
  keeps only records later than it. When the carrying run is the newest run, its own records are
  earlier than its `end` and are dropped.

The console writes the stamp at the end of each look, on blur or hidden (`mark_seen`, `console.js`
`endOfLook`). A carry needs an Accept made earlier in the console, so the stamp normally exists by
the time a carry runs. The gap is narrow: the window never lost focus or hid between that Accept and
the carrying run, or the stamp file is gone (a rebuilt profile). Q4's cost names it, and this spec
accepts it rather than change `delta`.

**A known limit: the display cap.** `delta` counts every record in its summary but lists only the
newest `DELTA_RECORD_CAP` (200, `surface.rs:29`) and sets `truncated`. A carry that falls in a
window of more than 200 records is counted but not every record is expanded.

## 5. Data model

Everything here is additive. No existing field, ledger word, VIA or run-record key is renamed or
reshaped.

### 5.1 The `event-accept` card

`approvals/event-<slug≤40>-<primary date>.md`, with `-2`, `-3` on a collision. It is created whole
through `write::create`; its frontmatter goes through `yamlemit::safe_dump_block`, as `write_check`
does (`eventemit.rs:620-686`). Fields, not exact bytes:

```yaml
---
type: approval
kind: event-accept
title: "Required · Career fair · Thu 1 Oct 10am–3pm"
status: pending
verdict: obligation            # or opportunity
source_uid: "localist:77:1"    # the primary instance
series_uid: "localist:77"
events: ["localist:77:1"]      # every listed uid, as event-check writes them
instances:                     # one mapping per listed uid, for the settlement (D10)
  - {uid: "localist:77:1", title: "Career fair", start: 2026-10-01T10:00, end: 2026-10-01T15:00,
     location: "Ferguson Center", registration: false, registration_deadline: null}
judgment_id: "…"               # when the ledger holds one
judgment_kind: event
proposed_at: 2026-09-24
first_proposed_at: 2026-09-24
expires: 2026-10-01
snooze_until: null
created_by: events
---
```

`events:` keeps the shape `card_event_uids` already reads (`eventemit.rs:468`), so every
never-ask-twice reader covers the new kind unchanged. `instances:` is new and read only by the
settlement. Its `start` and `end` are formatted from real datetimes. The digest's payload has the
same property, and it is why preserved defect 15 cannot reach it (`eventemit.rs:18-22`).
`title`, `location` and the other free-text fields are clipped with `judge::one_line`.

### 5.2 The `event-check` card, extended

New `event-check` cards gain the same `instances:` block (D11). Their closing sentence changes to
"Approve if it applies to you: it goes on your schedule for that day. Reject and it's dropped.
Either way you won't be asked again." Cards without `instances:` settle exactly as today.

### 5.3 Notes written on Accept

A commitment in `commitments/` (§4.2 step 1, the parent spec's §2.1–§2.2 fields) and, when the
event requires registration, a task in `tasks/` (§4.2 step 2). Both are ordinary notes. They sync,
they are edited through `set_fields`, and they are deleted through `delete_note`, which moves them
to `archive/`. **The ever-written set:** before writing, the settlement and the carry (§4.5) both
read `source_uid` from every note in `commitments/`, `tasks/` and `archive/`, and skip any uid (or
`register:<uid>`) found. Notes with `type: approval` are left out: an archived event card carries
its primary's uid as `source_uid`, and counting it would stop that instance from ever being
written. This set is the record of an Accept that outlives the note: a live note
means "already written", and an archived one means "written, then deleted by the student". Both
mean "never write again". `approvals::existing_source_uids` reads only approval cards
(`approvals.rs:1080`), so it is not enough on its own here. Archived notes sync (F4), so a restore
reads the same set.

### 5.4 The ledger

No new line shape and no new verdict word. Decline writes `record_declined` lines. Accept writes
nothing to the ledger for `event-accept`: the notes and the archived card are the record, and a
confident `obligation` or `opportunity` verdict already puts the event in Coming up. An
`event-check` Approve still writes its `record_answer` line (F1–F3).

### 5.5 The switch

`config/events.yaml` gains `event_cards: true` in both campus presets, including `none.yaml`, so a
vault that adds sources later gets cards too. The loader reads it as a boolean: `true` or `1` means
on, and anything else, or no key, means off with no warning. `EventsConfig` gains
`event_cards: bool`. The engine never writes `events.yaml`.

## 6. The engine surface and the commands

### 6.1 Engine

| Where | Change | On the contract list? |
|---|---|---|
| `engine/src/eventemit.rs` | `emit_event_accepts` beside `emit_event_checks`. One shared selection (§4.1) over a verdict-to-card table: `unsure` → `event-check`, `obligation` → `event-accept`, `opportunity` → `event-accept` (switched). Shared caps, series grouping and never-ask-twice. Never-ask-twice's series test is the union of `settled_series` and `eventcarry::answered_series`, the same union `judge_roster` uses. The emitter calls `answered_series` and has no predicate of its own for answered `event-accept` cards. `instances:` on both kinds. `inherit_series_answers` and `settled_series` are **unchanged**. | No |
| `engine/src/eventcarry.rs` (new) | The vault-writing event state outside `approvals.rs`: §5.3's ever-written set, §4.5's decline and accept carry, D9's rebuild of `declined` lines from `archive/`, and `answered_series` (the `event-accept` series settled in `archive/`). There is **one** `answered_series`, and the emitter, the carry and `judge_roster` all call it. Its edge rules follow `settled_series`: it reads `archive/` only; `executed` answers accept and `rejected` answers decline; `expired` or any other status answers nothing (ruling G1); a card with a missing or empty `series_uid` answers no series, and the uid rule (§4.1) closes that card's own instances; if two cards answer one series, the lowest file name wins. `judge_roster` (`events.rs:447`) skips the union of `settled_series` and `answered_series`, so it stops paying for a settled series' new instances. That one call-site edit in `events.rs` is made here, after T1. | No, but contract-engineer's (§13): a silent error here declines or hard-books a whole series. |
| `engine/src/eventaccept.rs` (new) | Pure builders with no I/O. From one instance, it builds the commitment mapping §4.2 hands to `create_confirmed`, or the all-day marker for Q1b's lane. It builds the register task's frontmatter, with `due: null` when there is no deadline. | No |
| `engine/src/surface.rs` | Q1b: `the_day`'s all-day lane gains accepted all-day and multi-day events, read from `executed` `event-accept` and `event-check` cards' `instances:`. Coming up marks an accepted event "Accepted". Read-only. | No |
| `engine/src/approvals.rs` | The `event-accept` arms: approved → the accept settlement, then stamp and archive; rejected → the declined lines, then archive. The `event-check` approved arm runs the accept settlement after `settle_event_check` when `instances:` is present. `proposal_weight` stays 1 for both kinds. | **Yes** |
| `engine/src/commitments.rs` | `create_confirmed_as`' body line for `kind: event` (§4.2), with the true end for a past-midnight event. `p3-registrar` edits the same branch (§9). | No |
| `engine/src/events.rs` | `event_cards` in `EventsConfig` and `load_events_config`. | No |
| `engine/src/cli.rs` | The events pass, in this order: series inheritance, then `eventcarry`'s D9 rebuild and §4.5 carry; obligation cards; the digest **only when `event_cards` is off**; the `unsure` checks; opportunity cards when it is on. Each is sized to the budget the ones before it left. New cards are added to `approvals.pending`. | No (a shared file, applied by whoever owns the merge) |

**Invariants this keeps:**
- **`rank` never calls a model.** Every step above reads the ledger and the roster, and writes
  through `write`.
- **The same input gives the same bytes.** Cards are filed in `(primary start, primary uid)` order,
  and instances are listed in `(start, uid)` order.
- **All JSON goes through `ledger::dumps_value`.** No new JSON is written: the cards and notes are
  YAML through `yamlemit`.
- **An older engine is tolerated.** An engine without this change warns `unknown kind:` on an
  approved `event-accept` card on every run, and never settles it. The vault is single-desktop and
  single-engine in the MVP, so that cannot happen until Launch's two-desktop work (§9). An older
  engine reads `event_cards:` as an unknown key and ignores it.

### 6.2 Engine commands

- **No new command and no new flag.** `rank` gains the pass above.
- `judge`, `sync`, `coursework` and `ingest` are unchanged. `surface` is unchanged: event cards are
  pending approvals, so they are ordinary deck cards (`surface.rs:1326`).
- `docs/reference/engine-commands.md`'s `rank` entry gains one paragraph.

### 6.3 App commands

- **No new Tauri command.** Accept and Decline are the existing `decide` (`commands.rs:443`),
  registered in the console window's `generate_handler!` list in `app/src/main.rs:177`. Its
  in-process `process_approvals` (`commands.rs:291`) runs the settlement, so the schedule shows the
  event on the same click.
- **Neither `generate_handler!` list changes**, so the command count is unchanged.
- `app/src/commands.rs` computes nothing new.
- **`app/static/console.js`**: on `event-accept` and `event-check` cards, the primary button reads
  **Accept** and the secondary **Decline**. The verdicts sent stay `approved` and `rejected`. This is
  the same per-kind switch `commitment-ask` already uses (`console.js:300`).
- The app writes nothing else. The preset change (§5.5) is an asset edit in `app/assets/campus/`.

## 7. Privacy

**No new data class leaves the device, so `PRIVACY_VERSION` does not move.** The rule at
`app/src/account.rs:21-23` is that the page's date and the constant move together in one commit.
This spec changes neither.

- **Cards and notes** are vault text. They are held and synced as the page's "Your tasks and notes"
  already says (`site/privacy.html:36`). What they hold comes from public campus feeds: title, time,
  place, organiser, link. The event's description is not copied into a card or a note.
- **The ledger** stays on the device, as it does today (F4).
- **Telemetry** gains no row shape (D12). A declined `event-accept` card is the existing `decision`
  row: ids and words, no title. The page's "Interaction events" and "Your corrections" entries
  (`:42-44`) already cover it.
- **Model calls** are unchanged. `judge-event` sends what it sends today, and `rank` sends nothing.

**A wording gap that predates this spec.** The page says CoreWeave "reads … the events on your
calendar" (`:66`). Campus-feed events go to `judge-event` too, and have since C2. This spec does not
widen that, but the sentence should name campus events. **Recommendation:** add it to privacy bump
#1's text, which already has a lawyer read scheduled (ruling 12), rather than make a bump of its own.

## 8. Contract-list impact

| Contract | Impact |
|---|---|
| `engine/src/approvals.rs` | **Changed**: two new arms and one extended arm (§6.1). This is contract-engineer work, reviewed by contract-reviewer. The existing digest, `calendar-event` and `event-check` arms stay byte-for-byte in behaviour; the digest arms stay forever, so a digest filed before the switch still settles. |
| `write.rs`, `journal.rs` (`VIAS`), `yamlemit.rs`, `yaml.rs`, `pystr.rs`, `ledger.rs`, `ids.rs`, `provenance.rs` | **Used, not changed.** Every note goes through `write::create`. There is no new actor: the writers are `agent:approvals`, `agent:commitments` and `agent:events`, which all start `agent:`. No new id kind is needed: `cmt_` and `task_` exist. |
| `sync.rs` | **Not changed.** `event-accept` is not a local card kind. It syncs like `event-check`, as its facts are public. The commitment and the task sync as notes. |
| `entitle.rs`, `wincred.rs`, `reconcile.rs`, `app/src/{credentials,account,updates}.rs` | Not touched. |
| `engine/tests/fixtures/**` | **Not touched.** Every new test builds its vault in a temporary directory, as `eventemit.rs`'s tests do. |
| The oracle, sync and entitlement tests | **Not changed. They must stay green unchanged:** `oracle.rs` (both goldens; F3 is why D6 exists), `surface_oracle.rs`, and every sync test. |
| Ledger contract (`state/events-seen.md`) | No new line shape, and `VALID_VERDICTS` is unchanged. |
| Note frontmatter | Two new approval fields (`verdict`, `instances`) on a new kind, plus `instances` on new `event-check` cards. These are additive, and no existing note is rewritten. |

**Frozen references:** none is regenerated, and none may be. `golden-today-full.md` stays true
because `vault-full` has no `event_cards` key and no `obligation` verdict. The three
`surface-today-*.json` references are read-only renders of fixture vaults that hold no event card.

## 9. Two desktops, and other lanes

- **Launch's two-desktop spec** (`2026-09-25-two-desktop-design.md`, draft PR #20, frozen) lists the
  digest and `calendar_note` among its producers (`:103-104`), and gives non-holders
  `rank --no-digest` (D12). When that stream resumes, it must:
  - add `emit_event_accepts` and `emit_event_checks` to its producer table;
  - make `--no-digest` cover every event card;
  - give accepted-series notes (D4) deterministic ids.

  **Recommendation:** record this as a hand-off in that spec at its next revision. It changes
  nothing in the MVP.
- **Ruling 11** (the `student` human token) changes `journal::human_set`, which `settle_event_check`
  uses for `by:`. This spec adds no `"quinn"` literal and reads the human actor only through
  `journal` helpers. Test 28 asserts that an actor is human with `!provenance::is_agent(actor)`,
  never by the token. So the behaviour holds under either merge order. The files do not (below).
- **The commitment model's phase 3** (`p3-registrar`, Pilot) does not touch `kind: event` notes:
  `detect_changes` watches series keys only (`commitments.rs:3493`).

**File collisions with lanes in flight.** Taken from `git diff 97dc27b...<branch>` in the
2026-09-29 review, and spot-checked. p3's copy of the body-line branch is at `commitments.rs:4042`
on `p3-registrar`. The integrator re-runs `git merge-tree --write-tree` against each branch before
T1 starts, and again before the branch merges.

| Branch (stage) | Shared files | Events task | Merge order and resolution |
|---|---|---|---|
| `human-actor` (MVP, ruling 11) | `app/tests/commands.rs`, `app/tests/scaffold.rs`; the human actor `decide` writes | T7, T6 | **Before events.** If it has merged by the time the branch is cut, there is nothing to resolve. If not, the events branch merges main after it, and T6 and T7 rebase their test additions onto its token changes. Test 28 is token-free either way. |
| `m1-grades` (MVP) | `engine/src/commitments.rs`: the `NOTE_FOLDERS` tripwire test (`the_servers_note_path_rules_name_every_note_folder`) | T1; test 25 leans on `NOTE_FOLDERS` | **Before events.** Test 25 asserts only that `commitments/` and `tasks/` are note folders, so a folder that grades adds cannot break it. T1's hunk is in `create_confirmed_as`, away from the tripwire, so a textual conflict is unlikely. |
| `p3-registrar` (Pilot) | `engine/src/commitments.rs` (the `create_confirmed_as` body-line branch), `engine/src/cli.rs`, `app/static/console.js`, `app/tests/static_assets.rs`, `site/privacy.html` | T1, T4, T5 | **After events: p3 rebases in the Pilot.** The body-line branch keeps both arms: p3's `registrar::CALENDAR_PREFIX` key arm, and events' `kind: event` arm, tested first because it is by kind, not by key. `cli.rs` keeps p3's registrar pass and events' pass as separate blocks. `console.js` keeps both per-kind label switches. Events does not edit `site/privacy.html` (§7). |
| `two-desktop` (Launch, frozen) | its spec's producer table | none | Hand-off only (above). |

## 10. The judgment side: j-events and B1

This build needs **no cloud change**. The device reads the same four verdict words whichever prompt
the service pins.

B1 is the gate on `j-events`' `event-4` prompt, measured by its harness in
`scripts/experiments/e1-decomposition/`. It asks whether splitting the two drop rules
("audience excludes the student", "standing or drop-in") into scored fields beats `event-3`. No
result is recorded yet. The harness informs this spec in two ways:
- It records a refused reply as `unsure` (`arms.ts`, `deviceRecorded`), exactly as the device does,
  so `event-4`'s refusal rate becomes the rate of `unsure` cards.
- Its audience rule is what would keep faculty-only events from becoming opportunity cards. That is
  why D7 caps opportunities, and why D6 lets opportunity cards be switched off per vault.

**Recommendation** (Q5): do not make this build wait for B1. Run B1 in a cloud lane before the
founder's MVP proof, and ship `event-4` only if it wins, as its migration header says. If it does
not win, revert the branch as the header plans. Q5 asks the one part of this that changes the
build: whether the Alabama preset switches opportunity cards on before B1 has a result.

## 11. Test plan, test first

Every behaviour below gets its failing test before its code. Vaults are built in temporary
directories, as `eventemit.rs` and `approvals.rs` tests already do. No test touches the network
(ledgers are seeded, not judged), Credential Manager or `engine/tests/fixtures/**`.

### 11.1 The emitter (`eventemit.rs`, `eventaccept.rs`)

1. An `obligation` inside the horizon files one `event-accept` card with `verdict: obligation`, the
   `Required · ` title, the why paragraph first, `instances:` and `events:`, and the ledger's
   `judgment_id`. A `proposed` line follows the card, not the other way round.
2. An `opportunity` files a card only when `event_cards` is on. With it off, the digest is filed
   exactly as today: the existing digest tests pass unchanged.
3. Caps: four obligations in one day file three. Three opportunity cards already first proposed
   today file none. `event-check`'s 3 is independent. The budget argument bounds all three.
4. Order: obligations, then checks, then opportunities, each in `(start, uid)` order. Opportunities
   use the digest's `sort_key`. Two runs over the same input write the same bytes.
5. Never ask twice: a uid named on any event card of either kind, in `approvals/` or `archive/`, is
   skipped. So is a uid with a `proposed` or `declined` line, or one whose series has a live card or
   an answered one. An expired card closes only its own instances (ruling G1). The answered-series
   case seeds an `executed` and a `rejected` `event-accept` card in `archive/` and asserts a skip.
   It goes through `eventcarry::answered_series` and does not restate the predicate. The edge cases
   are T2b's unit tests, not T2a's.
6. A series files one card listing at most 20 instances, the soonest as primary.
7. Expiry: an obligation card expires at its primary's date; an opportunity card at the earlier of
   that date and `first_proposed_at + 14`.
8. The builders: a one-day timed event gives the §4.2 commitment mapping. A past-midnight event
   gives a mapping that ends at 23:59 on its start day, with its true end for the body line. An
   all-day or multi-day event gives the all-day marker and no commitment. `registration: true` with
   a deadline gives the task due at the deadline at 23:59. `registration: true` with no deadline
   gives `due: null`, never the event's start, and a body naming the start as an upper bound.

### 11.2 The settlement (`approvals.rs`, contract-engineer)

9. Approve an obligation card: one `commitments/` note (`kind: event`, `level: hard`, the one-day
   `meets`, `from` = `until`, `source_uid` = the uid, `status: confirmed`). The card ends `executed`
   in `archive/`. The journal's `create` record comes before the file.
10. Approve an opportunity card: the same, with `level: soft`.
11. Approve with `registration: true`: the register task, with `source_uid: "register:<uid>"`.
    With a deadline, `due` is the deadline at 23:59. With `registration_deadline: null`, `due` is
    `null`, the task is in the read model's undated list, and no field holds the event's start.
12. Idempotence: a card left `approved` by a failed stamp, settled again, writes no second note.
    A note that already carries the uid in `commitments/`, `tasks/` or `archive/` is skipped. An
    archived approval card that carries the uid as its `source_uid` does not count, so the primary
    is still written.
13. Reject: one `declined` line per listed uid; `relevant_events` then excludes them; the card is
    `rejected` in `archive/`.
14. Expiry writes no ledger line and no note (D8). Contrast the digest's expiry test, which still
    declines.
15. `event-check` with `instances:`, approved: the `record_answer` line as today, then the
    commitment. Without `instances:`: byte-identical to today's settlement.
16. An all-day instance: the answer stands, no commitment is written, and nothing warns. A
    past-midnight instance writes the 23:59 commitment, and its body gives the true end.
17. The `calendar-event` and digest arms: their existing tests pass unchanged.

### 11.3 `rank` and the read model (`cli.rs`, `surface.rs`)

18. A full `rank` over a vault with an obligation, an opportunity, an `unsure` event and a series:
    the right cards, the pending count raised by their number, the budget respected, and
    `defer_over_budget` snoozing only same-day overflow.
19. D9: a `rejected` `event-accept` card in `archive/` and an empty ledger. `rank` writes the
    `declined` lines, and the event is not in Coming up.
20. D4: a later instance of a declined series gets a `declined` line. A later instance of an accepted
    series gets its commitment, once: a second `rank` writes nothing.
20a. **Delete sticks:** accept a series, run `rank` so that a later instance is carried, delete that
    commitment through `write::delete` (it moves to `archive/`), then run `rank` twice. Neither run
    writes a note or a journal record for that uid. The same holds for a deleted register task.
20b. **Listed:** after a `rank` that carries an instance, `surface`'s delta, called with a `seen_at`
    earlier than the carrying run's start, counts the `create`. It expands to a record naming the
    carried commitment and the run id.
20c. **The fallback is pinned:** the same vault with no `seen_at`, where the carrying run is the
    newest run. The delta's `since` is that run's `end`, `since_kind` is `"run"`, and the delta holds
    none of the carry's `create` records. This pins today's `delta` (§4.5), so a failing 20b is never
    "fixed" by editing `delta`. If it fails, `delta` changed, and the change goes to Quinn.
21. `judge_roster` does not send a new instance of a settled `event-accept` series.
22. `surface` shows an event card as an ordinary deck card, with `why` from the first paragraph.
    After Accept, `the_day` for the event's date carries the commitment block.
22a. Q1b: after Accept of an all-day, two-day event, `the_day` for each of its days has it in the
    all-day lane, capacity is unchanged, and Coming up marks it "Accepted".
23. **The oracles:** `oracle.rs` (both goldens) and `surface_oracle.rs` pass with no reference
    regenerated.

### 11.4 Telemetry, sync and the app

24. `labels_to_report`: a rejected `event-accept` card with a `judgment_id` gives one `decision`
    row. An approved one gives none. An `event-check` gives its `verdict` row, as today.
25. Sync, asserted from `eventemit.rs`'s own tests so that no sync test is edited: `event-accept`
    is not in `commitments::LOCAL_CARD_KINDS`, and `ids::NOTE_FOLDERS` contains `commitments` and
    `tasks`. It checks membership only, never the list's length or order, so a folder another lane
    adds cannot break it (§9). Every existing sync test passes unchanged.
26. `app/tests/static_assets.rs`: `console.js` labels `event-accept` and `event-check` cards
    Accept and Decline, and still sends `approved` and `rejected`.
27. `app/tests/scaffold.rs`: a vault scaffolded from either preset has `event_cards: true`.
28. `app/tests/commands.rs`: `decide` approving an `event-accept` card returns it in `executed`, and
    the commitment exists before the call returns. The card's `status` record has an actor that
    `provenance::is_agent` rejects, and the commitment's `create` actor is one it accepts. The test
    names no human token (`quinn` or `student`), so it passes whether `human-actor` merges before
    events or after (§9).

**The live proof** (the main session, on the founder's scratch profile, per the autonomous-proof
practice): one real slot against the Alabama feeds; one obligation and one opportunity accepted;
one declined; the schedule and Coming up checked; one `rank` repeated to show that nothing is
re-asked. The scratch profile is removed afterwards.

## 12. Open questions for Quinn (all decided, 2026-09-29)

Quinn signed this spec on 2026-09-29 and accepted every recommendation. Each question below is kept
with its options and reasoning; its **DECIDED** line records the answer.

These are W1's five research questions, merged and trimmed to the ones that change the build. Two
were dropped:
- **Merge order (research Q4) is moot.** #12 is on main.
- **Campus feed scope (research Q5)** changes no line of this build. It moves to the Pilot's
  onboarding work (§2).

One question was added (Q4), and the 2026-09-29 review added two parts to Q1 (Q1a, Q1b). Ask them
one at a time, in this order. Q1a and Q1b apply only if Q1 is (b) or (c).

**Q1. What does Accept on a required event put in the vault?**
Ruling 10 says "required events become tasks". Since it was written, the commitment model's phases
1–2 reached main, and they give an event a truer home.
- (a) **A task.** This is quinn-ops parity, and the literal ruling. `due` is the event's start and
  the effort is its length. Capacity then counts the event's hours on the days *before* it, as work.
- (b) **A one-off commitment**, `kind: event`: hard if required, soft if optional. It is on Today's
  schedule, takes its hours on its own day, and Knowlu never proposes work over it (VISION
  commitment 4). There is no list item.
- (c) **(b), plus a "Register" task** when the feed says the event needs registration, due at the
  registration deadline.

**DECIDED (Quinn, 2026-09-29): (c), as recommended.** It is the honest answer to "what's next?":
the event is on the schedule, and the only list item is the one real piece of work. Signed, so ruling
10's parenthetical now says required events enter the plan as a one-off commitment, and the cloud
design carries the marker from the signing commit (§14).
It costs one small change in `commitments.rs` (§4.2) and nothing on the contract list beyond (a)'s.

**What (b) and (c) cost in behaviour.** A commitment needs a start and an end on one day. With
nothing more, an accepted all-day or multi-day event would be on no schedule and in no list. That
breaks §0's promise, and campus feeds often publish fairs and orientations that way. (a) has no
such gap: a task takes any shape, due at the event's start. **How many Alabama events have that
shape is not measured.** This revision was written offline, and no file on disk marks one. T0
(§13) counts them from the Alabama preset's feeds before Q1 is asked, so Quinn sees the number.
Q1b is the fallback, and it stands whatever the count is. (Quinn signed before T0's count existed,
on that basis; T0's number is still recorded when it is measured.)

**Q1a. When the feed says "registration required" but gives no deadline, how is the Register task
dated?**
- (i) **Undated** (`due: null`). The body names the event's start as an upper bound and links the
  page. It shows under "No due date", and sorts last until the "needs a date" marker exists.
- (ii) **No task.** The card and the commitment say "Registration required", with the link.
- (iii) Due at the event's start. This was the draft, and it is withdrawn: the start is not the
  deadline, and VISION says Knowlu never invents a due date.

**DECIDED (Quinn, 2026-09-29): (i), as recommended.** It keeps the one real piece of work in a list, and invents nothing. Its
cost: an undated task sorts last, so a student who never opens the undated list can miss a
registration that closes early. The event itself is still on the schedule. The task is not closed
when the event passes. The student ticks it or deletes it.

**Q1b. Under (b) or (c), where does an accepted all-day, multi-day or past-midnight event go?**
- (i) **The all-day lane.** An all-day or multi-day event is drawn in `the_day`'s all-day lane on
  each of its days. The lane takes no capacity (anatomy §3.7), and Coming up marks the event
  "Accepted". A past-midnight event is a commitment that ends at 23:59, and its body gives the true
  end.
- (ii) **A task** for those shapes only: "Attend: <title>", due on its first day, effort its
  length clamped to 0.25–4 (1 hour when the length is unknown). Capacity counts it before the day.
- (iii) **Nothing** (the draft): the answer is recorded and the event stays in Coming up.

**DECIDED (Quinn, 2026-09-29): (i), as recommended.** It is the only option that puts the event on its day. It invents no hours
for a fair whose length is the student's choice. It costs one read in `surface.rs` (T4b) and no
vault writes.

**Q2. Should a required event still ask, or become part of the plan on its own and be declinable
afterwards?**
- (a) **Ask**: an Accept/Decline card; an unanswered card changes nothing.
- (b) **Auto-add** the commitment and file a card to undo it.

**DECIDED (Quinn, 2026-09-29): (a), as recommended.** A campus feed is not an authority on what this student must attend: the
model inferred it, and VISION commitment 5 says Knowlu proposes and waits when it noticed something.
The miss (cost 3) is covered because the event stays in Coming up while its card waits or after it
expires (D8). (b) would put a model's guess into the student's hard time without a click.

**Q3. Do opportunities keep a digest?**
- (a) **No**: one card per opportunity (or series), expiring after 14 days or at the event. The
  digest stays only for vaults without the switch.
- (b) **Keep the digest** for opportunities, and fix its two defects (F1 and F2) instead.

**DECIDED (Quinn, 2026-09-29): (a), as recommended.** The deck cannot show a digest (`surface.rs:1323-1326`), so a student cannot
answer one at all, and its expiry declines every event in it. Fixing it would mean building a
batch-card UI the anatomy already ruled out ("Event digests — one approval per event", anatomy §6).
The switch keeps `golden-today-full.md` byte-identical. The founder's scratch profile is created by
the wizard, so it carries the switch.

**Q4. Does one answer cover a whole series?**
- (a) **Yes, both ways**: a declined series stays declined, and an accepted series adds its later
  instances as they appear.
- (b) **Declines only**: each later instance of an accepted series gets its own card.
- (c) **Neither**: every instance is asked on its own.

**DECIDED (Quinn, 2026-09-29): (a), as recommended.** VISION already rules the decline half ("a declined recurring opportunity
stays declined across the series"). For the accept half, a weekly required meeting asked about
every week is the notification fatigue VISION designs against. The student asked once, so Knowlu
acts (VISION commitment 5), lists each carried instance in the delta, and the student can delete
any one instance from the schedule.

**What (a) costs.** The accept carry needs a record of the Accept that outlives the note, or a
deleted instance would come back at the next `rank`. §5.3's ever-written set is that record: it
reads `archive/`, where a deleted note goes. So a delete is permanent for that instance. The only
way back is for the student to add it by hand, and there is no "restore this event" control in the
MVP. Carried notes arrive with no card. They are listed in the delta line, which records them but
does not ask about them. That listing needs the console's `seen_at` stamp (§4.5). With no stamp,
because the window never lost focus between the Accept and the carrying run or the stamp file is
gone, the carry is written but not listed. The note is still on the schedule. The delta also expands
at most 200 records (it counts the rest). This spec accepts both limits and does not change `delta`.
Tests 20, 20a, 20b and 20c pin them.

**Q5. Does the Alabama preset switch opportunity cards on before B1 has a result?**
With `event-3`, 47 of 50 labelled campus events came back as opportunities, faculty-only ones
included. D7's cap holds that to 3 cards a day.
- (a) **On now**: cards from the first slot, at up to 3 a day of real noise until B1 or the
  audience filter lands.
- (b) **Off in the preset until B1 is decided**: only required and `unsure` events are asked about;
  opportunities show in Coming up only.
- (c) **On, and run B1 before the founder's MVP proof**, so the proof sees whichever prompt wins.

**DECIDED (Quinn, 2026-09-29): (c), as recommended.** The build is the same either way: one line in a preset. B1's harness is
already built and its seed is 26 cases. The proof is the first time a student meets these cards, so
it should meet the better prompt.

**Review findings of 2026-09-29.** Five findings were checked against the code and accepted, and
none was rejected. Two are only partly met, and each says why:
- **The Alabama count (finding 5)** could not be measured while revising: no file on disk marks an
  event all-day, and fetching the feeds was out of scope for a docs-only revision. T0 measures it
  before Q1 is asked. Q1b gives a fallback that does not depend on the number.
- **The branch file lists (finding 4)** are the reviewer's `git diff`. This revision spot-checked
  three of them: p3's body-line branch, human-actor's token in `app/tests`, and m1-grades' tripwire.
  It did not re-run the diff. The integrator re-runs it before T1 (§9).

**Re-check findings of 2026-09-29.** Two findings were checked against the code on 97dc27b and
accepted, and none was rejected:
- **The delta window (§4.5, tests 20b and 20c).** Confirmed at `surface.rs:1699-1721` and
  `newest_end_ts`. A carry is listed when the console passes `seen_at`, and not listed with none
  when the carrying run is the newest. Test 20b now passes a `seen_at`, 20c pins the fallback, and
  the 200-record display cap is named. The no-stamp gap is accepted in Q4's cost and not raised as a
  separate question. It is narrow because the console stamps at the end of every look, and an Accept
  needs a look. If Quinn wants carried notes listed even then, that is a change to `delta` and a
  question of its own.
- **One `answered_series` (§4.1, §6.1, §13).** Confirmed: T2a would have had to copy T2b's
  predicate. The predicate now has one definition in `eventcarry.rs`, with its edge rules written
  down. T2b lands it first, and T2a, the carry and `judge_roster` all call it.

## 13. Task sketch

One branch and one worktree, cut from main after signing, and after `human-actor` and `m1-grades`
if they have merged by then (§9). The files are disjoint by task with two exceptions:
`engine/src/lib.rs` (both `mod` lines, T1), and `engine/src/events.rs`, which T1 and then T2b edit
in sequence. Each task is test first: the tests named from §11 fail before the code exists. Reviews
land in `docs/reports/`. A cheaper agent's work goes through `reviewer` before any push.
`approvals.rs`, `eventcarry.rs` and T4's `cli.rs` hunk go through `contract-reviewer`.

| Task | Agent | Files | Tests (§11) | Why this agent |
|---|---|---|---|---|
| **T0** The Alabama count | `researcher` (Sonnet, medium) | none. It returns counts, and the controller adds the number to Q1 | — | Read-only research over public feeds, before signing. It counts the next 60 days of the Alabama preset's feeds by shape: timed one-day, past-midnight, all-day and multi-day. |
| **T1** Builders and the switch | `implementer` (Sonnet, high) | new `engine/src/eventaccept.rs`; `engine/src/events.rs` (`event_cards`); `engine/src/commitments.rs` (the `kind: event` body line); `engine/src/lib.rs` (both new `mod` lines) | 8, plus the loader's parse | Fully specified, pure (no I/O), compiler- and test-checked, and off the contract list. It writes nothing to a vault. |
| **T2a** The emitter | `implementer` (Sonnet, high) | `engine/src/eventemit.rs` | 1–7, 25 | It starts after T2b's first commit and calls `eventcarry::answered_series` from it. It does not write its own predicate. Off the list, and it generalises `emit_event_checks`, whose shape it keeps. What it writes is a card and its `proposed` line. A wrong card is a proposal the student sees and answers, never a silent change to their plan. `reviewer` checks it before any push. Two failed attempts go to Opus, high. |
| **T2b** The series carry and D9 | `contract-engineer` (Opus, xhigh) | new `engine/src/eventcarry.rs`; the `judge_roster` call site in `engine/src/events.rs` (after T1) | 19, 20, 20a, 21, plus unit tests for the ever-written set and for `answered_series`' edge rules (§6.1: `expired`, a missing or empty `series_uid`, an `event-check` card not counted, two cards for one series). Its **first commit** is `answered_series` and those unit tests alone. T2a starts from it. | A silent error here declines or hard-books a whole series, or brings back a note the student deleted, with no card to catch it. CLAUDE.md sends that to Opus. The roster has no Opus-`high` implementer, and contract-engineer's description names this case. `contract-reviewer` reviews it with T3. |
| **T3** The settlement | `contract-engineer` (Opus, xhigh) | `engine/src/approvals.rs` | 9–17 | On the contract list. A silent error here writes the wrong note into a student's vault, or none. It calls T2b's ever-written set, so it runs after T2b. |
| **T4** `rank` wiring | `contract-engineer` (Opus, xhigh) | `engine/src/cli.rs` | 18, 20b, 20c, 22, 23, 24 (the test sits in `enrich.rs`; no code changes there). It never edits `surface::delta` (§4.5). | It sets the order of the vault-writing passes (the carry and D9 before any card) and the budget each gets. A wrong order files or carries against stale state without a word, so it goes to Opus for the reason T2b does. `cli.rs` is shared with `p3-registrar` (§9), and the integrator applies the hunk at merge. It runs after T1–T3. |
| **T4b** The all-day lane (Q1b) | `implementer` (Sonnet, high) | `engine/src/surface.rs` | 22a | Read-only: `surface` never writes, so an error misdraws a lane but never changes vault bytes. `surface_oracle.rs` must pass unchanged. It runs only if Q1b is (i), after T3. |
| **T5** Deck labels | `console-ui` (Sonnet, medium) | `app/static/console.js`; `app/tests/static_assets.rs` | 26 | `app/static` is console-ui's. |
| **T6** Presets | `mechanical` (Sonnet, low) | `app/assets/campus/*.yaml`; `app/tests/scaffold.rs` | 27 | Two one-line asset edits and one assertion. |
| **T7** `decide` end to end | `test-writer` (Sonnet, medium) | `app/tests/commands.rs` | 28 | A test only, over a finished engine. It runs after T3. |
| **T8** Documents | `docs-keeper` (Sonnet, medium) | `docs/surface/anatomy.md` §3.7 (the all-day lane, if Q1b is (i)), §3.9 and §3.11; `docs/reference/engine-commands.md` (`rank`); `docs/notes/2026-09-29-vision-program.md` (the P4 row); `HANDOFF.md` | — | Documents only. |
| **T9** B1 | `cloud-engineer` (Opus, high), on `j-events` | `scripts/experiments/e1-decomposition/` (run, not edited); a report in `docs/reports/` | B1's own | Cloud-side, and needs the OpenRouter key from Credential Manager. Only if Q5 is (c). |
| **T10** Live proof | main session | none (a scratch profile) | the live proof in §11.4 | It needs a staging session by OTP and a desktop. Neither is delegated. |

**Order:** T0 before signing. After signing, T1. Then T2b's first commit (`answered_series`, the one
definition). Then T2a (implementer) runs in parallel with the rest of T2b and then T3
(contract-engineer, in sequence: T3 calls T2b's set). All three test against §5.1's card, built by
hand. Then T4, then T4b, T5, T6 and T7 in parallel, then T8. T9 runs in its own lane
at any time before T10.

**Checkpoints for Quinn:**
- at signing (§12), with T0's count in Q1;
- after T2b's and T3's contract review, with the carry's and the settlement's diffs and the report;
- before T10, with the whole-branch review.

**Size:** L overall. T2a, T2b and T3 are the weight, each with its own tests. The rest are small.

## 14. What signing changes elsewhere

- Q1 was decided (c), so Amendment 2026-09-29, ruling 10's MVP list is reworded in the signing
  commit: required events enter the plan as a one-off commitment, with the marker *(amended
  2026-09-29, events spec Q1: enter the plan as a one-off commitment, signed by Quinn)*.
- `docs/notes/2026-09-29-vision-program.md`'s P4 row points at this spec (T8).
- Nothing in `VISION.md` changes. The spec applies commitments 4 and 5 and the Learning section's
  series rule as they stand.
