# What is on the app — sorted by where it came from

**Date:** 2026-08-31. **Purpose:** Quinn asked, in his words, to be told *everything* that will be
on the console, split by what is being ported from the Obsidian dashboard, what the mockup added,
and what he asked for in conversation that never made it into a mockup. This file is the answer,
and it is the scope check for S2 — if something is not in one of these tables, it is not being
built.

Sources audited: `engine/render.py` in full, all 30 engine modules, every CLI, `.obsidian/`
(config, plugins, live workspace), `views/*.base`, every Meta Bind block in the vault,
`docs/superpowers/notes/2026-08-26-today-page-redesign-brainstorm.md`,
`docs/superpowers/notes/2026-08-28-redesign-program.md`,
`docs/superpowers/specs/2026-08-11-personal-ops-system-design.md` §5/§6/§9, `VISION.md`, and the
nine pre-redesign specs.

---

## 1. Ported from the Obsidian dashboard

The real dashboard today is smaller than it feels: **two tabs** — `state/today.md` as the
Homepage (opened on launch, replacing all open notes) and `views/today.base` — plus
`views/approvals.base` pinned when wanted, three Meta Bind buttons, and Obsidian's Git status bar.

### 1.1 From `state/today.md` (everything `render.py` can emit)

| # | On the page today | In the app |
|---|---|---|
| 1 | `# Today — Sunday, August 30` | topline |
| 2 | `**Runway: -6 days (behind)**` + the four-word vocabulary (behind/tight/steady/ahead) | topline figure beside the prose verdict |
| 3 | `Capacity today: 10.0h (template 10.0h − 5.0h calendar)` | meter subtitle, computed values |
| 4 | `137 active` | topline |
| 5 | `**Approvals: 13 pending** (oldest 3d)` | deck header |
| 6 | `N approved awaiting calendar pass` | deck header |
| 7 | `**Events: 8 in today's digest**` | deck header (retires itself when S3 kills digests) |
| 8 | `## Schedule` — timed events | THE DAY |
| 9 | `- (all day) …` — and all-day events never subtract capacity | THE DAY's all-day lane |
| 10 | `## Must do` — title, due, hours left | MUST DO rows |
| 11 | `⚠ conflicts with <x>` | `.fl` span on the row |
| 12 | `⚠ Must-do work (26.4h) exceeds today's capacity (10.0h)…` | kept **as the sentence with both figures**, under the meter |
| 13 | PENDING AMENDMENT blocks — age, `8/26 → 8/24 (2 days EARLIER)`, the `stale:` marker, the proposal pointer | amend cards in the deck + a badge on the task row |
| 14 | `- Nothing is at risk today.` | designed empty state |
| 15 | `## Recommended` — designated takes | THE DAY's nested rows + a named list |
| 16 | `— 2.0h (of 6.0h left)` partial takes | both figures on the row |
| 17 | `— {hours}h · recurring` commitments | rendered as commitments, not calendar blocks — they consume budget, a block does not |
| 18 | `- No spare capacity today.` | designed empty state |
| 19 | `## Everything active` — **137 numbered rows** with `start by 8/25`, `OVERDUE START` / `14d slack`, hours left | the **All active** view; start-by and slack return to every row |
| 20 | `## Coming up` — next 5 events | COMING UP rail block |
| 21 | `*Generated 2026-08-30 23:55 by local*` | topline, **runner name kept** — it is the diagnostic that separates "cloud is dead, local still refreshing" from health |
| 22 | `Nothing active.` + that branch's whole distinct layout | designed view state |

### 1.2 From `views/today.base`

A flat table over `tasks/` where `status: active` — Title, Due, Effort (h), Importance, Progress,
sorted by due. Cells are editable in place, and this is **the only bulk-edit surface that
exists**. Bases cannot express ranking, slack, must-do partition or capacity.

→ The **All active** view, plus click-to-edit on every field. It gains rank position, start-by and
slack, which Bases could never show.

### 1.3 From `views/approvals.base`

A card grid over `approvals/` — Title, Kind, Status, Proposed, Expires, oldest first. Filters on
`type == approval` only, **not** on `status: pending`, so settled notes linger until a run
archives them.

→ The **decisions deck** (one at a time) and the **Decisions view** (the full list).

### 1.4 The interactive parts — all eight affordances that exist today

