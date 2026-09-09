# Personal Operations System — Design

Date: 2026-08-11
Status: Design approved pending one open decision (see Open Decisions)
Repo path when committed: `docs/superpowers/specs/2026-08-11-personal-ops-system-design.md`

---

## 1. Purpose

A system that answers one question every morning: **what should I work on today, and in what order?**

It answers it by knowing what is due, how long things take, how much time exists, and what recurring commitments are falling behind. It ingests information autonomously and proposes actions for approval.

**v1 scope: school only.** Tutoring, SetNForget, content, and opportunity scanning are deferred but designed around — see Section 11.

### Success criteria

The system succeeds if, after four weeks:

1. Quinn opens the Today tab first thing and works from it without maintaining a parallel mental list.
2. The ranking is agreed with, or correctable in under 30 seconds when it isn't.
3. Nothing has been missed that the system had the information to catch.
4. Work is being completed before due dates rather than at them, visible as slack trending flat or up.

### Failure modes to design against

- **Trust decay.** One bad ranking with no correction path and the list gets ignored.
- **Notification habituation.** Too many pings and all pings stop working.
- **Capture friction.** If adding a task is slow, a second list forms in Quinn's head and the system loses authority.
- **Silent changes.** If the system removes or reorders something without saying so, it becomes unpredictable.

---

## 2. Architecture

### Components

**Vault repo** — a private GitHub repo under the account tied to `quinn.hall.scho@gmail.com`. All state is markdown with YAML frontmatter. Single source of truth. Cloned by the cloud runner, opened locally by Obsidian and Claude Code.

**Cloud runner** — Claude Code Routines, running on Anthropic infrastructure. Fires regardless of machine state. Minimum interval one hour, no automatic retry on failure. Reads and writes the vault repo, uses account-level connectors (Gmail, Google Calendar). Requires **Allow unrestricted branch pushes** enabled on this repo so it can commit to `main` directly. No branch protection rules on `main`.

**Local runner** — Claude Code Desktop scheduled tasks on Windows. Chosen over Cowork because tasks are stored on disk as `SKILL.md` files under `~/.claude/scheduled-tasks/`, which makes every automation editable by Claude Code — a core requirement. Runs only while the Desktop app is open and the machine is awake. On wake, performs one catch-up run for the most recently missed schedule and discards older misses. Owns all browser work.

**Obsidian** — the read/write interface, opened on desktop and laptop against the same repo. Not a component so much as a window.

### Why the split

The cloud runner is reliable but has no local file or browser access. The local runner has both but only when the desktop is on. Blackboard and any other SSO-gated source can only be reached by the local runner. Everything else goes to the cloud so it survives the desktop being off.

### Sync

- Obsidian Git plugin on both machines: auto commit-and-sync at 5 minutes, "auto commit-and-sync after stopping file edits" on, pull on startup on, auto-pull interval 5 minutes.
- Local scheduled tasks run `git pull --rebase` before starting and push after finishing.
- **Editing rule enforced in every runner prompt:** never rewrite a file wholesale. Append or edit specific entries only. A conflict should cost a line, not a day.

### Obsidian plugin stack

| Plugin | Role |
|---|---|
| Bases (core) | Dashboard views over task frontmatter. `.base` files are plain text and editable by Claude Code. |
| Meta Bind | Approve/Reject/Snooze buttons and inline property editing on the Approvals tab. |
| Obsidian Git | Sync. |
| Templater | Task and note scaffolding, quick capture hotkey. |
| Calendar + Periodic Notes | Daily brief notes. |
| Homepage | Opens the Today tab on launch. |

Dataview is deliberately excluded — Bases covers the need natively and produces editable output.

---

## 3. Repo layout

