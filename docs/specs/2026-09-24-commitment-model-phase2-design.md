# The commitment model, phase 2: the app

**Status: DRAFT 2026-09-24. For Quinn's review.** The parent spec is
`2026-09-23-commitment-model-design.md` (on `p1-commitments`, PR #14). It already designs the three
pieces: the confirm screen (§5.1), the per-course fallback card (§5.3) and the *Your week* panel with
the planning-day editor (§6.4, §10 phase 2). This document does not repeat that design. It records
where phase 2 departs from it, settles the calls the parent left to "the phase-2 plan", and names
the files, commands and tests. Where this document is silent, the parent spec governs.

Inputs: the parent spec; C1c's design (`2026-09-22-c1c-first-day-design.md`, PR #13), which owns
the wizard and the first-run view; the friends-shell design, whose decision 3 says the wizard writes
nothing before Finish (except credentials); and Quinn's rulings in memory:

- **Login-only onboarding:** students bring only their logins.
- **The planning day:** 8am–10pm by default, editable at any time, and the student sees how an edit
  changes the suggestions.
- **Classes come from both sources:** the registrar where a school is supported, plus a per-course
  card.

## 1. Decisions

- **D1. The confirm screen is the console's first screen, not a wizard panel.** The parent spec puts
  it right after the calendar step and before the first slot. That cannot work as written:
  - The wizard writes nothing before Finish.
  - The vault does not exist until then.
  - `knowlu-engine commitments` needs the vault: the course notes for the code table, and
    `config/cloud.yaml` for the session.

  So the screen opens in the console right after Finish, over C1c's first-run view, while the first
  slot runs. That wait is about a minute (C1c §0), and the screen gives it a use. The wizard keeps
  its nine panels, and `the_wizard_has_nine_panels` does not change.

  *Cost if wrong:* nothing gets asked before the first slot starts. D2 makes that harmless.
- **D2. No `commitment-check` card is filed on the vault's first day.** The vault's first day is the
  local date, in the vault's timezone, of its earliest journal record (amended A1; the journal
  file's name is a UTC date and only a fallback). The parent spec's §5.3 day-3 rule counts from
  the same date. On that day, `emit_checks` files no proposal card and no window card. Without this,
  the first slot's `rank` would file cards for the rows the student is confirming on the screen at
  the same moment. The withdrawal rules would clean those cards up, but each would already have
  been charged to the 15-a-day cap and shown once.

  A vault with no journal file yet is on its first day. A student who skips the screen gets the cards on day 2. Change cards cannot exist on day 1, so
  they are unaffected.

  *Cost if wrong:* a student who skips the screen waits one more day to be asked.
- **D3. The screen shows when the vault is on its first day and has no `planning-day` note.** No new
  state file is needed:
  - Finish on the screen always writes the note (§3), so the screen never comes back after Finish.
  - "Not now" hides it for the rest of that console session.
  - From day 2, the cards take over, as the parent spec's §5.1 says for a student who leaves
    without Finish.
- **D4. The screen's writes are an engine command: `commitments --confirm <file>`.** The parent
  spec's §5.1 left the choice between a mode of `commitments` and an app command to this plan. It is
  a mode of `commitments`:
  - The app passes keys, levels and a window, never event data (§3).
  - The engine re-derives each row from the series file, then writes through the existing
    `create_confirmed` and `create_marker`.
  - The *Your week* panel's "add" and the window editor's save use the same mode.
  - The actor is the console's human context (`console_ctx()`, via `dashboard`), as §5.1 says.
- **D5. `window` is written by the engine, not added to `set_fields`' `EDITABLE`.** The parent
  spec's §10 said to add `window`, `level` and `kind` to `EDITABLE`, with sequence validation in the
  app. That would put the §2.2 validation in the app, and `app/src/commands.rs` computes nothing.
  Also, a window save must create the `planning-day` note when there is none, which `set_fields`
  cannot do.

  So:
  - The window goes through `commitments --confirm`, which creates the note or edits its one
    `window:` line.
  - `level` and `kind` are enumerations, so they join `EDITABLE`. `set_fields` refuses both on any
    note outside `commitments/`, and refuses any value outside the parent spec's §2.2 lists.

  *Cost if wrong:* none known.
- **D6. The panel is a new console view, *Schedule*, headed "Your week".** The nav already has
  *This week* (tasks due this week), so a second "week" link would be ambiguous. The view id is
  `schedule`. It is a page view, not a read-model view: it polls the read model as `today`, and
  `schedule` never reaches `surface::View::parse` (amended A4).
- **D7. The console says what moved.** The today view prints `moved.text` under the day's
  heading when the read model carries it (§6.4). Phase 1 put `moved` in the read model and left
  showing it to phase 2.
- **D8. There is no "remove" control in phase 2.** Setting a commitment's level to *optional*
  already tells Knowlu not to plan around it (§2.2). Removing a note would need a decline path for
  a confirmed note's key, and nothing asks for it yet.

  *Cost if wrong:* one control, added later.

## 2. The confirm screen ("When do your classes meet?")

- **Where it appears.** `#week-setup` is a new block in `index.html` that sits over the day and the
  first-run view. It opens when `your_week` (§5) reports `setup: true`, meaning D3 holds.
- **How it loads.**
  - Opening the screen calls `commitment_proposals`. That runs
    `knowlu-engine commitments --vault <v> --json` as a child process, the same way `discover_coursework`
    runs `coursework-discover`.
  - The call fetches from Google, so the screen says "Reading your calendar…" while it waits.
  - The first slot's `rank` may refresh `state/calendar-series.json` at the same moment. Both writes
    are atomic and come from the same account's calendars, so whichever lands last is correct; `--confirm`
    reads whatever is there, and a key that went missing between the two is warned and skipped.
  - A failed or empty fetch shows only *Your day*, plus the course rows below. It never shows an
    error wall.
- **The groups.** These follow the parent spec's §5.1.

  | Group | What it holds |
  |---|---|
  | *Your classes* | One row per `class`/`lab` proposal. Each row shows the title, then the days and times in the §5.2 card format (`Mon/Wed/Fri 12–12:50pm`), then `where` in small type. |
  | *Your week* | `work`, `club` and `meeting` rows. |
  | *Office hours* | Office-hours rows. |
  | *Your day* | The window editor from §4. It is pre-filled from the window proposal when there is one, otherwise 08:00–22:00 every day. |

  Rows are sorted as the cards are (§5.2's order).
- **Courses with no class row.** A course with neither a class proposal nor a confirmed class note is
  listed under *Your classes* as "BUI 100 — no class times found; Knowlu will ask this week". The
  engine computes this list: `commitments --json` gains a key, `uncovered_courses: [{slug,
  title}]`. The new key is additive; `proposals` and `warnings` do not change.
- **Row controls.**
  - *Mine* and *Not mine* are two toggle buttons. With neither pressed, the row is unanswered.
  - Class, lab and work rows start at *Mine*; every other row starts unanswered.
  - A level control offers *Must keep*, *Usually* and *Optional*, which write `hard`, `soft` and
    `optional`. It is shown only on a *Mine* row, and starts at the proposal's `level`.
  - Nothing on the screen accepts typed text.
- **The buttons.**
  - **Finish** sends one `commitments_confirm` with every *Mine* key and its level, every *Not mine*
    key, and the window. It then closes the screen and the day repaints.
  - **Not now** writes nothing.
  - Both say what they do in one line under them: "Rows you leave blank will come back as questions
    over the next few days."

## 3. `commitments --confirm`

`knowlu-engine commitments --vault <v> --confirm <path> [--today YYYY-MM-DD]`. The file is JSON:

```json
{"mine": [{"source_uid": "gcal-series:…", "level": "hard"}],
 "not_mine": ["gcal-series:…"],
 "window": "[{days: [mon, tue, wed, thu, fri], start: \"08:00\", end: \"22:00\"}]"}
```

Every key is optional. The app writes the file in the profile's temp folder and deletes it
afterwards. It is a file rather than an argument so that no argv length limit applies.

- **The series file is the source; there is no fetch.** Each `mine` and `not_mine` key is looked up
  among the proposals derived from `state/calendar-series.json`, the same `proposals()` the cards
  use. A key that is not a current proposal is skipped with a warning. Nothing from the app becomes
  a field value except `level`, which must parse as a §2.2 level or the key is skipped.
- **`mine`** creates the confirmed note with `create_confirmed`, using the chosen level. It is
  skipped, with no warning, when a confirmed note already has the key or signature: a second Finish
  is a no-op.
- **`not_mine`** creates a decline marker with `create_marker` for every key of the row's twin
  group (§3.3), as a card rejection does.
- **`window`** is parsed with `parse_window`. If it is invalid, the command writes nothing at all and
  exits 2. If no `planning-day` note exists, the command creates one. Otherwise it writes the note's
  one `window:` line through `write`, as a human edit that `journal.human_set` will protect.
- **The writes.**
  - The actor is `quinn`, via `dashboard`: the console's human context, passed in with `--via`
    (default `dashboard`).
  - Journal first, as for every write.
  - Nothing is charged to the cap, because no card is filed.
- **Order and atomicity.** The window is validated first, then the writes run in input order. A write
  that fails is reported and the rest still run. Every write is individually idempotent, so a
  re-run finishes the job.
- **Output** (`--json` is implied):

  ```json
  {"created": n, "declined": n, "window": "created"|"updated"|"unchanged"|null, "warnings": [...]}
  ```

  The exit code is 0 for this output, and 2 for unreadable input or an invalid window.
- **Pending cards.** Any card still pending for a confirmed or declined key is withdrawn by the
  existing rules (§5.2 *Withdrawn*) at the next `rank`.
- **`rank_cannot_reach_a_judgment_endpoint`** already covers the `commitments` arm. `--confirm`
  makes no network call, and a test pins that.

## 4. The *Schedule* view ("Your week") and the window editor

- **Data.** The `your_week` Tauri command is an in-process read, like `state`, and does no fetch.
  It calls a new pure engine function, `commitments::overview(vault, today) -> Overview`, which
  returns:
  - `commitments`: every confirmed note, with `id`, `path`, `kind`, `level`, `title`, `course`,
    `meets`, `where`, `from` and `until`
  - `office_hours`: the office-hours proposals from the series file, not yet confirmed or declined
  - `window`: the effective window per weekday, with `source: "note"|"template"`
  - `uncovered_courses`: as in §2
  - `setup`: the D3 test
  - `warnings`

  It is serialised through `ledger::dumps_value`.
- **Commitments.** Each confirmed commitment is one row, showing the title, the times, a kind select
  (`class`, `lab`, `work`, `club`, `meeting`, `office-hours`) and the level control. A change goes
  through `set_fields(id, {kind|level})` (D5). The planning day is not listed as a row: it is the
  editor.
- **Office hours.** Each office-hours row has an **Add** button. It sends `commitments_confirm` with
  `{mine: [{source_uid, level: "optional"}]}`, since office hours default to optional (§2.2).
- **Uncovered courses.** Each one shows "Knowlu will ask when it meets" (the §5.3 card). There are no
  controls.
- **The window editor.** It has seven weekday rows, Mon to Sun, each with a start and an end
  `<input type="time">`, plus a "same as Monday" shortcut for Tue–Fri.
  - **The flow sequence.** `console.js` builds the sequence from its own pickers, grouping identical
    days into one entry. That is string assembly, not validation.
  - **Preview.** After an edit (debounced 400 ms), `preview_window(window)` calls
    `surface::build_state_preview` in-process, with the today view.
  - **What the preview shows.** Beside the editor, the panel shows:
    - the preview's `moved.text`, or "No change to today's plan"
    - the first five items of the previewed day, in order
  - **Save** sends `commitments_confirm({window})`, then refreshes. An invalid window, such as a start
    after its end, comes back as the engine's message and shows under the row. Nothing is written.

  The editor is the same component inside `#week-setup` (§2) and in the *Schedule* view: the same
  pickers. The preview runs only in the *Schedule* view, since on the first day there is no plan
  yet to diff against (amended A6).
- **The today view.** It shows `moved.text` under the day heading (D7), with no control.

## 5. The per-course fallback card (`commitment-ask`)

The engine half follows the parent spec's §5.3 exactly. Phase 2 settles these three calls:

- **The emitter.** `commitments::emit_asks(vault, proposals, today, budget, ctx, journal)` runs in
  `rank` after `emit_checks`, with what is left of the budget (amended A2: it needs the proposals
  to see a pending class proposal).
  - It files at most 2 cards a day.
  - It files nothing before the vault's day 3, counted from D2's first day.
  - It files cards through `file_card`, whose `LOCAL_CARD_KINDS` already allows `commitment-ask`.
  - It picks courses in slug order.
  - Each card filed adds one to `approvals.pending`, as the check cards do.
- **The answer.** The new `answer_card(id, meets)` Tauri command sits beside `decide` in
  `commands.rs`.
  - It writes the card's `answer_meets` through `write`, as a human edit, with `meets` passed as the
    form's flow sequence and emitted by `write::to_literal`.
  - It then runs `decide(id, "approved")`'s path. The settlement validates the answer. An invalid
    answer puts the card back to `pending` with the warning. The console shows the engine's
    warning on the card, from `answer_card`'s envelope, until the next repaint (amended A3).
  - The class note it creates is titled by the course note (`title`, else `name`, else the slug),
    at `commitments/<slugify(title)>.md` (amended A5).
  - The app validates nothing. The engine refuses `answer_card` on anything but a pending
    `commitment-ask` card.
- **The form.** `renderDecisionsView` gains a per-kind branch. For a card of kind `commitment-ask`,
  the body is:
  - day toggles, Mon to Sun
  - a start and an end time picker
  - "add another time"
  - **Save times**, which calls `answer_card`
  - **No set times**, which rejects the card

  Snooze stays. Every other kind renders as it does today. In the rail's deck, an ask shows
  **Answer…** in place of Approve, which opens Decisions; Reject and Snooze stay (amended A7).

## 6. Commands, files and counts

- **New Tauri commands, all in the console window.**

  | Command | Where it lives | What it does |
  |---|---|---|
  | `commitment_proposals` | `week.rs` | spawns the engine |
  | `commitments_confirm` | `week.rs` | spawns the engine, with a temp file |
  | `your_week` | `week.rs` | in-process |
  | `preview_window` | `week.rs` | in-process |
  | `answer_card` | `commands.rs` | |

  `week.rs` is the new `app/src/week.rs`. The console window's count goes from 42 to **47**, and the
  wizard window stays at 29. `CLAUDE.md`'s count is recounted by script.
  - **Notes that change:** `commitments_confirm` and `answer_card` mutate notes. That takes the list
    of mutating commands from seven to nine.
- **Engine.**
  - `commitments.rs`: `overview`, the confirm function, `emit_asks`, the settlement arm for
    `commitment-ask`, and `uncovered_courses`.
  - `main.rs`: the `--confirm` flag.
  - `cli.rs`: the day-1 gate in the card pass (D2), and the `emit_asks` wiring.
  - `approvals.rs`: the `commitment-ask` arm.
- **App.**
  - `app/src/week.rs` (new)
  - `commands.rs`: `EDITABLE` and its checks, and `answer_card`
  - `main.rs`
  - `app/static/index.html`, `console.js` and `console.css`
- **Docs.** `docs/surface/anatomy.md` (the view), `app/README.md`, and `CLAUDE.md`, whose command
  list gains `--confirm`.

## 7. Tests

- **Engine.**
  - `--confirm` writes exactly the rows given, at the given level, with a human-context journal line.
  - Unknown or stale keys are warned and skipped.
  - A second Finish is a no-op.
  - Twin groups are declined together.
  - An invalid window writes nothing and exits 2.
  - A window edit on an existing note changes one line.
  - The day-1 gate files no check card.
  - `emit_asks`: the day-3 rule, 2 a day, the decline marker, withdrawal, and an invalid answer going
    back to pending.
  - `uncovered_courses`.
  - `overview`'s shape.
  - The no-network pin for `--confirm`.
- **App.**
  - `app/tests/week.rs`:
    - `your_week` on a scratch vault
    - `preview_window` reports `moved`
    - `commitments_confirm` against the real sibling engine
  - `app/tests/commands.rs`:
    - `EDITABLE` refuses `kind`/`level` outside `commitments/`, and refuses bad values
    - `answer_card` writes and approves
- **Static and behavioural.**
  - `static_assets.rs` pins `#week-setup`, the `schedule` nav link and view, the per-kind card
    branch, the moved line, and the 400 ms debounce.
  - `scripts/wizard-check.py` gains a console scenario, with fake invokes, covering:
    - a first-day vault shows the screen
    - class rows are preset to *Mine*
    - Finish sends one `commitments_confirm` holding the expected keys
    - Not now sends nothing
- **Unchanged.** The frozen fixtures, the surface-oracle JSONs (none of their vaults has a first
  day, a planning note or courses without class notes that would reach them), and the wizard's nine
  panels.

## 8. Sequencing

- **What phase 2 builds on.** Phase 2 needs phase 1's engine (PR #14, stacked on #12 and #11) and
  C1c's app (PR #13). It is built on a branch `p2-commitments`, cut from `p1-commitments` with
  `c1c-first-day` merged in. A test merge came out clean (`git merge-tree`, 2026-09-23). Once #13
  and #14 are on `main`, the branch rebases onto it.
- **C3′.** The parent spec's §10 lists C3′ as a prerequisite. Phase 2 adds no synced note shape
  beyond phase 1's, so P21 covers it; C3′ does not block phase 2.
- **Privacy.** Phase 2 reads no new data, so it adds nothing beyond the parent spec's §9 line.

## Amendments (2026-09-24, plan review)

From the pre-execution review of the plan (`docs/plans/2026-09-24-commitment-model-phase2-plan-review.md`,
"ready with fixes"). The parent spec is not edited; where it disagrees, these amendments supersede
it. They take effect with Quinn's go at the checkpoint.

- **A1 (D2; supersedes parent §5.3's "earliest `state/journal/` file" wording).** The vault's first
  day is the local date, in the vault's timezone, of its earliest journal record. It is not the
  date in the earliest journal file's name. The file name is a UTC date, and after local midnight
  east of UTC it names local yesterday: that student's first real day would be day 2, the screen
  would never open, and the day-1 cards would be filed. The file-name date is used only when no
  record's timestamp can be read. Parent §5.3's day 3 counts from the same date. (Plan Q1-a;
  review finding 1.)
- **A2 (§5, the emitter).** The signature is `emit_asks(vault, proposals, today, budget, ctx,
  journal)`: "no pending class proposal" cannot be tested without the proposals, and `rank`
  already holds them. (Plan Q6-a.)
- **A3 (§5, the answer).** An invalid answer's warning is shown on the card from `answer_card`'s
  envelope, until the next repaint. The read model gains no warning field: that would need a
  second write onto the card for a message the student has just seen. (Plan Q7-a, Q8-c.)
- **A4 (D6).** `schedule` is a page view. It polls the read model as `today` and never reaches
  `surface::View::parse`, which refuses any name that is not a read-model view. (Plan Q10-a.)
- **A5 (§5; supersedes parent §5.3's `commitments/<slug>.md`).** The class note an answered ask
  creates is titled by the course note (`title`, else `name`, else the slug). It is written at
  `commitments/<slugify(title)>.md`, which for a wizard-made vault is `<slug>.md`; §2.2 already
  requires a title, and a file name is never an identity. (Plan Q7-b.)
- **A6 (§4, the editor).** The confirm screen and the *Schedule* view share the same pickers.
  The preview runs only in the *Schedule* view, because on the first day the first slot has not
  ranked yet and there is no plan to diff against. (Plan Q11-a; review finding 7.)
- **A7 (§5, the form).** In the rail's deck, a `commitment-ask` card shows **Answer…** in place of
  Approve, which opens Decisions; Reject and Snooze stay. The parent's §5.3 warns that "a card the
  console cannot answer would teach the student to reject". (Plan Q11-b.)

The review's ruling on the plan's sixth flagged problem needs no spec change. The P16 `rank` test
vaults gain a seeded journal day and, in one test, a `card:cs-100` marker; these are setup only,
never assertions.