| # | Today | In the app |
|---|---|---|
| 23 | **Approve** — Meta Bind button, writes `status: approved`, executes on the next runner pass | deck primary action, same semantics |
| 24 | **Reject** — Meta Bind button | deck secondary action |
| 25 | **Snooze 2 days** — **half-broken.** `enableJs: false` in Meta Bind's config, so it writes `status: snoozed` but `snooze_until` silently fails, producing the `snoozed without snooze_until` WARN. Also absent from digest notes | card overflow action, **fixed** — the date is computed server-side, no JS setting involved |
| 26 | **Events-digest checkboxes** — tick events, then Approve; unticked are declined. Two-step, real semantics | S3 retires digests; each event becomes its own card |
| 27 | **Frontmatter / Bases cell editing** — how `progress` actually gets recorded today | click-to-edit on every field |
| 28 | **Note body editing** — tasks, courses, profile | **done (M2):** body editing in the detail drawer for tasks and courses (`set_body`, a hash-only journal record, conflict keeps the draft, *Saved · Undo* for 10 seconds); the profile in Settings (*Your preferences*, *Campus events*) |
| 29 | **Ticking `- [ ]` in `today.md`** — **inert.** Nothing reads today.md back; the file is rewritten every run and the tick is discarded | **replaced** by a progress control that writes for real (below) |
| 30 | Obsidian **Git status bar / branch indicator** — the ambient sync signal | topline synced stamp + pending-push count |

### 1.5 Other reading surfaces being carried in

| # | Today | In the app |
|---|---|---|
| 31 | `state/runner-log.md` — the health page | RUNS panel + view, with the **full untruncated** text the log currently cuts |
| 32 | `state/events.md` — the relevant roster and the dropped-event audit trail | COMING UP; the audit section stays a linked file (§6.3) |
| 33 | `state/calendar.md` — snapshot/fallback | data, not a page; already reaches him through THE DAY |
| 34 | Approval notes' `**Why proposed:**` paragraph and the ` ```task ` preview fence | the card body |
| 35 | `courses/*.md` — 7 syllabus digests, grade weights, `**verify**` flags | linked from the detail drawer; **nothing parses them** until S5 |

### 1.6 Deliberately NOT ported