```
quinn-ops/
  CLAUDE.md                 system behavior rules; edit to change how it thinks
  VISION.md                 full end-state, checked against every v1 decision
  profile/
    me.md                   programs, priorities, standing commitments
    people.md               advisors, professors, directors + contact cadence
    preferences.md          learned ranking corrections, with reasons
  courses/
    cs-100.md               syllabus extract, grade weights, current grade
    gn-103.md
    ph-106.md
    bui-100.md
    bui-101.md
    engr-101.md
    ua-101.md
  tasks/
    <slug>.md               one note per task
  archive/                  completed and removed tasks, never deleted
  approvals/
    <slug>.md               one note per pending proposal
  intake/
    inbox.md                raw quick-capture lines awaiting parsing
    documents/              dropped PDFs: schedules, syllabi, finals
  briefs/
    2026-08-11.md           dated snapshots
  views/
    today.base
    approvals.base
    week.base
    grades.base
  rituals/
    semester-rollover.md
    session-health.md
```

---

## 4. Data model

### Task note frontmatter

```yaml
title: PH 106 problem set 4
domain: school
course: ph-106
due: 2026-09-18T23:59
effort_hours: 2.5
effort_confidence: low        # low | medium | high
effort_source: inferred       # inferred | stated | measured
importance: 4                 # 1-5
importance_reason: "20% of final grade, exam prep"
start_by: 2026-09-15          # computed: due - effort - buffer
slack_days: 3                 # computed
status: active                # active | done | archived
progress: 0                   # 0-100
slices:                       # present when effort_hours > 2
  - {label: "problems 1-6", hours: 1.25, done: false}
  - {label: "problems 7-12", hours: 1.25, done: false}
rank_override: null
override_reason: null
created_by: blackboard        # blackboard | email | calendar | manual | claude
created_at: 2026-09-02
completed_at: null
actual_hours: null
```

### Other note types

- **Course notes** hold grade weights extracted from the syllabus, current letter grade, and anything flagged late or missing.
- **People notes** hold role, last contact date, desired contact cadence, and context.
- **`preferences.md`** accumulates ranking corrections as human-readable statements with reasons. Loaded on every run. Editable and deletable by hand — this is how a wrong lesson gets unlearned.

---

## 5. Ranking engine

### Start-by, not due

For each task: `start_by = due − effort_hours (converted to working days) − buffer`

Urgency is measured as distance to `start_by`, not to `due`. This is what makes the system push toward being ahead rather than merely on time. A six-hour project due in ten days goes amber before a twenty-minute quiz due tomorrow.

### Slack

`slack_days` = working days of breathing room remaining on a task. Aggregate slack across all active tasks is the **gauge** displayed at the top of Today. Slack trending down across a week means ground is being lost even when nothing is late.

### Capacity — schedule-aware

Free time is not fungible. The Fall 2026 schedule produces radically uneven days:

| Day | Usable hours | Shape |
|---|---|---|
| Mon | ~8.2 | Two long blocks (8:00–11:00, 12:50 onward) |
| Tue | ~3.9 | Three fragments, none over ~75 min, classes 9:30–6:15 |
| Wed | ~4.7 | Four fragments |
| Thu | ~5.9 | Two fragments plus a long afternoon from 2:45 |
| Fri | ~8.25 | Two long blocks |
| Sat | ~10 | Unbroken |
| Sun | ~10 | Unbroken |

Usable = contiguous blocks of 45 minutes or more between 8:00 and 6pm, excluding class. The morning workout runs 5–6am to roughly 7:30, and work begins at 8:00. Evenings after 6pm are available as reserve capacity but are not counted toward planned capacity — they exist to absorb overruns, not to be budgeted.

Weekends are fully available and carry real deep-work capacity. The engine's preference is to spend weekend capacity on **pull-ahead** rather than rescue: if a weekend is being consumed to make a Monday deadline, that is a signal the week went wrong, and Today should say so rather than quietly absorbing it.

Consequences the engine must respect:

- Tasks or slices needing 2+ contiguous hours are placed only on Mon, Fri, Thu afternoon, or the weekend.
- Tue and Wed receive slices under an hour: reading, German practice, small problem sets.
- A slipped Monday is expensive; the engine should say so explicitly rather than silently reflowing.
- 18 credits at the conventional two-hours-out-per-credit heuristic implies ~36 hours of outside work; weekday usable time is ~31 hours. Weekends close the gap, which means **the weekend is structural, not surplus.** The gauge will run tight from week one. This is a real constraint, not a calibration error, and the system should say so plainly rather than producing a comfortable-looking list.

### Importance

