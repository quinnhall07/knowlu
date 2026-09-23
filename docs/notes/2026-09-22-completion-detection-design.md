# Completion detection — can Jev tell us a task is done?

**Date:** 2026-09-22. **Asked by Quinn**, mid-stream-J: *"see if we can get Jev to work for checking
if a task has been completed based on the different sources we'll receive info from."*

**The short answer.** Yes for a narrow and valuable slice, and the valuable slice needs no model at
all. The part that does need a model is a better Jev case than the event kind was — because, alone
among everything in this repository, **it labels itself**. But it is third in line, and the first two
steps are what build the corpus the third one would need.

This note argues from the code. Every claim about Knowlu carries a file and a line; anything about
the outside world is marked unverified unless it carries a URL and a date.

---

## 1. What "done" means in the vault today

| fact | where |
|---|---|
| `status` is `active` / `done` / `archived` | `engine/src/approvals.rs:476`, default `active` at `engine/src/models.rs:232` |
| only `active` notes rank at all | `engine/src/models.rs:290`, `:305` |
| `progress` is `0`–`100` | `engine/src/models.rs:150`, parent design §4 |
| **`progress` is never written after creation** | `engine/src/coursework.rs:241`, tests at `:2090` and `:2244` |
| `completed_at` is in the parent design's schema | `docs/specs/2026-08-11-personal-ops-system-design.md:137` |
| **`completed_at` does not exist in the engine** | `grep -rn completed_at engine/src` returns nothing |

So: nothing in Knowlu derives completion from anything. The student ticks a task done by hand in the
console, and that tick is the only completion signal the system has ever had. The parent design
anticipated more — "actual completion time is recorded on check-off and feeds recalibration"
(`:192`) — and that half was never built either.

**This is the gap.** It is not a small one. A ranked list that keeps showing finished work is the
single most corrosive failure this product has, because it teaches the student that the list is
wrong, and a list the student does not trust is a list they stop opening.

## 2. The permission story already exists, and it is the right one

The repository's standing rule (CLAUDE.md): *"Judge once, re-propose freely: an agent never re-sets a
field the journal shows the user set; with `propose` it files a `kind: amend` approval instead."*

Two facts decide the whole design:

- **`status` is one of the nine amendable fields** — `engine/src/approvals.rs:42-52`.
- **`progress` is not** — and the refusal is asserted by a test, `engine/src/approvals.rs:2746`
  (`field not amendable: progress`).

Therefore a completion detector's lawful output is **an `amend` proposal moving `status` from
`active` to `done`**, which the student approves or rejects in the console. It is capped at 15 new
proposals a day, overflow snoozed and never deleted, and it goes through machinery that already
exists, is already tested, and already has a surface (`surface --view decisions`).

Nothing about this needs a new field, a new write path, a new note type or a migration on the device
side. That matters more than it sounds: it means step 1 below is small.

## 3. What each source actually gives us

This is the heart of Quinn's question — *based on the different sources we'll receive info from.*

| source | what we fetch today | completion signal | needs a model? |
|---|---|---|---|
| **VHL** | `percentage_complete` per item, every run | **yes, already in hand** — `engine/src/vhl.rs:235` | **no** |
| **zyBooks** | sections and `total_points`; writes `progress: 0` hardcoded at `engine/src/zybooks.rs:309` | **yes, verified 2026-09-22 — `section_scores`, already in the payload** | **no** |
| **LMS `.ics`** | due dates | none, deliberately | no |
| **Gmail** | subject, sender, scrubbed body into five tiers | **yes, and we throw it away** — see below | yes |
| **Calendar events** | title, time into obligation / opportunity / drop | weak | yes, marginally |
| **The journal** | every field the student set, with actor and timestamp | this is not a signal — **it is the ground truth** | n/a |

Three of those rows deserve their own paragraph.

