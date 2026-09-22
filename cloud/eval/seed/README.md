# The eval seed

This directory was empty on purpose. The eval suite needed a floor of labelled cases on day one, and
the only place enough of them existed was an archived vault — but on 2026-09-14 that read was
declined: nothing is ever read from an archive to seed this suite. So `cloud/eval/` shipped with the
schema, the scrub rules and the loader that would check every case against them, and no cases at
all — until T2 (stream J, 2026-09-22), which hand-wrote `events.jsonl` below: the first cases this
suite has ever scored. `tasks.jsonl` and `email.jsonl` still carry no cases and fill the same way the
rest of this file describes: one account at a time, and only with that account's consent. A later
stream adds the opt-in — a student reviewing one of their own corrections and choosing to let it
become an eval case — and that correction already carries the shape below, because it is the same
`request` the device sent when it was first judged.

## The honest limit (T2)

`events.jsonl` is twenty-six hand-written cases, invented, not sampled — no real student, event,
organizer or institution appears in it. They measure one thing: whether a model applies the rules
`judge_prompts.ts`'s event template actually states (the two drop rules, the three verdict words,
now four — see below). **They are not the real distribution of campus events**, and at this size a
three-class estimate carries roughly twenty points of error at ninety-five percent confidence. A
score on this seed **can rule a prompt or model change out. It cannot rule one in.** The first real
signal comes from `eval_cases` rows the opt-in writes, not from this file.

`events.jsonl` includes cases labelled `verdict: "unsure"` — J-1's ruling (2026-09-22, design (a)):
a fourth verdict word for "whether this event obliges this student is not in the event text."
Nothing in `cloud/eval/schema.ts` or `cloud/eval/loader.ts` restricts `theirs.verdict` to a fixed
set of words (only that the key `verdict` is the one an event case may label), so these cases load
and score today; `cloud/eval/score.ts`'s `COST` table is this directory's own enumeration of event
verdicts, and it prices `unsure` transitions by the same default-1 fallback every unnamed transition
gets — ratifying named costs for `unsure` (like the rest of the cost-matrix proposal,
`docs/notes/2026-09-22-cost-matrices.md`) is separate, unruled work this task does not do.

## The record shape

A seed record is one line of a `cloud/eval/seed/*.jsonl` file, an object with exactly four keys:

```json
{
  "id": "seed-0001",
  "kind": "task",
  "request": {
    "kind": "task",
    "item": { "id": "seed-item-0001", "title": "Problem set 3", "source_uid": "zybooks:cs-101:ps3", "created_by": "zybooks", "course": "cs-101", "due": "2026-10-01" },
    "heuristics_seed": { "course": "cs-101", "effort_hours": null, "slice_hours": 1.5, "weights": "Homework 20%\nExams 50%", "preferences": "", "known_courses": ["cs-101"] }
  },
  "theirs": { "effort_hours": 2.5, "importance": 4, "course": "cs-101" }
}
```

`id` is a synthetic, non-empty string with a `seed-` prefix — never an email address, never a UUID
from a live table. `kind` is `task`, `event` or `email`. `request` is exactly the body the device
sends to `/judge-<kind>` today (`kind`, `item`, `heuristics_seed` — nothing else), and `request.item`
carries no key outside the fields that kind's request actually has. `theirs` is the human's label:
a non-empty subset of the fields that kind scores — `effort_hours` / `importance` / `course` for a
task, `verdict` for an event, `tier` and any of `title` / `course` / `due` / `effort_hours` /
`importance` for an email. An event and an email case follow the same shape:

```json
{"id": "seed-0002", "kind": "event", "request": {"kind": "event", "item": {"uid": "seed-event-0002", "title": "Fall involvement fair", "start": "2026-10-05T10:00", "end": "2026-10-05T14:00", "source": "campus-calendar"}, "heuristics_seed": {"interests": ""}}, "theirs": {"verdict": "opportunity"}}
{"id": "seed-0003", "kind": "email", "request": {"kind": "email", "item": {"message_id": "seed-msg-0003", "subject": "Assignment reminder", "date": "2026-10-02"}, "heuristics_seed": {"known_courses": ["cs-101"]}}, "theirs": {"tier": "task", "title": "Problem set 3", "course": "cs-101"}}
```

`cloud/eval/schema.ts` exports `SeedRecord`, `validateSeedRecord` and `scrubViolations`, both
checked against this shape.

## The scrub rules

Every case that ever lands here goes through two checks before `loadSeed` (`cloud/eval/loader.ts`)
will hand it back: `validateSeedRecord` (the shape above — the four keys, the allowed item fields,
the labelled `theirs` fields) and `scrubViolations` — no `@`-bearing token anywhere in `request` or
`theirs`, no `http://`/`https://` URL, no Windows or POSIX home path, no run of 7 or more digits, no
key named `email`, `from`, `to`, `organizer_email`, `attendees`, `body`, `snippet` or `raw` anywhere
in `request.item` (some of those really are fields the device sends today — the rule is that a
*seed* item never carries one), and no string field over 200 characters.

A line that fails either check does not get silently skipped: `loadSeed` throws, naming the file and
the line number. A hole in this suite is worse than a loud failure while it is being written.
