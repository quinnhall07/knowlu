# Anatomy of the Ops Console

**Required by** program record §11.3. **Written** 2026-08-31.

## What this file is for

`docs/surface/inventory.md` says *where every element came from*. This file says *what each region
is, why it exists, and what it must not become.* It exists because of one ruling:

> **Quinn, 2026-08-31:** *"I can open a Claude session and have things understood and altered by AI
> easily."*

That is an architectural constraint. A future session must be able to change a region **correctly**
without reading `engine/surface.py` and without re-deriving a decision that was already argued out.
So every region below carries four things: the question it answers, what computes it, which ruling
put it there, and what was deliberately left out.

**If you are about to delete or reshape a region, read its "left out" line first.** Most of them
record something that was already tried and rejected.

---

## 1. The constraints that shape everything

| Constraint | Consequence |
|---|---|
| **The file you read is the file that ships** (§11.3) | No JSX, no bundler, no minifier, no generated CSS. The only substitution is the `ops-token` meta tag. |
| **Tokens at the top of one stylesheet** (§11.3) | Restyling the whole page is editing ~30 values in `console.css`. Never hard-code a colour or a gap. |
| **Not keyboard-driven** (§11.4) | No `J`/`K`, no `A`, no `⌘K`, no command palette. Every action has a visible control. Work's filter box is a plain text input, not a launcher. |
| **A plan, not a feed** (§11.7) | Polling never reorders under the cursor. If ranking changed, the delta line says so and offers `refresh order`. |
| **Every section always renders** (§12.1) | No region is conditionally hidden. Empty states are designed and named. |
| **Ranking is soft** (§12.2) | Sort is `start_by` then `due`. Importance is displayed, not sorted by. |
| **Nothing lives only in a UI** (VISION.md) | Every visible fact comes from a note, a ledger or a run record. The console renders and edits; it never stores. |
| **All writes go through `engine.write`** | `actor="quinn"`, `via="dashboard"`. The console is a caller, never a writer. |
| **`rank` never calls a model** (Knowlu spec decision 11) | Judgment is `knowlu-engine judge`, a separate command and a separate slot step, writing fields into notes before `rank` reads them. Nothing the read model or the renderer touches can reach a model, a socket or a subprocess. |

---

## 2. The page set