**VHL is free money.** `percentage_complete` is fetched on every coursework run and used to seed
`progress` at creation, then never looked at again (`engine/src/coursework.rs:241`). A VHL item
reporting 100 is a vendor certifying that the student finished it. That is not a judgment, it is a
number, and turning it into an amend proposal is deterministic code in the engine — which is where
it belongs, because the engine is deterministic and `rank` never calls a model.

**The `.ics` feed must stay silent, and the code already knows why.** An item vanishing from an LMS
feed is not evidence of completion; feeds have windows, and `engine/src/ingest.rs:724` says it
plainly: *"the one thing worse than importing a stale task is archiving a live one."* That judgment
is correct and this design does not disturb it.

**Gmail is where the signal is, and we discard it by design.** The email triage has five tiers —
`task`, `borderline`, `event`, `opportunity`, `information` (`judge_prompts.ts:66`,
`judge_validate.ts:22`). **None of them means "this reports that existing work is finished."** So
"Your submission for HW 3 was received", "Your quiz has been graded", "Lab 4 marked complete" all
land in `information`, and `information` is the noise tier: nothing is written, and the uid is
recorded so it is never asked about again (`engine/src/enrich.rs:628-636`, test at `:1923`).

The best completion evidence a student receives is currently classified as noise, dropped, and
tombstoned so it can never be reconsidered.

## 4. Where a model belongs, and where it must not

The judge command already has three tiers (heuristics, promoted rules, the model). Completion
detection has exactly the same shape, and the tiering is the whole argument:

- **Tier 1, deterministic.** Vendor says 100, propose done. No model. It must not be a model: a
  number that arrives certain does not get handed to something that will answer 0.87.
- **Tier 2, promoted rules.** The same promotion machinery `judge-rules` already runs. "Email from
  the LMS no-reply address whose subject starts `Submission received:` and whose course matches" is
  a rule, and rules are free, auditable and instant.
- **Tier 3, the model.** Only the residue: evidence whose meaning is genuinely in the prose. This is
  a typed decision over a bundle of signals, returning one of a small set with a probability — which
  is precisely the shape Jev sells.

## 5. Why this is a better Jev case than the event kind — two reasons

The event kind closed on measurement, not privacy (`docs/notes/2026-09-22-jev-event-kind-experiment-design.md`).
It closed because there is no corpus, no cheap way to label one, and the published calibration is
too weak to support a threshold. Completion detection changes both halves of that.

**(a) It labels itself.** Every amend proposal the student approves or rejects is a ground-truth
label, produced as a byproduct of a flow that already exists, journaled with its actor, its
timestamp and the evidence that prompted it. Nobody labels anything. The corpus grows because the
product is being used. This is the one property the event kind could not get at any price, and it is
worth saying flatly: **ship the deterministic half and the corpus builds itself.**

**(b) The cost asymmetry is sharp, knowable, and points the opposite way from the current floor.**
A false *done* hides live work and the student misses a deadline — the worst outcome this product
has. A false *not-done* shows a finished task the student dismisses in one click. Under Elkan's
theorem the cost-optimal threshold is `t* = C_FP / (C_FP + C_FN)`
(`docs/notes/2026-09-22-confidence-calibration-and-the-floor.md` §4). At a 10:1 ratio that is
**0.91**; at 20:1, **0.95**. Compare `CONFIDENCE_FLOOR = 0.6`
(`cloud/supabase/functions/_shared/judge_validate.ts:6`), which the calibration note shows is
optimal for a 2:3 ratio — the wrong direction entirely for this question.

A threshold at 0.91 is a demanding thing. The calibration note's finding is that self-reported
`confidence` is too coarse to support it: the mass sits between 0.5 and 0.95 and carries little
information. **A threshold that high is exactly the case a calibrated probability is for**, and it
is the strongest argument in this document for eventually buying one.

## 6. What it would cost to add a fourth kind — and why not to

A fourth judgment kind is not a prompt change. The three kinds are welded into the schema:

```
cloud/supabase/migrations/20260911000100_judgment_service.sql:39   models.kind
cloud/supabase/migrations/20260911000100_judgment_service.sql:70   judgments.kind
cloud/supabase/migrations/20260911000100_judgment_service.sql:103  judgment_kind
cloud/supabase/migrations/20260911000100_judgment_service.sql:113  (kind)
cloud/supabase/migrations/20260911000100_judgment_service.sql:155  (kind)
cloud/supabase/migrations/20260911000700_eval.sql:9                eval cases
cloud/supabase/migrations/20260911000700_eval.sql:31               eval runs
```

**Seven `check (kind in (...))` constraints across two migrations**, plus a new edge function beside
`judge-task` / `-event` / `-email`, a `DAILY_CAP` entry (`judge_caps.ts`), a `models` row
(`judge_models.ts:27-30` keys on kind), a prompt, a schema and a new `promptHash`. In the engine: a
new pass in `enrich.rs`'s `run_lines_with`, which already hosts four cloud pulls per slot.

**A sixth email tier costs one enum member and one prompt line.** `judge_prompts.ts:66` gains
`completion`; `judge_validate.ts:22` gains the same; `enrich.rs` routes that tier to an amend
proposal instead of to the drop. Same route, same provider, same pinned model, same budget, same
prompt-hash discipline, **no migration**.

That asymmetry decides the ordering by itself.

## 7. Open questions, and what is unverified

1. ~~**Does the zyBooks payload carry per-student completion?**~~ **ANSWERED 2026-09-22, and the
   answer is yes.** See §7a.
2. **Does a VHL item ever report 100 and then move?** Unverified. The design must survive it: an
   amend proposal is reversible by construction, and the journal records the reversal.
3. **What is the real cost ratio?** Quinn's, and it is the same T0 question already blocking the
   floor work — a completion matrix is one more row on a form that is already waiting.
4. **Jev's calibration on a three-way choice with evidence of this shape.** Nothing published covers
   it. Every number in the Jev notes is from the event-kind and development work; none of it
   transfers to this task, and claiming otherwise would be inventing evidence.
5. **Whether Jev may see this data at all.** The email path already sends a scrubbed subject, sender
   and clipped body (`judge_prompts.ts:185-198`), so the *data class* is unchanged and needs no new
   ruling. But Jev is a different provider from the pinned OpenRouter route, and under D-R8 a new
   inference provider is a privacy-page version bump, a notice to every account and an in-app yes.
   **That is the real gate on step 3, and it has not moved.**

## 7a. The zyBooks answer, measured 2026-09-22

Run against Quinn's own zyBooks account with their standing permission, read-only, nothing written:
`GET /v1/zybook/UACS100Fall2026/assignments` — **the endpoint the engine already calls on every
coursework run** — returns 23 assignments, each carrying an eleven-key object that includes
**`section_scores`**, an array with `challenge_earned`, `lab_earned`, `participation_earned`,
`chapter_number`, `section_id` and `section_number` per section.

Summing those three earned figures against the `total_points` the parser already reads gives a
completion percentage directly:

| state | count of 23 | examples |
|---|---|---|
| 100% of points | **7** | Lab 01, Lab 02, Lab 03, HW 01, HW 02, HW 05, Project 1 |
| partial | **2** | HW 03 at 91% (16 of 18 sections scored), Project 2 at 87% |
| 0% | 14 | HW 04, HW 06–13, Lab 04–08 |

**So the completion signal has been arriving in the payload all along and the parser drops it on the
floor** — `parse_assignments` reads `title`, due dates, section counts and `total_points`, ignores
`section_scores`, and hardcodes `progress: 0` at `engine/src/zybooks.rs:309`.

**zyBooks therefore joins VHL in tier 1, and it is the bigger half** — this is the CS coursework,
23 items against VHL's language sections.

**Two design consequences, both real.**

