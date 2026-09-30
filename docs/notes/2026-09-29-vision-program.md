# The vision's program — streams, the MVP line, and what quinn-ops parity still needs

**Date:** 2026-09-29. Written from `VISION.md` ("Build order") and a read-only parity audit of the
archived quinn-ops against Knowlu's code the same day. It maps each piece of the vision to a stream
and says which stream owns it. `HANDOFF.md` stays the source of truth for what is running right now;
this file is the map those streams come from.

## The MVP line

**The MVP is everything quinn-ops did, plus grades from Blackboard.** Knowlu inherited quinn-ops's
engine and console whole, so most of parity is already done: the today page (runway verdict, capacity,
schedule, must-do against capacity, recommended takes, all-active with start-by and slack, coming up),
the decisions deck (approve, reject, snooze), progress, field editing, runs and issues, and the
Blackboard feed, zyBooks, VHL and Google Calendar sources.

What parity still needs, most noticeable first:

| # | Gap | Home |
|---|---|---|
| P1 | **Class schedule and capacity** — no class times and a fixed 08:00–18:00 day, so every capacity number is wrong for a new student | the commitment model (PRs #14, #16; another session's program) |
| P2 | **Grades** (not in quinn-ops; the MVP adds it) | **M1** (`m1-grades`, spec `docs/specs/2026-09-29-grades-design.md`) |
| P3 | **Gmail can't be connected in the app**, and production holds none of C2 | C4's settings row + HANDOFF §4's production-parity checklist |
| P4 | **Events** — required events never become tasks; events can't be accepted or declined; feeds exist for Alabama only | event judging v2 (the direction note's piece 2) and the `unsure` card (PR #12, merged) |
| P5 | **Task body and profile editing** — the drawer body is read-only; interests and preferences need a text editor | **M2** (small; a set-body command through `write` and the drawer) |
| P6 | Smaller: a producer for Good to know items, conflict flags, opportunity proposals expiring after 14 days, the dropped-event audit list | M2, or cut on Quinn's word |

Deliberately not parity (rulings stand): toasts and the daily email (superseded for nudges by the
2026-09-29 amendment, ruling 2, post-MVP), writing to Google Calendar, event digests (replaced by one
card per event), phone access to `today.md`, the Obsidian layer.

## After the MVP — the vision's phases, in order

| Phase | Stream | Status |
|---|---|---|
| 1 | The commitment model | phases 1 and 2 merged (PRs #14, #16); phase 3 on `p3-registrar` |
| 2 | The main page: domain strip, free time, all clear, today's schedule | spec after the commitment model lands (free time needs it) |
| 3 | Syllabus upload → grade weights, GPA and band, exam prep | spec after M1 (it replaces M1's "points so far") |
| 4 | Quick capture (hotkey, box, dictation) | spec |
| 5 | Nudges and lock-in | spec |
| 6 | The assistant | spec; its purpose is fixed by VISION |
| 7 | Outlook / Microsoft 365, more homework platforms | spec |
| 8 | The dedicated Knowlu calendar in Google and Outlook | spec; needs a new consent |

**The order of work is `HANDOFF.md` §3, the single ordering** (the stages MVP → Pilot → Launch →
Beyond of the cloud design's Amendment 2026-09-29, ruling 10; C4 is no longer a phase). Where this
note names C4 or an older order, §3 wins.
