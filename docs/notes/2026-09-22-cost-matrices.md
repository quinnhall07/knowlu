# The cost matrices — a proposal for ratification (T0)

**Date:** 2026-09-22. **Status: RULED by Quinn 2026-09-22** ("looks good"). Quinn asked to address
these properly rather than guess, so this works every cell out from stated principles and brought
the result back for a yes, a no, or an edit. §9 below records the `unsure` and `completion` cells
the controller ruled the same day for the two things this note left open (event's fourth verdict,
and the `completion` tier this note's §5 flags but does not price as a *classification* cost). All
42 off-diagonal cells (event's 12, email's 30) are now in `cloud/eval/score.ts`'s `COST` map.

**What a cost matrix is here.** Not a measurement. A statement of how bad each mistake is *in units
of the other mistakes*. The numbers only have to be right relative to each other, because what they
feed is a ratio: Elkan's cost-optimal threshold is `t* = C_FP / (C_FP + C_FN)`, so doubling every
cell changes nothing and getting one cell wrong by a factor of two moves a threshold.

---

## 1. What is already ruled, and what is silently guessed

`cloud/eval/score.ts:28-37` names eight cells:

| truth → ours | cost | kind |
|---|---|---|
| `obligation → drop` | 3 | event |
| `obligation → opportunity` | 1 | event |
| `opportunity → drop` | 1 | event |
| `drop → opportunity` | 1 | event |
| `task → information` | 3 | email |
| `task → borderline` | 1 | email |
| `information → borderline` | 1 | email |
| `information → task` | 2 | email |

A missing answer costs 3 (`score.ts:44`), deliberately, so that a model which fails safe by
answering nothing scores no better than one that answers worst.

**The event kind has six off-diagonal cells and the email kind has twenty.** Eight are stated.
**The other eighteen fall through `COST[...] ?? 1` and are scored as 1** — which is a guess wearing
the costume of a decision. It currently asserts that calling a scholarship a newsletter costs the
same as calling a lab a lecture.

## 2. The five principles the missing cells are derived from

Each is drawn from a ruling or a design commitment that already exists, not invented here.

- **P1 — losing real work is the catastrophe.** This is the product's one promise. Anything whose
  truth is `task` landing in `information` is the maximum, 3. Already ruled.
- **P2 — an abstain lane is nearly free.** `borderline` exists so a human decides. A misroute *into*
  borderline costs 1, never 0: it costs the student a decision, which is real but small. Already
  ruled for two cells; extended here to all of them.
- **P3 — inventing work costs trust.** Anything not truly a task, presented as a task, costs 2.
  Already ruled for `information → task`; extended to the rest. The asymmetry against P1's 3 is
  deliberate: invented work is discovered in seconds and dismissed, missing work is invisible until
  the deadline.
- **P4 — confusions among the non-task tiers are mild**, 1, when the item still surfaces somewhere.
- **P5 — a vanished opportunity is a catastrophe in email and a nuisance in events**, because
  **the word means two different things.** In the email tiers, `opportunity` is defined as "an
  application, a scholarship, a job, a research post" (`judge_prompts.ts`, email template) — dated,
  consequential, and gone if missed. In the event verdicts it means a calendar item worth offering,
  a talk or a dinner. The existing map already prices event `opportunity → drop` at 1; this proposal
  prices email `opportunity → information` at 3, and the two are consistent *once the overload is
  noticed*. **Flagging the overload is itself a finding**: two enums, one word, two meanings.

## 3. The event matrix, complete

Rows are the truth, columns are what we answered. Diagonal is zero throughout.

| truth ↓ / ours → | obligation | opportunity | drop |
|---|---|---|---|
| **obligation** | 0 | 1 *(ruled)* | **3** *(ruled)* |
| **opportunity** | **2** *(new)* | 0 | 1 *(ruled)* |
| **drop** | **2** *(new)* | 1 *(ruled)* | 0 |

**The two new cells, and why 2.** Both are "we told the student this is a must, and it is not" — P3.
Not 3, because a false obligation is visible and dismissible; not 1, because false urgency in a
product whose entire claim is *what should I work on today* is the fastest way to lose the student's
trust in the list.

## 4. The email matrix, complete

| truth ↓ / ours → | task | borderline | event | opportunity | information |
|---|---|---|---|---|---|
| **task** | 0 | 1 *(ruled)* | **2** | **2** | **3** *(ruled)* |
| **borderline** | **1** | 0 | **1** | **1** | **2** |
| **event** | **2** | **1** | 0 | **1** | **2** |
| **opportunity** | **2** | **1** | **1** | 0 | **3** |
| **information** | 2 *(ruled)* | 1 *(ruled)* | **1** | **1** | 0 |

**Sixteen new cells. The four that are not 1 or the obvious application of a principle:**

- `task → event` and `task → opportunity` = **2.** The deadline survives and the item is visible,
  but it is not tracked as work and never enters the ranking. Worse than `borderline` (which asks),
  better than `information` (which vanishes).
- `borderline → information` = **2.** Borderline's whole purpose is that a human should look. Losing
  it silently defeats the tier, so it costs more than borderline's other confusions.
- `opportunity → information` = **3.** P5. A scholarship deadline that vanishes is the second
  catastrophe in this product, and pricing it below `task → information` would say that a missed
  application matters less than a missed problem set. It does not.
- `event → information` = **2**, not 3. Announced events are usually announced more than once.

## 5. The completion matrix — and the thing that changes it

This is the new kind (T8/T9/T10, `docs/notes/2026-09-22-completion-detection-design.md`), and it
needs saying carefully, because **the answer depends on the delivery mechanism and I got this wrong
in the first pass.**

The design note reasoned that a false *done* hides live work and a missed deadline follows, giving
10:1 or 20:1 and a threshold of 0.91–0.95. **That is the cost under auto-apply, and T8 does not
auto-apply.** `status` is amendable (`approvals.rs:42-52`), so the output is an `amend` proposal the
student approves or rejects. A wrong "done" therefore never removes anything: it produces a proposal
that gets rejected in one click.

So there are two matrices, and which one applies is a design choice rather than a fact.

**(a) Under proposals — what T8 actually ships:**

| truth ↓ / ours → | done | not_done | unsure |
|---|---|---|---|
| **done** | 0 | **1** | **1** |
| **not_done** | **2** | 0 | **1** |

`t* = 2 / (2 + 1) = 0.67`. A false proposal costs a rejection plus a slot against the 15-a-day
approvals cap; a missed one costs a task lingering on the list until the student ticks it.

**(b) Under auto-apply — if a measured slice ever earns it:**

| truth ↓ / ours → | done | not_done | unsure |
|---|---|---|---|
| **done** | 0 | **1** | **1** |
| **not_done** | **15** | 0 | **1** |

`t* = 15 / 16 = 0.94`. Marking live work finished removes it from the ranking silently, and the
student finds out at the deadline. 15 rather than 10 or 20 because it should sit clearly above the
3 that prices every other catastrophe in this file: those *fail to add* something, this one
*removes* something the student was already tracking.

**Why this matters more than the numbers.** It gives the graduation path for free. **Ship at 0.67
behind a proposal, collect approve and reject as ground-truth labels, and only consider auto-apply
for a slice whose measured precision clears 0.94.** The cheap version generates exactly the evidence
the expensive version would need, which is the same argument the design note makes for the corpus.

## 6. What ratification changes in the code

`cloud/eval/score.ts`'s `COST` map gains eighteen cells, so nothing silently defaults. The diagonal
stays implicit (`cost()` returns 0 on a match) and the missing-answer rule stays at 3. **No model
change, no prompt change, no migration.** A ruling line lands in `HANDOFF.md`.

The event thresholds in `cloud/eval/thresholds.json` are unaffected in form but their *meaning*
changes, because `weighted_exact` is computed against these costs. That number is marked unmeasured
today, so nothing regresses.

## 7. What I am least sure of, stated plainly

1. **`drop → obligation` at 2.** An argument exists for 3: a list that invents obligations is worse
   than one that misses them, because the student stops believing it. I chose 2 because the error is
   visible and one-click. **This is the cell most worth Quinn overruling.**
2. **`opportunity → information` at 3 in email.** It equals the product's worst error. Defensible,
   and it is a claim about what this product is for.
3. **The completion ratio of 15.** It is a placeholder for "clearly worse than 3", not a measured
   quantity, and it only ever applies if auto-apply is built.

## 8. Sources

`cloud/eval/score.ts` and `thresholds.json`, `cloud/supabase/functions/_shared/judge_prompts.ts`
and `judge_validate.ts`, `engine/src/approvals.rs`, read 2026-09-22 at `main` `0a580d1`. Elkan's
threshold result and its derivation are carried from
`docs/notes/2026-09-22-confidence-calibration-and-the-floor.md` §4, which holds the citation and
date. No external number is introduced here; every value in this note is a preference, not a
measurement, and that is what makes it Quinn's to rule.

## 9. Rulings after T0: the cells this note left open

This note's §3 (event) covers only `obligation`/`opportunity`/`drop`, and its §4 (email) covers
only `task`/`borderline`/`event`/`opportunity`/`information`. Two things were still open when T0
went to implement the `COST` map: event's fourth verdict, `unsure` (added after this note's first
draft — see `EVENT_VERDICTS` in `judge_validate.ts`), and `completion`, the sixth email tier this
note's own §5 discusses only as a *field* (`status`) with its own two-matrix analysis, never as a
*classification* cost against the other five tiers. The stream controller ruled both sets of cells
on 2026-09-22, the same day as this note's ratification, so `score.ts`'s `COST` map could be
completed without a second round-trip. Verbatim, with the one-line reason each cell carries:

**Event `unsure` (six cells):**

- `obligation → unsure` = **3**, not 2. `unsure` never surfaces to the student today (T1's design),
  so calling an obligation `unsure` hides it exactly as completely as `drop` does — pricing it at
  the "invented obligation" 2 would score it as *milder* than a plain miss, when it is the same
  miss with a different name. Revisit to 2 once `unsure` surfaces somewhere the student can see and
  correct it — at that point it becomes P2's abstain lane, not P1's catastrophe.
- `opportunity → unsure` = **1**. P4: the item still surfaces (as `unsure` gets handled today),
  a mild confusion.
- `drop → unsure` = **1**. Same: neither direction removes anything that wasn't already headed for
  drop.
- `unsure → obligation` = **2**. P3: an invented obligation from an honest "I couldn't tell" is the
  same false-urgency cost as `drop → obligation` and `opportunity → obligation`.
- `unsure → opportunity` = **1**. P4, mild.
- `unsure → drop` = **2**. Symmetric with `obligation → unsure`'s reasoning read the other way: an
  event the truth calls uncertain, we call closeable, is close enough to inventing a false "safe to
  ignore" that it prices like the false-obligation cells, not like the mild ones.

**Email `completion` (ten cells):**

- `task → completion` = **3**, the same as `task → information`. A live task marked "already done"
  is hidden from the ranking exactly as completely as one filed as information — P1's catastrophe,
  same price, different disguise.
- `completion → task` = **2**, the same as `information → task`. A real completion signal invented
  as a task is P3's false urgency: visible, dismissible, but a trust cost.
- `completion → information` = **1**. A completion email that matches no active task by title
  writes nothing (`completion::propose_done`'s match requirement, T9) — the same silent no-op as
  filing it as `information` in the first place, so the two cost the same, P4's mild confusion, not
  P1's catastrophe: nothing that was being tracked was lost by this particular misroute.
- `information → completion` = **2**. The reverse: treating an ordinary item as a completion signal
  risks a spurious `status: done` amend proposal on some task, an invented change a student has to
  notice and reject — P3's false-urgency price, not P4's mild one.
- `completion → borderline` = **1**, `completion → event` = **1**, `completion → opportunity` = **1**.
  All three still surface the item somewhere a human can act on it — P4, mild, the same price as the
  other non-catastrophic confusions among the five original tiers.
- `borderline → completion` = **2**, `event → completion` = **2**. Same reasoning as
  `information → completion`: a real borderline or event item recast as a completion signal risks
  an invented amend proposal against some unrelated task, P3's price.
- `opportunity → completion` = **3**, the same as `opportunity → information` (P5): opportunity's
  email-tier meaning is a dated, consequential item (an application, a scholarship, a job), and
  recasting it as a completion signal both loses it from the ranking and risks a spurious proposal
  elsewhere — the worst of both, so it takes the ceiling price rather than a lesser one.