*Points earned is not the same as finished.* A student can complete every section and still not earn
full points on a challenge. So 100% of points is a **stronger** condition than "done" — it implies
every point-bearing section was completed correctly. The recommendation is to fire tier 1 at 100%
only, and leave the 80–99% band (HW 03, Project 2) to tier 3 later, where it is exactly the
ambiguous residue a model is for.

*Section counts are the weaker signal, not the stronger one.* HW 01 shows 18 of 25 sections scored
yet 193 of 193 points, because seven sections carry no points. Counting sections would call it 72%
done when it is finished. **Points, not sections** — which is also what the effort model already
uses.

The probe was read-only, printed field names and aggregates, stored nothing and wrote nothing to the
vault. The credential was read from Windows Credential Manager at runtime and never persisted, per
the standing rule that no secret enters the repo, a log, a prompt or a test name.

## 7b. The email answer, measured 2026-09-22, and what was built

Run against Quinn's own Gmail with their standing permission, read-only. Five weeks of Blackboard
mail held 18 templated threads from the vendor's no-reply sender, in four shapes: **"Submission
received"** (the body names the assessment and a confirmation number), **"New grade and feedback for
<title> in <course>"**, **"<title> is overdue in <course>"** and **"<title> is due soon in
<course>"**. The assessment titles match the vault's Blackboard task titles verbatim.

Two consequences changed the design in §6 and §8:

- **The email evidence is rule-tier, not model-tier.** A template this rigid is a rule. T9 therefore
  runs a deterministic recogniser in `gmail-read` *before* the model: a recognised receipt costs no
  model call and no cap, and becomes the new `completion` tier with the title taken from the body.
  The model path also learned the tier, for receipts from vendors with no template yet.
- **A grade is not completion.** Instructors post a zero for missing work, which sends the same
  "New grade" email. Only the submission receipt counts. A data-driven veto downgrades any
  `completion` answer — model or promoted rule — for a vendor's grade, overdue or due-soon mail to
  `information`, so this is enforced in code rather than hoped for in a prompt.

On the device, a `completion` item matches exactly one active task by normalised title and files the
same `status: done` amend proposal the vendor tier files (`engine/src/completion.rs`); zero or several
matches write nothing. A device only receives the tier if it declares `accepts: ["completion"]`, so an
older engine sees `information` and never files a wrong card.

**What this does to the Jev case (§5, step 3).** It shrinks it. The structured half of the email
evidence never reaches a model at all, so tier 3 sees only the residue — receipts from vendors with
no template, and prose that genuinely needs reading. The self-labelling argument of §5 still holds,
but the volume a model would ever judge here is small, and E5's stop rule should expect that.

## 8. The honest ranking

1. **Tier 1, deterministic, no model.** VHL at 100 **and zyBooks at 100% of points** propose
   `status: done`. Engine-side, no cloud, no provider, no ruling, no migration. This is the free
   half, it now covers both vendors, and it should ship regardless of what happens to everything
   else in this note.
2. **The sixth email tier.** One enum member, one prompt line, existing route and budget. Routes
   "submission received" to a proposal instead of to the bin.
3. **Ask whether a model beats the rules** — using the labels that steps 1 and 2 have been
   collecting all along, against the promoted-rule baseline, at a threshold derived from Quinn's
   cost matrix rather than from a constant.

**Jev is a candidate for step 3, not for step 1.** And the thing that makes step 3 answerable is
that by the time we reach it we will have what the event kind never had: a corpus, labelled for
free, by the person whose judgment is the target.

## 9. Sources

Knowlu code and specs as cited inline, at `main` `20b4de0`, read 2026-09-22. Elkan's threshold and
the calibration findings are carried from
`docs/notes/2026-09-22-confidence-calibration-and-the-floor.md` (2026-09-22), which holds the URLs
and dates for every external number quoted there. The provider ruling is D-R8 as amended, in
`docs/specs/2026-09-09-knowlu-cloud-design.md` §1 and
`docs/notes/2026-09-16-inference-provider-and-model-scoping.md`. **No number in this note comes from
Jev's documentation**, because none of the published material covers a decision of this shape.