- **The `- [ ]` checkboxes in `today.md`.** They look like the primary control and do nothing.
  The 2026-08-21 request — a fully-ticked box marks the task complete — never shipped. The app
  replaces them with one progress control per task (Quinn, 08-26: *"Progress only; done is implied
  at 100. One uniform control per task, no separate checkbox."*).
- **`Untitled.base`** — a stray empty file from an accidental "create new base". Delete it.
- **Templater** — installed, enabled, entirely unconfigured, no `templates/` folder. Dead weight.
- **`today.md` as the working surface.** It keeps being generated by the runners and stays useful
  as a record and a phone-readable fallback, but it stops being where work happens.

---

## 2. Added by the mockup

Invented in the 2026-08-30/31 design session, agreed as `2026-08-31-s2-console-CHOSEN.html`.

| # | Element | Note |
|---|---|---|
| 36 | **The segmented capacity meter** — one block = 30 min, blocks that fit inside a bordered container, blocks that do not drawn outside in outline red | the signature element; today 10 in, 39 out |
| 37 | **THE DAY** — a vertical time-ordered plan in the left rail, scheduled work nested under each open slot | this is `designate_today`'s block↔task mapping, which the engine computes at `ranking.py:65-103` **and throws away**. Surfacing it is real new engine work |
| 38 | **The decisions deck** — one card, two slivers behind, `N behind`; page height never changes however long the queue | answers the objection that killed per-event proposals in the first place (15 notes × 3 clicks was "a chore that gets abandoned") |
| 39 | **Note field on the card, always visible, one click commits** | |
| 40 | **AHEAD** — 13-day hours-due ribbon with a generated takeaway | replaced the course-load panel Quinn cut |
| 41 | **GOOD TO KNOW** — open info items | |
| 42 | **CLOSED THIS WEEK** — with me/agent marks | |
| 43 | **RUNS** — recent run lines with status chips | |
| 44 | **Horizon grouping** in Must do (Overdue to start / Due Monday / Rest of the week) | the 08-26 ruling: group by time horizon |
| 45 | **Left-rail nav with live counts** | `href="#"` in the mockup; real views in the app |
| 46 | **Verdict `h1` + lede** — two sentences of prose on what the day is about | the parent design asked for exactly this in 2026-08 |
| 47 | **Per-row progress track** | |
| 48 | **Source tag chips** + flag/merge spans | where S3's markers land |
| 49 | **The dark console design system** — 4 surface lifts, 4 text levels, one accent, a 4px scale | |
| 50 | **Sync stamp + pending-push count** in the topline | |

**Two mockup defects being fixed rather than copied:** the peak-day label `8.7h` is a hardcoded
CSS `content:` string disconnected from the bars, and both typefaces load from Google Fonts (the
window must work offline). The `.kb` keyboard legend is deleted per §11.4.

---

## 3. Asked for in notes, never in a mockup

### 3.1 In S2 because this spec put them there

| # | Want | Source |
|---|---|---|
| 51 | **⚑ issue flags on every AI-judged object** — category chips **and** free text, both always; nothing acts on it autonomously | program §3 #9, §11.11 |
| 52 | **The delta line** — "since 08:04 — 2 ticked by agent · 3 new approvals · CS 100 P2 date moved" | §11.11 |
| 53 | **Full click-CRUD** — create with a visible New task button, edit every property inline, delete | §11.11, rubric #2 |
| 54 | **A tick that actually writes** — progress only, done implied at 100 | Quinn, 08-26 #2 |
| 55 | **Agent completions visible with a me/agent mark** | program §3 #10 |
| 56 | **Info space with an open/close lifecycle** ("package is here" opens, "package received" closes) | program §3 #11 |
| 57 | **A note field at approve/reject time** | program §3 #8 |
| 58 | **Run health at a glance** — including *crashed*, *missing* and *late* as distinct states with grace windows | rubric #9, S1 §2.6 |
| 59 | **Per-note journal history** — every write, who, when, which device and run | S1's journal, never surfaced |
| 60 | **Provenance visible** — the `judgment` block, `effort_source`, `effort_confidence`, `importance_reason` | S1 §2.2; today a hand-tuned, a vendor and a guessed number look identical |
| 61 | **Sync reconciliation made visible** — `supersede` records rendered as "laptop's 60% won over the agent's 100%" | S1 §4.2; `reconcile.py` has **zero callers** today |
| 62 | **Obsidian edits surfaced** — `detect_external` journals them and nothing shows them | S1 §4.3 |
| 63 | **The approvals budget** — 15/day, spend, remaining, deferrals; currently not queryable anywhere | CLAUDE.md, budget spec |
| 64 | **Amendments of every urgency** — neutral and decreasing ones are counted in "N pending" and never shown | approvals spec |
| 65 | **Unbind a wrong duplicate bind**, and **resolve a duplicate issue** side-by-side | S3 §3.5b, §3.6 |
| 66 | **Coursework ingest run records** — it writes none, and its per-item detail goes to stdout that Task Scheduler discards | Quinn, 08-31 — folded into S2 |
| 67 | **Unreadable notes rendered in place**, never a silently shorter list | S1 follow-ups |

### 3.2 Asked for, and NOT in S2 — the honest gaps

These are real requests with no home in the current spec. Each needs a ruling.

| # | Want | Source | Where it stands |
|---|---|---|---|
| **G1** | **The aggregate-slack gauge.** *"Aggregate slack across all active tasks is the gauge displayed at the top of Today. Slack trending down across a week means ground is being lost even when nothing is late."* | design `:157-159`; **VISION.md success criterion 4** — "slack trending flat or up" | **RESOLVED 2026-08-31 — Quinn: "I love the slack gauge. Definitely keep it." Now S2 §7.7.** The capacity meter is a different instrument: it answers *does today fit*, not *am I gaining or losing ground over weeks*. Program §3 #3 said "the slack meter, nothing else" and the meter that got built measures capacity. A trend needs history the journal can now provide |
| **G2** | **Heads-up items** — grade changes, late/missing flags, registration windows, **who is owed an email** | design `:216` | **Not in S2.** The pieces are spread across S5 (grades/late), S3 (events), S6 (owed an email); no region collects them |
| **G3** | **Pull-ahead view** — *"When today's designated work is complete, Today reveals a ranked pull-ahead view of tomorrow's and this week's slices. Completing one raises aggregate slack and moves the gauge."* | design `:202-204` | **Not in S2.** Deferred continuously since 2026-08-13 |
| **G4** | **The rank-correction learning loop** — override a rank with a reason, written to `preferences.md`, loaded every run | design `:206-210`; **VISION.md criterion 2** | **CLOSED 2026-08-31 by the soft-ranking ruling (program §12.2)** — a soft ranking needs little correction machinery. Kept only as an editable field. `rank_override` is read by `ranking.rank`, written by nothing, set on zero notes, not amendable, and nothing learns. Cut three separate times before this |
| **G5** | **`actual_hours` recalibration** | engine backlog since 08-13 | **Nowhere.** Estimates never learn from reality |
| **G6** | **Notifications** — BurntToast desktop toasts for time-critical-and-at-the-desk, and one 8am email brief, hard-capped | design §9 `:269-279` | **Entirely absent from S2**, which even declines launch-at-login on the grounds that a self-opening window is a notification |
| **G7** | **The course-hours alert** — PH 106 hides **35.0h behind only 3 tasks**; a count-only view points at the wrong course | 08-26 brainstorm finding #2; §11.9 preserved it as "an occasional alert" | **Open question**, no home yet |
| **G8** | **A Week view** — seven days, capacity per day, commitment progress | design `:222` | **Partial.** AHEAD shows hours *due* per day for a fortnight; it does not show capacity per day or commitment progress |
| **G9** | **`profile/preferences.md` editing** — a file that changes ranking behaviour, invites editing ("this is how a wrong lesson gets unlearned"), and has no UI | `profile/preferences.md` | **Not in S2** |
| **G10** | **Grades / GPA** — letter grade per course, weights, late and missing | design `:224`; program §3 #12 | **S5 by design.** S2 §17 says explicitly it has no region in this layout; decide its home in S5 rather than wedging it in |
| **G11** | **A "reason" on each change** — the design asked for *"every add, removal, re-rank, and re-estimate since last viewed, each with a one-line reason"* | design `:216` | **Partial.** The delta line shows old → new, actor and run id. It does not show *why* |
| **G12** | **One pairwise calibration question per day** — *"When genuinely uncertain, the system asks **one** pairwise calibration question per day in the brief — 'is X more important than Y?' — never more."* | design `:188` | **CLOSED 2026-08-31 with G4** — same reasoning. Never built, and the brief it lived in no longer exists. It is the other half of the correction loop (G4): G4 captures a correction Quinn initiates, G12 solicits one when the system knows it is guessing |
| **G13** | **Say plainly when the week went wrong** — *"if a weekend is being consumed to make a Monday deadline, that is a signal the week went wrong, and Today should say so rather than quietly absorbing it"*; and *"a slipped Monday is expensive; the engine should say so explicitly rather than silently reflowing"* | design `:163-184` | **Not built.** Capacity fitting reflows silently today. This is a *narrative* signal, closest in spirit to the over-capacity sentence S2 does keep |
| **G14** | **Cadence progress on recurring commitments** — *"weekly cadence items carry their own progress-against-cadence and grow more urgent as the period closes"* | design `:200` | **Partial.** Recurring commitments render and consume budget; nothing shows progress against the cadence or escalates as the period closes |

### 3.3 Explicitly cut — do not re-propose

- **Course-load panel** — *"I'm not sure what the course load section is for. It confused me."* (§11.9)
- **Keyboard shortcuts** — `J`/`K`, `A`, `⌘K` and the legend block (§11.4)
- **Event digests** — one approval per event instead (§3 #7); the digest's own justification is now
  answered by the deck
- **A mail digest section on the page** (google-ingest spec)
- **Calendar entries becoming tasks** — never
- **Amending anything outside `tasks/` and `courses/`** — "it turns an unattended twice-daily job
  into a general-purpose vault editor"
- **Two-step Confirm on the deck** — an action keeps its name through the whole flow
- **The weather line** (design `:214`) — never built, never re-requested
- **Electron / Tauri / a browser-only PWA** (§11.1)

---

### 3.4 One reversal worth noticing

The 08-26 mockup's rule was *"a section that has nothing to say emits no heading at all."* S2 §9.6
**reverses it** — every empty state is designed and named (`Nothing is at risk today.`, queue
clear, no runs in 24h). The reasoning: a console whose sections vanish makes absence ambiguous,
and VISION.md's standing rule is that silence never is. Flagging it because it is a deliberate
contradiction of an earlier decision, not an oversight.

---

## 4. The three things most likely to surprise

1. **The `today.md` checkboxes are theatre.** They are the most control-like thing on the current
   page and nothing reads them back. Anyone who thinks ticking already works will read the new
   progress control as a small change; it is not — it is the first time a tick means anything.
2. **The mockup is a subset, not a superset.** Ported literally it deletes the entire 137-row
   ranked list, per-task start-by and slack, Coming up, the runway figure, amendments, and every
   empty state. §1 of this file is what stops that.
3. **Nothing in the vault links to anything.** There are no `[[wikilinks]]` in `today.md` at all —
   no click-through from a task line to its note, or from "13 pending" to the approvals view.
   Every navigation today is quick-switcher or file explorer. The app makes navigation exist for
   the first time, which is a bigger behavioural change than any single feature.

## 5. The asymmetry worth a ruling

The system is about to have **two** learning loops, and only one of them is specced.

- **Event relevance learns.** S3.7: the profile is the cold start, and the journal's approve/reject
  records with Quinn's notes tune it — *"what turns 'wasn't my vibe' on Big Al's Mystery Night
  into a rule without Quinn having to state one."*
- **Ranking does not.** G4 and G12 above. `rank_override` is read by the ranker, written by
  nothing, set on zero notes, not amendable. `profile/preferences.md` exists and invites editing,
  the cloud routine is still instructed to honour it, and nothing writes to it. This has been cut
  three separate times (approvals, announcement and events specs) and S2 defers it a fourth.

Both loops serve the same purpose — correcting judgment without a code change. VISION.md's
criterion 2 is the ranking one: *"The ranking is agreed with, or correctable in under 30 seconds
when it isn't."* Correctable it is; learned-from it is not.