Stored as a 1–5 value plus a stated reason. Inferred from syllabus grade weight where the work is graded; stated by Quinn where it isn't (competition papers, applications). When genuinely uncertain, the system asks **one** pairwise calibration question per day in the brief — "is X more important than Y?" — never more.

### Effort

Inferred by default from task type and history, tagged with confidence. Quinn states estimates when he has them. Actual completion time is recorded on check-off and feeds recalibration. During the first month, estimates display confidence openly rather than projecting false precision — this is a first semester and the priors are weak.

### Slicing

Anything over ~2 hours is split into slices with their own start-by dates, so large work appears as "draft section 2 today" rather than an intimidating monolith. Slicing is also how effort estimates improve fastest.

### Recurring commitments

Weekly cadence items (post once a week, practice German most days, contact a professor every N weeks) carry their own progress-against-cadence and grow more urgent as the period closes. German in particular is modeled as near-daily small work rather than deadline-driven.

### Pull-ahead

When today's designated work is complete, Today reveals a ranked pull-ahead view of tomorrow's and this week's slices. Completing one raises aggregate slack and moves the gauge. The reward is honest because it is the same number used to apply pressure.

### Corrections

Quinn can override a rank and attach a reason. The reason is written to `preferences.md` and loaded on every subsequent run. Check-offs record whether the item was designated for today or pulled ahead.

---

## 6. Dashboard

A saved Obsidian **workspace layout** with four real tabs, each backed by a `.base` file.

**Today** — gauge, weather line, two or three sentences of prose on what the day is about, then the ranked list with due dates and progress fields. Below it: "What changed" (every add, removal, re-rank, and re-estimate since last viewed, each with a one-line reason), heads-up items (grade changes, late or missing flags, registration windows, who is owed an email), and the approvals count.

**Approvals** — card view of pending proposals, full context on each, Meta Bind buttons underneath.

**Week** — the seven-day view with capacity per day and commitment progress.

**Grades** — letter grade per course, weights, anything flagged late or missing.

There is no separate brief document. The brief *is* the Today tab, so it is never stale.

---

## 7. Permission boundary

**Rule:** if only Quinn is affected, the system acts. If another person can see the outcome, or Quinn's schedule changes, it proposes.

**Autonomous:** reading email, reading and parsing calendar, reading Blackboard, creating and updating task notes, ranking and re-ranking, writing briefs, committing to the vault.

**Requires approval:** sending any email, creating/moving/deleting calendar events, anything visible to another person.

### Approval mechanics

Proposals are written to `approvals/` with full context — draft body, the conflict it resolves, the reasoning. Clicking Approve flips `status: approved` in frontmatter. Execution happens on the next runner pass: within about a minute if the desktop is on, within the hour otherwise. Approved items are executed once, then archived.

Note the latency honestly: approving at the desk is near-immediate; approving at 11pm executes by morning.

---

## 8. Ingest sources

| Source | Path | Runner | Status |
|---|---|---|---|
| Google Calendar | Connector | Cloud | Working |
| Personal Gmail | Connector | Cloud | Working |
| Company email (Google Workspace) | Connector | Cloud | Working, deferred to v2 |
| Crimson email | Forwarded to Gmail, filtered to `Crimson` label | Cloud | Forwarding configured; M365 connector blocked by UA tenant policy |
| Blackboard due dates | Shared `.ics` subscribed in Google Calendar | Cloud | To configure |
| Blackboard grades / late flags | Browser, via myBama | Local | To build |
| Handshake / ECDC events | Email digests + browser | Cloud + Local | v2 |
| Calendly bookings | Arrive as Google Calendar events | Cloud | Working |
| Linear | Stays separate; surfaces as "work on weekly sprint goals" | — | By design |
| Syllabi, schedules, finals | Dropped into `intake/documents/` | Either | Manual drop, automatic extraction |

### Crimson resolution

The M365 connector requires tenant admin consent, which UA does not grant to users. A request to ITSD@ua.edu asking for single-user consent is outstanding but should be assumed denied. Forwarding to Gmail is the working path and is strictly better than the connector would have been, since it works with both machines off. Outbound crimson mail stays manual — drafts land in the vault, Quinn pastes and sends. Acceptable because all sends require approval anyway.