Nine pages, one window (§12.5 #1, §12.6.4). The rails persist; the main column swaps.

| Page | The question it answers |
|---|---|
| **Today** | What do I do right now? |
| **Work** | Everything, filterable — horizon filters plus a text filter box |
| **Week** | Seven days: capacity, hours due, commitments — and where **pull-ahead** lives |
| **Decisions** | The full approval queue, every kind and urgency |
| **Info** | Things to know, open/close — including grade postings and late/missing flags |
| **Grades** | Projected term GPA vs 3.75, per-course standing, needed-on-remainder |
| **Issues** | Flagged judgments, cleared in one sitting |
| **Health** | Runs, sync, ingest and source status |
| **Schedule** | What is my week made of, and which hours do I plan in? (§3.15) |

**Left out:** separate `Overdue` / `This week` / `Later` destinations. They are *filters inside
Work*. The mockup's nav implied three pages; three pages that differ only by a date predicate is
three places to maintain one list.

A nav view may be a page view, not a read-model view, as long as it polls `state` as `today`
(`stateView()`): *Schedule* is the one.

**Four panels that are not pages** (Knowlu plan 4a; the confirm screen, commitment model phase 2). None of them is a view: a view name reaches
`surface::View::parse` on every poll and would be refused (R-P4a-4), so none has a hash, none is in
the nav, and none is an observed region. Each is markup in the same document, put up and taken down
by one function.

| Panel | What it holds | How it is reached |
|---|---|---|
| **Picker** | one row per registered profile — name over vault path, with *Open* — and two buttons, *Use an existing vault…* (adopts a folder) and *Create a new vault…* (the wizard) | a launch with no `--vault` and more than one profile registered; or `--pick`, which is the settings overlay's *Switch profile…* |
| **Wizard** | seven panels in order — Welcome, Your vault, Backup folder, Your LMS calendar, Coursework logins, Slots and campus, Finish. Back is always allowed and **nothing is written until *Finish*** except the Credential Manager entries, which are written when the credentials panel is left | a launch with no `--vault` and no profile registered; or the picker's *Create a new vault…* |
| **Settings** | seven rows over the live page — profile name, vault path, backup folder, start with Windows, updates, local judgment, diagnostics | the topline gear, or *Settings* in the tray (which `eval`s `window.KNOWLU_OPEN_SETTINGS`) |
| **Confirm screen** (`#week-setup`) | "When do your classes meet?" — four groups: *Your classes* (with the courses that have no class row), *Your week*, *Office hours* and *Your day* (the window editor). Each row has Mine / Not mine / unanswered, and a level control on Mine. *Finish* writes through `commitments_confirm`; *Not now* writes nothing and hides it for the session | `your_week` says `setup` (the vault's first day, no planning-day note), over the first-run view |

The picker and the wizard replace the whole document and run in a **vault-less window** — no
`ConsoleState`, no tray, no scheduler, no autostart registration, because none of those has a vault
to point at yet; they end by *relaunching* into the console, never by acquiring a state after the
fact. The settings panel is an **overlay** over a running console, and nothing inside it carries a
`data-id`.

**Local judgment** is the seventh settings row (plan 3a). It has a state for every outcome —
`runtime not installed`, `model not installed`, `ready — 2.1 GB model` — and "not installed" is
rendered as an ordinary state, never as an error, because running with no model is the free tier's
zero-cloud default and not a fault. Its two install buttons are two doors to one rule: **a runtime
is executed only if its SHA-256 is in `inference::SUPPORTED_RUNTIMES`**, a table compiled into the
app, whichever door it came through (ruling R-P3a-2). A refusal names the digest, so the answer to
"why won't it take my file" is a string the reader can send us. *Left out:* a progress bar. A
two-gigabyte download would want one, and the row deliberately says `installing…` instead: the
command is `(async)`, the page has no channel to stream progress over, and inventing one for a
button pressed once per install is cost without a reader. *Also left out:* pinning model digests —
a `.gguf` is data parsed by a runtime the table already vouched for, and the spec allows any 1–4 B
Q4 model, so a pinned list would make the row useless to anyone who wants a different one.

---

## 3. Today, region by region

### 3.1 Topline

- **Answers:** is this page current, who made it, and how far behind am I?
- **Computed by:** `surface::topline → state.topline` (Rust port, console plan 1 Task 6/7) — shares
  its `runway()`/`status_word()` helpers with `surface::verdict`; reads the newest `end` run record
  and `l.ranked.len()`.
- **Carries:** date · synced-at · unsynced count · generated-at · **runner name** · runway figure
  with its status word (behind / tight / steady / ahead) · active count.
- **Why the runner name is non-negotiable:** it is the diagnostic that separates "the cloud runner
  is dead but the local one keeps refreshing" from health. CLAUDE.md treats a missing cloud line as
  a symptom; dropping the name makes that symptom invisible. The mockup's `synced 09:12` dropped
  it — that was a regression, not a simplification.
- **The sync/backup/scheduler line (Knowlu plan 1, Task 15; rebuilt on the account, C3′ Task 7) rides
  under the topline, on `state.topline.{sync,backup,startup_missed,last_slot,scheduler}`.** Unlike
  every other field this document names, **these are not written by the engine's `surface::topline`**
  — `app/src/commands.rs`'s `build_state_value` copies `topline.sync` / `topline.backup` /
  `topline.startup_missed` straight from `ConsoleState`'s cached `knowlu_engine::sync::SyncStatus`
  (`cs.sync`, filled by `state::run_sync` and by the slot's own `sync` step through
  `state/sync-status.json`) and `BackupStatus` (`src/backup.rs`), and `attach_scheduler` copies
  `topline.last_slot` / `topline.scheduler` from the running `Scheduler` on every poll (spec §3.1:
  "nothing in `commands.rs` computes" — this is copying, not computing). **`topline.auto_sync`,
  `topline.engine_newer` and `topline.vault_head` are gone** (C3′ Task 10, hand-off H9b): they
  compared the vault's git HEAD against this build and reported whether auto-sync was on, and a vault
  has not been a git repository since git left the product. The console's own build stays —
  `topline.console_build` — the diagnostics blob and the issue report both still name it.
- **`startup_missed`** is the scheduler's count of slots that were due while the app was not
  running (Task 12 [of plan 1]) — the console's own answer to "was I asleep for a run."
- **Left out:** a weather line (parent design `:214`, never built, never re-requested).

### 3.2 The delta line

- **Answers:** what changed since the last run?
- **Computed by:** `surface::delta → state.delta`, reading `Journal` records since the last `end`
  run, grouped by `(op, field, actor class)`.
- **Window is "since the last run"** (§12.5 #2) — not "since I last looked". The same line then
  means the same thing to any observer, which matters once other people use this.
- **Reads like:** `since 08:04 — 2 ticked by agent · 3 new approvals · CS 100 P2 date moved`.
  Expands to individual records: object, field, old → new, actor, run id.
- **Two record types earn special rendering:** a `supersede` reads *"laptop's 60% won over the
  agent's 100%"* (S1 §4.2's sync resolution, otherwise invisible); a `via: external` reads as an
  Obsidian edit (`detect_external` journals these and nothing has ever shown them).
- **Left out:** a one-line *reason* per change. The design doc asked for it (`:216`); what exists is
  provenance. See §7 — it is now being built, sourced from `importance_reason` and the derivation
  inputs, not invented.

### 3.3 Verdict and lede

- **Answers:** in one sentence, what kind of day is this?
- **Computed by:** `surface::verdict → state.verdict` — the status word plus the two figures the
  lede cites (must-do hours, capacity hours).
- **Prose, generated from real numbers.** Never a fixed string; never a number that disagrees with
  the meter below it.
- **The moved line.** The today view prints `moved.text` under the headline (`#moved`, spec D7);
  hidden when `moved` is absent (§3.7).

### 3.4 The capacity meter — the signature element

- **Answers:** does today fit? Answered before any number is read.
- **Computed by:** `surface::meter → state.meter` — `fit_blocks = floor(capacity × 2)`,
  `spill_blocks = max(0, ceil(must_hours × 2) − fit_blocks)`. One block = 30 minutes.
- **The form is the argument** (§11.6): blocks that fit sit inside a bordered, accented container;
  blocks that do not are drawn **outside it in outline red**. Today that reads 10 in, 39 out.
- **Spill blocks are hoverable and name their tasks** (§12.6 #5). This is Runway's threshold strip
  folded in: the meter answers *how much* doesn't fit, the hover answers *what*.
- **Capped at 120 spill blocks**, then `… and N more`. A freshman in week three generates unbounded
  spill and a meter that fills the viewport has stopped answering its question.
- **Keep the sentence too.** `⚠ Must-do work (26.4h) exceeds today's capacity (10.0h)…` renders
  under the meter with both figures. The meter *shows* it; the sentence *says* it, and the two
  figures are the ones a person repeats out loud.
- **Left out:** a day-shape timeline and a course heat strip, both compared and rejected in §11.6.

### 3.5 The slack gauge

- **Answers:** am I gaining or losing ground over weeks? — VISION criterion 4.
- **Computed by:** `surface::gauge → state.gauge`, reading `state/runs/`, one point per run. **Ported
  and wired into the payload (console plan 1 Task 7); no console region renders it yet** — this
  section still describes intent, not a landed page element, and `state.gauge.direction` is always
  `no-history` until the cutover closes and a writer exists (spec §4.5).
- **The metric is `days_to_recover` = deficit hours ÷ daily budget** (§12.5.3), where deficit =
  must-do remaining minus capacity, floored at 0. Zero means even. Direction word: gaining /
  holding / losing.
- **Why not literal aggregate slack:** summing `slack_days` over dated tasks is dominated by
  far-future work and falls whenever a task is merely *added* — it would report losing ground for
  good reasons.
- **This is not the capacity meter.** The meter asks *does today fit*; the gauge asks *is the week
  going better than the last one*. Both belong; neither substitutes.
- **The counts must be recorded from the first run**, even before the gauge renders. Trend history
  cannot be backfilled — past task states are gone.

### 3.6 MUST DO

- **Answers:** what is actually at risk?
- **Computed by:** `surface::must_do → state.must_do`, via `ranking::partition_must_do` (slack ≤ 0),
  grouped by horizon, sorted `start_by` then `due`.
- **Row carries:** pip · title · source chip · **`start by`** · **slack** · progress track · hours ·
  ⚑ · optional flag span (`you: 100% · zyBooks: 60%`, `conflicts with …`, `extended → 9/18`) ·
  optional merge span · optional amend badge.
- **`start_by` and slack are not optional.** `start_by` is the most decision-relevant field on the
  page — urgency is distance to *start*, not to *due*, which is the whole reason a six-hour project
  due in ten days outranks a twenty-minute quiz due tomorrow. The mockup had no slot for either;
  that was its single largest loss.
- **Must-do-first rendering is an invariant:** slack ≤ 0 renders unconditionally. Duration never
  decides visibility; deadline feasibility does.
- **Every count carries hours** (§12.6.1 Route C) — `Overdue 11 · 8.4h`, never a bare count. This
  is what stops a course hiding 35 hours behind 3 tasks.
- **Left out:** ordering by a blended pressure score. Ranking is soft (§12.2).

### 3.7 THE DAY (left rail)

- **Answers:** where does today's work actually go?
- **Computed by:** `surface::the_day → state.the_day`, via `WeekCalendar::events_on`/`free_blocks`
  and `ranking::designate_today_explained`.
- **Vertical, because the column is narrow** (§11.7): time above, block below, scheduled work
  nested under each open slot. An **all-day lane** sits at the top — all-day events are context and
  never subtract capacity.
- **The one genuinely new piece of engine logic this region needed, landed in the port:**
  `ranking::designate_today_explained` (`ranking.rs`) returns the block↔task mapping instead of
  throwing it away; `designate_today` (kept, `(task, hours)` only, still used where the mapping
  isn't needed) now delegates to it. `surface::the_day` reads the explained form via `Loaded.takes`.
- **Partial takes show both figures** — `2.0h (of 6.0h left)`.
- **Recurring commitments render as commitments, not calendar blocks.** They consume budget; a
  block does not. Rendering them identically would double-count.
- **Commitment blocks (`commitments/`, spec §6.2).** The gap walk runs over
  `WeekCalendar::template_only_blocks(day)` — the week template's classes alone, never the confirmed
  commitment notes — so a confirmed club is never drawn as a generic "class" gap. Each commitment
  span active that day (`spans_on(day)`) is then added as its own block: `kind: "class"` for a
  `class`/`lab` note, `"busy"` otherwise, `label` the note's own title. A span is **clamped to
  `window(day)`** (the planning day's window, §6.3, or the template's `day_start`/`day_end` with no
  planning-day note) and dropped entirely when wholly outside it. A `busy` block built from the
  day's Google events is dropped when its start and end exactly match a commitment block's
  **unclamped** span, so a class that is both a confirmed commitment and a Google event draws once,
  titled from the commitment. **Google events themselves are never clamped to the planning window —
  only commitment blocks are** (carried forward from the P17 review): an event straddling or
  outside the window still draws at its own time.
- **`moved` (spec §6.4).** When a confirmed `planning-day` note's window differs from the window the
  day started with (`state/plan.json`, written by `rank` the first time each day the window is
  read), the today view reports what changed: how many scheduled items moved to a later or earlier
  part of the day (morning/afternoon/evening) and how many no longer fit. It is computed **live** —
  every `surface` read re-designates the day under both windows and diffs the result by task slug —
  so an edit to the window shows its effect at once, not just at the next `rank`. `moved` is omitted
  from the JSON entirely (no key, not `null`) when nothing changed, when there is no confirmed
  planning-day note, or on any view but today. The today view prints `moved.text` under the
  headline (`#moved`, spec D7).

### 3.8 Left-rail nav

- **Answers:** where else should I look, and how much is waiting?
- **Counts are live and carry hours** where hours exist.
- **Left out:** the `.kb` keyboard-shortcut legend. Deleted, not hidden (§11.4).

### 3.9 DECISIONS (the deck)

- **Answers:** what needs a decision from me, one at a time?
- **A deck, not a list** (§11.8) — one card, up to two slivers behind, `N behind` in the foot.
  **Page height never changes with queue length.** With fifteen approvals pending, a list would push
  AHEAD, GOOD TO KNOW, CLOSED and RUNS off the page entirely.
- **The note field is part of the card and always visible; one click commits.** An action keeps its
  name through the whole flow — the two-step "Confirm" relabel was built and removed.
- **Ordered by `first_proposed_at`**, never `proposed_at`, which `defer_over_budget` rewrites.
- **Header carries** pending count · oldest age · budget spent/remaining · deferred count ·
  calendar-execution backlog.
- **Snooze is in the overflow.** A third always-visible button makes the primary action ambiguous.
- **Empty state says so** — `Queue clear — nothing waiting on you.` — rather than leaving a hole.
- **The deck acts now (Knowlu plan 1, Task 14).** `decideCard(id, verdict, snoozeUntil)` calls the
  `decide` command with the card's note-field text and, for `snoozed`, the date input's value; the
  card slides out and is marked busy (`card.dataset.busy`) for the round trip so a fast
  double-click or Approve-then-Reject before the first `invoke` resolves can never fire twice for
  the same id. `decide` writes through `knowlu_engine::write` via `console_ctx()` (`actor: "quinn"`,
  `via: "dashboard"`) and returns any `decision.warnings` onto the delta line. **The header's
  "clear" wording is R31's**, unchanged by this task: it reads `clear` only when `pending == 0`
  **and** `events_in_digest == 0` — the header-level twin of R19, so a digest with pending events
  but no approval cards still doesn't claim the queue is empty.
- **Fires `decision_made` for `approved`/`rejected`, `decision_deferred` for `snoozed`** (§4.6) —
  ids only, as soon as the `invoke` round trip resolves, **before** the envelope's `ok` is
  checked; a refused decide still logs the attempt.
- **`kind: event-check` — "Does this apply to you?"** `rank` files one for an event the judgment
  could only call `unsure` (at most 3 a day, inside the 15). The deck renders it like any other card.
  **Approve** writes a human answer `verdict:obligation` to `state/events-seen.md` for every uid on
  the card, so the event joins Coming up; **Reject** writes `verdict:drop`. Either line carries
  `by:` (the actor of the journal's human `status` set, else `unknown`) and the instance's own
  `jid:`, and it is written by `process_approvals` in the same `decide` call: the card leaves the
  deck on the click, and the event joins Coming up at the next `rank`. A settled answer carries to
  the series' later instances: the next `rank`
  gives each unanswered instance the same answer, and the series is never asked about again. A card
  that expires or is deleted unanswered is not an answer: it writes nothing, its own instances are
  never asked again, and the series' next instance may be.
- **`kind: commitment-check` — "Is this part of your week? Knowlu found it repeating on your
  calendar."** `rank` classifies repeating calendar events (`commitments.rs`) into proposals — a
  class, a lab, work, a club, a meeting, or the day's wake-to-bed window — and files up to 5 a day
  as cards, charged to the day's 15-approval budget (they count against the 15, not on top of it).
  The deck renders these like any other card. **Approve** writes a confirmed note in `commitments/`
  (or the `planning-day` note, for the window card); **Reject** writes an anonymous decline marker
  holding only the source's opaque key, and marks every twin of the same underlying series (a
  Google/ICS duplicate) declined too, so it is never re-asked from either side. A calendar change to
  an already-confirmed commitment (a new meeting time, a new room, a new end date) files the same
  card kind with a `target` note and a `change`/`was` pair instead of a fresh `commitment:` mapping;
  approving it amends the note, rejecting it leaves the note as it is — either way the question is
  never asked again. These cards, their series data (`state/calendar-series.json`) and the
  proposals themselves never leave the device (`commitments::LOCAL_CARD_KINDS`): only what the
  student confirms or declines becomes a note, and only a note syncs.
- **`kind: commitment-ask` — answered in the Decisions view.** A course with no class row gets a
  card asking when it meets. The deck shows **Answer…** for it (never Approve), which opens the
  Decisions view; there the card is a form — day toggles, start/end time pickers, *add another
  time*, **Save times** (→ `answer_card`, which writes `answer_meets` and approves the card in one
  call) and **No set times** (→ reject).

### 3.10 AHEAD

- **Answers:** where is the wall in the next fortnight?
- **Computed by:** `surface::ahead → state.ahead` — 14 buckets (`today ..= today+13`),
  `remaining_hours` by due date, **overdue work landing on today**, undated reported separately as
  `undated_hours`.
- **The takeaway sentence is generated**, not authored: *"Fri 11 Sep is the heaviest day in a
  fortnight; Sun 6th and Mon 7th are empty — that is where it goes."*
- **The peak label must be a real element.** The mockup hard-codes `8.7h` as a CSS `content:` string
  on `.ribbon .d.pk::after`, unconnected to the bar heights. Left as-is it diverges from the data
  silently and forever.
- **Left out:** the course-load panel it replaced. Quinn: *"I'm not sure what the course load
  section is for. It confused me."* It answered where the semester's weight sits — real, but not a
  question asked at 8am, and nothing about today changes because of it.

### 3.11 COMING UP

- **Answers:** what is happening soon that I already accepted?
- **Computed by:** `surface::coming_up → state.coming_up` — reads `state/events.md`'s roster and its
  verdict ledger (`eventroster::relevant_events`), filtered to what's still relevant, converted and
  trimmed to the same five picks `render::upcoming` makes for `today.md`'s events section, so the
  console and `today.md` never show a different five.
- Next five events, from the roster.
- **Distinct from the deck**, which holds *undecided* proposals. Accepted events and pending
  decisions are different information and must not share a region.

### 3.12 GOOD TO KNOW

- **Answers:** what should I know that is neither a task nor a decision?
- **Computed by:** `surface::good_to_know → state.good_to_know`, via `info::list_info` — open items
  only, with `close_key` lifecycle.
- **Now also carries heads-up items** (§12.5 G2): grade postings, late/missing flags. Quinn's split:
  *who is owed an email* is a recurring task, *registration windows* are tasks, and only *grade
  changes* and *late/missing* are info.
- **Left out:** a separate heads-up region. Merged here rather than given its own strip.

### 3.13 CLOSED THIS WEEK

- **Answers:** what got finished, and did I do it or did the system?
- **Computed by:** `surface::closed_this_week → state.closed_this_week`, from the journal —
  `progress → 100`, `status → done|archived`, or `op: delete`, last 7 days.
- **Marks:** `me` (actor `quinn`) · `agent` (actor `agent:*`) · `lapsed` (note carries
  `status: lapsed`).
- **Lapsed items appear deliberately.** A relevance filter eating wanted things must be visible.
- **This is the only completion surface that has ever existed.** Today a done task simply vanishes
  from `load_tasks` and nothing anywhere says what got finished.

### 3.14 RUNS

- **Answers:** is the machine healthy?
- **Computed by:** `surface::runs_panel → state.runs_panel`, via `runs::Runs::read` +
  `runs::expected_status`.
- **States are named in words**: seen · late · **missing** · **crashed**, with grace windows (cloud
  45 min, local 20). A missing run is currently *an absent line* — the exact failure CLAUDE.md
  warns about. The console says it out loud.
- **Full untruncated warning text.** `runner-log.md` truncates (`blount: not an ICS response
  (+1 more)`) and persists the rest nowhere — a health signal that hides half its own content.
- **RUNS is a real view now, not just the right-rail's compact strip (Knowlu plan 1, Task 15).**
  The aside's `renderRuns` (`#runs`) still shows the top few late/missing rows plus the last six
  recent runs, badge-counted by `runs_panel.warn_count`; the Runs nav entry now opens
  `renderRunsView` (`#runs-view`), which lists **every** `runs_panel.expected` slot and **every**
  `runs_panel.recent` run in the last 48h with its full `summary` — never truncated — before the
  warnings block and, last, `runs_panel.empty_text`.
- **The warnings block concatenates two sources, in order: `runs_panel.warnings` then the
  page-wide `state.warnings`** (Task 1's warnings channel) — a run-specific warning and a
  general one render in the same block rather than needing two.

### 3.15 SCHEDULE ("Your week")

- **Answers:** what is my week made of, and which hours do I plan in?
- **Reached by:** the nav link *Schedule* (view id `schedule`). It is a **page view**, not a
  read-model view: it polls the read model as `today` (`stateView()`), because `schedule` would be
  refused by `surface::View::parse` (R-P4a-4).
- **Computed by:** `your_week` → `commitments::overview` (in-process; no fetch, no write).
- **Each commitment is a row** with its kind select and level control, both written through
  `set_fields` (as `commitments::check_console_edit` allows). *Office hours* have **Add** (a
  one-row `commitments_confirm`, level optional). A course with
  no class row reads "Knowlu will ask when it meets".
- **The window editor** (*Your day*) previews an edit through `preview_window` after 400 ms: the
  preview's `moved.text`, or "No change to today's plan", and the first five items of the
  previewed day.
- **Left out:** a remove control (spec D8).

---

## 4. Cross-cutting behaviours

### 4.1 The ⚑ issue flag

On every row or card where `judged` is true — the payload's own per-row `judged` field (`Row.judged`
in `src/surface.rs`, from `surface::is_judged`: a `judgment` block that is not `{migrated: true}`,
or a journal record showing an `agent:*` actor created it or set one of its fields).

**Popover shape (Knowlu plan 1, Task 14, wired for real), `openFlag(anchorEl, targetId)`:** the
eight fixed `issues::CATEGORIES` (`wrong-effort`, `wrong-course`, `duplicate`, `should-not-exist`,
`wrong-tier`, `wrong-date`, `wrong-verdict`, `other`) render as toggle chips — multi-select, click
again to untoggle — **and** a free-text `<textarea>` below them, **both, always** (S1 review
amendment). `Flag` stays `disabled` until at least one chip is on; `Cancel` discards. A click
inside the popover stops propagation so it can never fall through to the row/card underneath and
open the drawer. Submit (`submitFlag`) calls `open_issue` with the toggled category list and the
textarea's text, which snapshots the object and its judgment so the issue still means something
after a re-judge; on success it fires `issue_opened` (id, `"task"`) and removes the popover — on a
refusal the popover stays open and shows the refusal text instead.

**Nothing acts on it.** No autonomous follow-up, ever. The list is cleared by a later Claude session
in one sitting — that is the entire point of the mechanism (§3 #9).

### 4.2 The "why" disclosure

Every computed value can explain itself from inputs the engine already has and currently discards:
`start_by` shows its arithmetic, a THE DAY allocation shows which block absorbed it and what was
left, the meter shows template capacity minus calendar.

Costs almost nothing and is the difference between a system that is trusted and one that is audited
by reading Python. It also gives an issue note something concrete to disagree with.

### 4.3 Click-to-edit

Click a field, it becomes an input; blur or Enter commits, Escape cancels. **Twelve fields are
editable** (`EDITABLE` in `app/static/console.js`, Knowlu plan 1, Task 13): `title`, `course`,
`due`, `effort_hours`, `importance`, `importance_reason`, `status`, `progress`, `slice_hours`,
`domain`, `rank_override`, `effort_confidence`. Each carries `data-field="<name>"` in the drawer's
`<dl>` markup — that attribute, not a hard-coded list at the click site, is what the handler reads
to know which field it is committing; a field the drawer renders without `data-field` (everything
outside `EDITABLE`) is inert to a click.

**Never editable:** `id`, `source_uid`, `also_uids`, `judgment`. Keys and provenance; an edited key
is a corrupted index.

**Progress reaching 100 also sets `status: done` in the same write** — one journal moment, one line
in CLOSED THIS WEEK, and the `took:` prompt (§4.5).

**Every edit makes the field human-set**, which freezes it against sources (S3.3) and turns the
routine's future opinion into a `kind: amend` card. The console is the primary input to the freeze;
the deck is where the system argues back.

### 4.4 Empty states

Never a hidden section (§12.1). Each is designed and tested: `Nothing active.` ·
`Nothing is at risk today.` · `No spare capacity today.` · `Queue clear` · no open info · no open
issues · **no runs recorded in 24h** · no events coming up · a task with no due date · a note that
will not parse · offline.

**Asserted in the Rust port** (console plan 1 Task 13, `src/surface.rs`'s
`an_empty_vault_names_every_empty_state` and `a_day_with_no_free_block_says_so_rather_than_hiding_the_lane`):
`verdict.headline == "Nothing active."`, `must_do.empty_text == "Nothing is at risk today."`,
`recommended.empty_text == "No spare capacity today."`,
`decisions.empty_text == "Queue clear — nothing waiting on you."`,
`issues_panel.empty_text == "No open issues."`,
`runs_panel.empty_text == "No run recorded in the last 24 hours."`,
`ahead.takeaway == "Nothing due in the next fortnight."` (no `empty_text` field of its own — the
takeaway sentence carries it), `the_day.empty_text == "No free block today."`, and the four bare-`Vec`
regions render the payload's own `texts.{coming_up,info,closed,offline}` rather than any string
`console.js` hardcodes (`the_page_renders_the_engines_empty_strings_not_its_own`,
`app/tests/static_assets.rs`).

### 4.5 `took:` — capturing `actual_hours`

When progress reaches 100, the row offers an inline **`took: [2.5h]`** pre-filled with the original
estimate. Right estimate → do nothing. Wrong → change one number. Dismissible.

Paired with inference: a task allocated to a free block whose progress completed during it takes the
block's hours, at low confidence. That is the only signal available when an *agent* ticks something.

Feeds a per-course × per-kind actual ÷ estimated ratio applied as a prior to new estimates.
**This loop is retained deliberately** even though the ranking learning loop was closed — bad
estimates are what make capacity lie, and capacity is what the meter, the gauge and `start_by` all
rest on.

### 4.6 Interaction events

**Landed in Rust as `src/uievents.rs` (Knowlu plan 1, Task 2), not the Python-era
`engine/ledger.py` line this section used to name.** Same shape, new home: one append-only
`JsonlLedger` — the same kind the journal uses, a different store — writing to
`state/events-ui/YYYY-MM-DD.jsonl` (UTC day taken from the record's own `ts`, `merge=union`).
The eleven `uievents::ACTIONS` are the whole vocabulary — an unrecognised action is refused, not
silently accepted: `view_opened`, `object_seen`, `edit_started`, `edit_committed`,
`edit_cancelled`, `decision_made`, `decision_deferred`, `issue_opened`, `sync_run`,
`delta_expanded`, `why_expanded`. Emitted fire-and-forget from `console.js`'s `ev(action,
objectId, objectKind, ms)` through the `ui_event` command; a failure here must never surface to
Quinn or block the write it rode in on.

**`object_seen` is the important one.** It turns rejection-by-inaction — the most common outcome in
the system and the least recorded — into data. A proposal that lapsed after being displayed eleven
times is a completely different signal from one that lapsed unseen. **It fires on a 2-second dwell,
not a mere paint:** one `IntersectionObserver` watches every `[data-id]` the page currently has
(rows, deck cards, the drawer's `<dl>`), and only an element still intersecting 2000ms after it
started does `ev("object_seen", id, dataKind, 2000)` fire — a row that scrolls past in a flick
never counts. `seenOnce`, keyed by id, caps it at one `object_seen` per id per page load even
across repaints that re-observe the same still-visible element; a repaint releases and re-attaches
the observer's targets rather than leaking them. `object_kind` always comes from the observed
element's own `data-kind` attribute, never guessed from the view.

**No free text, ever — enforced server-side, not just by convention.** `uievents::record` checks
`object_id` against `ids::is_id` and `object_kind`/`session`/`view` against a token pattern
(alphanumeric, `_-:`, ≤ 64 chars); anything else is a `FreeText` error the command surfaces, not a
silently-truncated title. So an export is shareable without redaction.

**The opt-in is inert.** `state/events-ui/` is still git-ignored by default, and
`config/planning.yaml`'s `commit_ui_events: true` still parses (`engine::uievents::commit_opt_in`
reads it), but nothing calls that function any more: the git-staging code it fed
(`history::run_sync`'s `git add -f`) left with git itself (C3′ Task 10). The flag is dead
configuration, not a lie the page tells — no test asserts a behaviour for it, and setting it in a
vault today changes nothing. **Never read by the engine's ranking path either way** —
`tests/uievents_isolation.rs` proves it — determinism is untouched. (Found during Task 12's close;
`commit_opt_in`'s removal is recorded in this report's production-needs list rather than done here,
since it is a code change to a file outside this task's docs scope.)

### 4.7 Sync, backup, the scheduler

The line under the topline (`renderSyncLine`, §3.1) states health in words the state itself
supplies — nothing here is computed client-side; the severity class (`calm` / `amber` / `crit`)
rides with the string. Read top to bottom, first match per group wins:

**Sync (`state.topline.sync`, an `engine::sync::SyncStatus`, checked in this order):**
- `last_error` set, else `skipped` set → the engine's word, mapped through the page's `SYNC_SAYS`
  (C3′'s final fix wave, R-C3′-exec-39/-43; `app/tests/static_assets.rs` reads every such word out of
  `engine/src/cloudmodel.rs` and `sync.rs` and fails if the table lacks one):
  - `no session` / `signed out` → `signed out — sign in to sync` (amber);
  - `no entitlement` → `can't confirm your subscription — changes stay on this computer` (amber; a
    paying student more than 72 h offline lands here too, so it never says "inactive");
  - `offline: the account could not be reached` → `offline — changes stay on this computer` (amber);
  - `no account`, `another sync is running` → `sync skipped — <word>` (calm, ordinary states);
  - any other word → its own first line (amber for an error, calm after `sync skipped — ` for a skip).
  *Sync now*'s refusal toast says the same words. The diagnostics blob and the issue report keep the
  engine's raw word.
- else `at` set (a sync has completed) → `in step with your account` (calm).
- otherwise (no sync has ever run) → `not synced yet` (calm).

There is no `conflicted`, `ahead`, `is_repo` or `has_remote` any more (C3′ Task 10, git left the
product): a conflicting write is a `kind: amend` card in the deck, not a merge state on the topline,
and there is nothing here to be "ahead" of.

**Backup (`state.topline.backup`):** `last_error`'s first line prefixed `backup: ` (amber), else
`behind_days >= 1` → **`backup N days behind`** (amber, singular/plural handled), else `last_ok`
set → `backed up` (calm). No backup status at all renders nothing — not a fourth empty-state
string, since backup is additive to the sync line rather than a region of its own.

**Standalone bits, each independent (append if true, several can show at once):**
- `startup_missed > 0` → **`N slots missed while quit`** (amber) — the scheduler's count of due
  slots that passed while the app wasn't running.
- `engine_newer` → **`engine newer than console`** (amber) — see §3.1's caveat on what this check
  actually proves today.
- `last_slot` present → **`last slot HH:MM ok`** (calm) or **`… failed`** (amber), timestamp from
  `last_slot.ended` falling back to `last_slot.started` — **omitted entirely when both are absent**
  (the "a slot is already running" placeholder summary carries neither), so the line never renders
  a hole where a time should be. When `last_slot.reason` is set the slot was **refused**, not run:
  **`last slot HH:MM refused — <reason>`** (amber), because the engine was never invoked and
  "failed" would say the wrong thing. `run_slot_inner` stores a refusal in `Scheduler.last` for
  exactly this.
- `scheduler.mode === "app"` → `scheduler on` (calm) or `scheduler paused` (amber); absent or
  `"script"` mode renders nothing here — the scheduler is inert and has nothing to report
  (Task 12: `scheduler: app` in `config/runners.yaml`'s `local` entry, plus this being the named
  device, is what flips it from inert to real).

**Every state that can be entirely absent — `sync`, `backup`, `scheduler`, `last_slot` — must
render correctly on that absence**, not just on presence: a fixture built before Task 10/12, or a
console session that hasn't polled once yet, carries none of them.

Two buttons always render beside the line regardless of state: **`sync now`** (`sync` command,
fires `sync_run` on completion) and **`back up now`** (`backup_now` command, no interaction event).

---

## 5. The design system

All 30 tokens live at the top of `console.css`, commented with what each is *for*.

- **Surfaces, four lifts:** `--canvas #0A0B0D` · `--s1c #101215` · `--s2c #15181B` · `--s3c #1A1D21`.
  Hairlines `--hair #22262B` / `--hair-2 #30353B`.
- **Text, four levels — hierarchy is carried by these, not by colour:** `--t1 #F1F3F4` ·
  `--t2 #C3C9CE` · `--t3 #878E95` · `--t4 #5A6169`.
- **One accent**, for interactive state and "this fits" only, never decorative: `--acc #3FB68B`
  (`--acc-dim #15302A`). Semantic colour only where it encodes data: `--crit #E0605A`
  (`--crit-dim #33191A`), `--warn #D9A441` (`--warn-dim #302512`).
- **Spacing, 4px base — every gap on the page comes from it:** 4 inside a control · 8 label to
  value · 12 row padding and column gap · 16 related rows · 24 heading to body · 32 blocks in a
  column · 48 between sections · 64 between zones. Radii 4 / 6 / 8.
- **Type:** Instrument Sans (UI) + JetBrains Mono (all figures, times, ids). Deliberately not
  Inter — Linear's own face and the flagged "safe" default. **Both vendored**; the page makes no
  network request.
- **Widths:** `--measure 900px` (main), `--shell 1560px`. Past the shell the page gains **gutters,
  not width** — without the cap, `1fr` stretched and left an ~800px void between a task title and
  its hours at 1920.
- **Single-theme dark**, `color-scheme: dark` unconditionally, every colour from a token so the
  page never borrows the host background.

**Breakpoints:** 1400 (rails narrow) · 1180 (right rail becomes a full-width auto-fit grid, each
`.rblock` one grid item so headings never detach from their content) · 820 (single column, nav
wraps, titles wrap instead of truncating, progress column drops).

**1280 and 1440 logical px verified on the real exe, 2026-09-03** (console plan 1, Task 14 fix 1,
R33(a)/R35): `knowlu.exe` resized programmatically (`SetWindowPos`, no synthetic
input) to a 1280×860 and a 1440×900 logical client area on the live 200%-scaled display and
captured by `PrintWindow`-by-handle — both show the full desktop 3-column layout (left rail, main,
right rail) with no horizontal overflow and no clipped right rail. The 1560px-grid-vs-1440px-display
question from Task 14 is closed; no CSS change was needed.

**The three plan-4a panels, looked at rather than reasoned about (2026-09-06, headless Chromium;
`scripts/console-shots.py` shot all three at every viewport — since the final fix wave
`window.KNOWLU_SHOTS` exports `openSettings` too, so `<w>-settings.png` is shot beside the wizard
and the picker — and `scripts/settings-check.py` shot the settings panel again at 1512×945, which
is the one with its rows filled in, because `console-shots.py` runs with no backend behind them;
all eight viewports ≥ 820 reported `ok` across Today and the three plan-2 views):**

- **Wizard** (`1280-wizard.png`) — a single centred column about 600px wide on the bare canvas: the
  **Knowlu** wordmark top-left with `step 1 of 7` right-aligned against it, the *Welcome* heading,
  the privacy paragraph in `--t1` above the "what should I work on today" line in `--t3`, and
  *Back* / *Next* bottom-right of the column. There is no rail, no topline and no nav — this window
  has no vault to render one from — and *Back* is present but does nothing on panel 1.
- **Picker** (`1280-picker.png`) — the same centred column: **Knowlu**, the question *Which vault?*,
  then one profile row (`Ada` over its vault path in JetBrains Mono `--t3`) with a filled *Open*
  button at the right edge and a hairline under it, and below that the two buttons right-aligned —
  *Use an existing vault…* outlined, *Create a new vault…* filled. The wording is R-P4a-22's, not
  the spec's *Add another…*.
- **Settings** (`settings-1512.png`) — an overlay panel pinned to the top-right **over a fully
  rendered Today**, one lift above it and covering the right rail while it is open: `SETTINGS` with
  *Close*, then the six rows as a label/value/control grid — *Profile name* (text field + *Save*),
  *Vault* (path + *Copy*, with *Switch profile…* beneath), *Backup folder* (path + *Choose…*, with
  *Back up now* beneath), *Start with Windows* (checkbox), *Updates* (`version 0.1.0 — updater not
  configured, checked 2026-09-05 12:00` + *Check now*) and *Diagnostics* (*Copy diagnostics*). The
  Updates row states what the commands said and **names no channel** (M8) — there is only one.

---

## 6. Deliberately left out — do not re-propose

| Cut | Why |
|---|---|
| Keyboard shortcuts, `⌘K`, the legend block | Quinn, 2026-08-31 |
| The course-load / course-heat panel | *"I'm not sure what it's for. It confused me."* |
| An alert for "many hours behind few tasks" | Treated a symptom, unactionable. Replaced by hours-on-every-count + split proposals |
| Event digests | One approval per event (§3 #7) |
| Toasts and the daily email brief | Quinn, 2026-08-31: neither |
| Launch at login | A window that opens itself is a notification |
| The ranking learning loop, the daily calibration question | Closed by soft ranking (§12.2) |
| A separate heads-up region | Merged into Info |
| `Overdue`/`This week`/`Later` as pages | Filters inside Work |
| Two-step "Confirm" on the deck | An action keeps its name through the flow |
| The `- [ ]` checkboxes from `today.md` | They looked like the primary control and did nothing |
| A card-and-shadow dashboard; a mono terminal look | Both built and rejected (§11.5) |

---

## 7. How to change this page safely

### 7.1 Parity table

Not a §6 entry — nothing here was cut. This is the map from "a region on the page" to "the
payload key it reads, the command(s) that change it, and the interaction event(s) it fires" for
every write path landed in Knowlu plan 1 (Tasks 8–15). Use it the way §3's `Computed by:` bullets
are used for reads: before touching a region's write path, find its row.

| Region | Payload key | Command(s) | Events |
|---|---|---|---|
| The deck (§3.9) | `decisions` | `decide` | `decision_made` (approved/rejected), `decision_deferred` (snoozed) |
| The ⚑ flag (§4.1) | `judged` (per-row field, not a dedicated top-level key) | `open_issue` | `issue_opened` |
| The twelve editable fields (§4.3) | `note.frontmatter[field]` (from the `note` command; `data-field` names which) | `set_fields` | `edit_started` on click; `edit_cancelled` on Escape, an unchanged value, or a client-side validation refusal (empty/out-of-range number) — all without reaching `set_fields`; `edit_committed` only once `set_fields` returns `ok: true` (a server-side refusal fires neither) |
| The + task row | none — the new note appears wherever ranking next places it (`must_do`/`recommended`/…) | `create_task` | none |
| Delete (drawer) | none — the note leaves every region it was in | `delete_note` | none |
| Info close (GOOD TO KNOW) | `good_to_know` | `close_info` | none |
| The sync line (§4.7) | `topline.{sync,backup,auto_sync,startup_missed,engine_newer,last_slot,scheduler}` | `sync`, `backup_now` | `sync_run` (fires when `sync` completes; `backup_now` fires none) |
| The Runs view (§3.14) | `runs_panel` | — (read-only) | none |
| The wizard (§2) | none — the vault does not exist yet; `launch_state` supplies the timezone, the Documents path and the campus list | `create_vault`, `adopt_vault`, `restore_vault`, `apply_profile_settings`, `store_credentials`, `retarget_credentials`, `pick_folder`, `finish_onboarding` | none |
| The picker (§2) | none — `launch_state`'s profile list | `adopt_vault`, `open_profile`, `pick_folder` | none |
| The settings overlay (§2) | none — `settings_context` (vault path, version, profile name) plus `get_settings` | `set_settings`, `set_profile_name`, `pick_folder`, `copy_text`, `copy_diagnostics`, `backup_now`, `switch_profile` | none (`backup_now` fires none here for the same reason it fires none on the sync line) |
| The update offer (§2) | none — `check_for_updates`'s envelope (`version`, `staged`, `last_check`, `last_error`) | `check_for_updates`, `install_update` | none |

**Three rows fire no interaction event at all** — `create_task`, `delete_note`, `close_info` — and
`backup_now` fires none either (it shares the sync line's row with `sync`, which does) — because Task 15 instrumented the paths that needed rejection-by-inaction data
(the deck, edits, flags) and the ones with an obvious success/failure UI already (a row appearing,
a drawer closing), not every mutating command uniformly. If that gap is ever closed, add the
action name to `uievents::ACTIONS` first — an unlisted action is refused, not silently logged.

**Nor do the four plan-4a rows**, and that is deliberate (F19/R7): plan 4a adds **no** action to
`uievents::ACTIONS`. The wizard and the picker run before any vault exists to log into, and the
settings overlay and the update offer are configuration, not the judgement the event ledger is for.

### 7.2 The five steps

1. **Read the region's entry above**, especially its "left out" line.
2. **Change tokens, not values.** A colour or gap written inline is a bug.
3. **Run the screenshots.** `scripts/mockup-shots.py` at eight viewports. Three bugs in the design
   session were invisible in source and obvious on screen — an ~800px void, a breakpoint that
   detached every rail heading from its content, and a class collision that deleted every run-status
   label. **Then actually read the screenshots.**
4. **Check `tests/test_parity.py`.** If you removed a region, it will tell you what that region
   carried.
5. **Update this file in the same commit.** A region whose intent is undocumented is a region the
   next session deletes.
