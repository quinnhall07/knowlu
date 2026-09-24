# Know what's next — the commitment model and the program it starts

**Date:** 2026-09-23. **Status: direction agreed with Quinn; each piece still goes through
brainstorm → spec → plan before code.** Tagline (Quinn): **"Knowlu: know what's next."**

This note records what came out of the first real labelling session (2026-09-22/23), Quinn's product
decisions that followed, and the order the work goes in. It carries **patterns only** — the labelled
items themselves are Quinn's personal data and stay in the git-ignored workspace
(`.superpowers/sdd/2026-09-22-judgment-quality-plan/labelling/`), never in this repository.

## 1. Decisions (Quinn, 2026-09-23)

1. **Commitments live in the vault (option A).** Knowlu's calendar is typed notes in the vault, the
   single source of truth as CLAUDE.md already requires. Google Calendar is read as one input (the
   `calendar.readonly` scope we already hold); writing back to Google is a possible later addition
   and needs the `calendar.events` scope, a new consent and a privacy-page bump.
2. **Every calendar item carries its commitment as metadata**: its kind (class, work, club, meeting,
   event, exam, task block) and its level — **hard** (class, work: never overlapped), **soft** (club,
   meeting: a proposal may overlap it), **optional** (a proposal). The system proposes events that
   overlap soft commitments and never hard ones.
3. **Email auto-labels are dropped** ("not really necessary right now"). No `gmail.modify` scope.
4. **All the product ideas from the session go ahead**, in the order below, and every feature is held
   to one test: *does it improve the morning answer — what should I work on today, and in what order?*

## 2. The program, in dependency order

| # | piece | depends on | notes |
|---|---|---|---|
| 1 | **The commitment model** (typed calendar entries in the vault, commitment level, source) | — | engine only; touches no `app/static` file, so it cannot collide with the open UI branches. **First.** |
| 2 | **Event judging v2** — three stages: *is it a real, attendable event* (model), *is it for my level and role* (deterministic audience filter), *do I want it* (learned) — plus the deterministic rules below | 1 | |
| 3 | **Email v2** — Quinn's taxonomy (**event = already committed → calendar; opportunity = optional → propose**), vendor templates, digest splitting, what-and-when summaries on every approval | 1 | the prompt's current definitions differ; see §3 |
| 4 | **Preference learning** — interests with decay (they change) and exploration (never drive a category to zero), "useful / not useful" on information items, organisation membership (inferred or asked), unsubscribe offers for unwanted newsletters | 1 | per-account learning is permitted even for Gmail-derived signals ("that user's personalised model") |
| 5 | **A viewable calendar** in the app, Notion-Calendar-like, over the vault's commitments and Google's events | 1 | `app/static` — after C1b, C1c and C3′ land |

Unsubscribe uses the sender's own `List-Unsubscribe` (one-click URL or mailto) and needs no Gmail
write scope; it is an action on the student's behalf, so it is always the student's click.

**The gap piece 1 must close first: there is no class schedule.** A wizard-created vault's
`config/week_template.yaml` has an empty class list every day, and the LMS feed carries assignments,
not class meetings — so "never overlap a class" has no data today. Candidate sources: ask at
onboarding; a registrar `.ics` export; classes already on the student's Google Calendar.

## 3. What the labelling session showed (100 items: 50 campus events, 50 emails)

- **Event labels answer "is this a real, offerable event", not "is it for me".** 47 of 50 were
  labelled opportunity and none drop, including faculty-and-staff-only events; the notes carry the
  real rule — *not for undergraduates*, *targeted at graduate students*. Hence piece 2's three stages.
  Scoring the pinned models against these labels must use this reading, not the prompt's.
- **A class can also be a public campus event** (a course session listed for upperclassmen). It must
  become an obligation by deterministic match against the student's courses and class times.
- **Email "event" meant committed and "opportunity" meant optional** — meetings the student set up and
  registration confirmations are events (straight onto the calendar, no proposal); club meetings, job
  listings and programmes are opportunities. The current prompt defines event as anything at a stated
  time and opportunity as applications and jobs only. Quinn's split is the better taxonomy.
- **"Already on my calendar → don't propose it"** was noted four times, as a general rule.
- **Accepting some proposals should create a task** (register for the event; finish the presentation).
- **Digest emails** (weekly campus newsletters) were 10% of the sample and each holds several events,
  opportunities and notices — one tier loses most of it.
- **A posted grade is its own signal** — not completion (the T9 veto is right), but a grade to record
  against the course, which the parent design planned and nothing fills.
- **A declined recurring opportunity should stay declined** across the series.
- **LMS "new message" emails are stubs** — the useful content is the announcement behind them.

## 4. Deterministic vs heuristic

| deterministic (rules, no model) | heuristic (model or learned) |
|---|---|
| audience / level filter against the student's profile | splitting a digest into its items |
| class match → obligation | tasks and deadlines from personal correspondence |
| already-on-calendar dedupe (title + time overlap) | organisation membership, where not asked |
| vendor templates (LMS receipt / grade / due-soon / message; job-board listings; registration confirmations → calendar) | preference learning — per-feature scores (organiser, category, source) with decay and exploration; no LLM needed |
| newsletter detection from `List-Unsubscribe`; the unsubscribe action | whether a vague event is attendable (`unsure`) |
| series suppression after a decline; accept → "register" task when a registration link exists | summaries for email-derived items |
| approval summaries from structured fields ("Career fair · Thu 1 Oct 10–3 · venue") | |

The model's job keeps shrinking to the residue — the same conclusion the completion work reached
(`2026-09-22-completion-detection-design.md` §7b).

## 5. Related decisions carried from 2026-09-22

- **`unsure` events surface as a decision card** ("Does this apply to you?" — approve → obligation,
  reject → drop) built engine-side now; an in-Today "not sure this applies to you" marker follows the
  UI branches. The student's answer is the first real labelled event data the product collects.
- **Thread `judgment_id` end to end** (the device discards it today), so every class-(b) correction
  becomes a calibration pair — the first step of the user-data plan, needing no new consent.
- **A feed bug found by the collector:** the configured Localist and Engage URLs return only their
  default first page (10 events each); with date and page-size parameters the same feeds held 462 and
  176 events in the next month. The live product judges a sliver of the campus calendar.