### Blackboard session strategy

The local runner uses a dedicated Chrome profile that never clears cookies, starts at myBama, and reaches Blackboard by launching it from there — riding the CAS session rather than a stale Blackboard cookie. Duo "remember this device" is enabled where offered. Re-auth is expected somewhere between weekly and monthly. A daily session-health check makes one cheap authenticated request; failure produces a toast and a line in Today, success produces silence.

---

## 9. Notifications

Three tiers, with a hard cap on the noisiest.

1. **Ambient (default).** Homepage opens Today when Obsidian launches. Zero noise.
2. **Desktop toast** (local runner, Windows, BurntToast). Only for things that are time-critical *and* Quinn is at the desk: a moved meeting, Blackboard needing re-auth, an approval that expires. Every toast must state its reason in one line.
3. **Email, once daily at 8am, never more.** Compressed brief. The floor that works when everything else is off.

The once-daily email cap is a rule in the prompt, not a guideline. A notifier learned to be dismissed is worse than none.

---

## 10. Schedules, rituals, failure handling

**Brief generation:** cloud routine at 8am. Routines apply a small deterministic stagger, so it lands a few minutes after. Quinn is showered by 8:30 at the latest and reads it when he sits down.

**Local runs:** the desktop is on most of the day and off overnight. Blackboard checks run midday and early evening. On wake, one catch-up run fires for the most recent miss. The prompt includes a time guard: if the run is happening far outside its intended window, summarize what was missed rather than acting as though it is that hour.

**Semester rollover** — written now, while the system is understood, not in December. Archive the term's courses, re-share the Blackboard `.ics`, ingest new syllabi, rebuild the capacity model from the new schedule, reset weekly commitments, update people notes.

**Add/drop and finals** — documents dropped into `intake/`, extraction automatic, calendar changes proposed for approval. UA publishes the final exam schedule separately; ingesting it is a scheduled ritual, not an ad-hoc scramble.

**Failure visibility:** routines do not retry. Any run that fails or produces nothing writes a line to Today so silence is never ambiguous.

---

## 11. Deferred, but designed around

`VISION.md` holds the end state so v1 choices do not foreclose it:

- **Tutoring** — Calendly bookings arrive on the calendar already. Client notes per client, session prep proposed the day before, covered/to-cover tracking. Client data is low-sensitivity but lives in a private repo, first names preferred.
- **SetNForget** — Linear stays the work task list. This system surfaces "work on weekly sprint goals" as a commitment and retains MCP access to Linear for editing from Claude Code.
- **Content** — a single weekly commitment line until posting actually starts.
- **Opportunities** — Handshake and ECDC event scanning, internships, research, dinners. Requires a noise filter shaped by real examples of things missed.
- **People** — contact cadence reminders producing email drafts and meeting proposals for advisors, professors, and directors.

---

## 12. Build order

1. Vault repo, `CLAUDE.md`, data model, ranking engine. Hand-entered tasks only. **Validate the ranking against real assignments before building any integration.** If the order is wrong, everything downstream is wasted.
2. Today tab in Obsidian, Bases views, capacity model from the Fall 2026 schedule.
3. Google Calendar and Gmail ingest via cloud routine. Brief at 8am.
4. Approvals: notes, Meta Bind buttons, execution pass.
5. Blackboard: `.ics` subscription first (cheap, reliable), then the local browser runner for grades and late flags.
6. Notifications: ambient, then toasts, then the daily email.
7. Then, and only then, the v2 domains.

Usable in roughly two days. Trusted in roughly two weeks.

---

## 13. Open decisions

**Calendly availability windows.** Availability is currently 8am–9pm daily in 1- and 2-hour slots. This exposes Monday 8:30–11 and Friday 8:30–12 — the only multi-hour work blocks in the week — to client booking. A single booking in either place removes the ability to do deep work that week.

Options:
- **(a) Protect both windows in Calendly.** Roughly 50 hours of weekly availability remains, ample for overflow clients. Recommended.
- **(b) Leave availability open, flag conflicts after booking.** Preserves earning flexibility but the booking is already confirmed; resolving it means emailing a client.

Decision needed before the capacity model is finalized in step 2.
