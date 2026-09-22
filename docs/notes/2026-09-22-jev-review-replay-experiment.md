# Experiment 1 — replaying the C1b review corpus against Jev, offline

**Status: a procedure, written 2026-09-22. Not run. No Claude Code setting changed, no `.claude/`
file touched, no product code modified.** This specifies experiment 1 of
`docs/reports/2026-09-22-jev-system-one-for-development.md` ("Experiments, ranked") as something a
person can execute end to end, with the stop rules that decide it and what it costs. It needs one
thing from Quinn before it can run, named in section 2.

## What the experiment asks

*Can a typed-decision model reproduce the severity grade a human reviewer gave a code-review
finding, and the disposition the controller then ruled?*

Not "can it review code" — it cannot, and the development report settled that. This tests the one
adjacency that is real: **triage of findings that already exist**. If it works, the payoff is a
pre-sort on a long review so the controller reads the blockers first. If it does not, the shelf is
where the rest of the ideas stay.

## 1. The corpus — what we already own

Two labelled sets exist, from one stream, and they use different vocabularies. Both are already on
disk and no new labelling is needed.

**Set A — the plan review.** `docs/reports/2026-09-17-c1b-sign-in-plan-review.md` (495 lines, on
`main`). Findings carry a **three-way severity assigned by section heading**:

| class | items | n |
|---|---|---|
| Critical | C1-C6 | 6 |
| Important | I1-I8 | 8 |
| Minor | M1-M10 | 10 |
| | | **24** |

This is the better set: the classes are close to balanced, the grading is explicit, and the reviewer
committed to it in the document rather than in prose.

**Set B — the seven task reviews and the whole-branch final review**, in the worktree at
`.claude/worktrees/c1b-sign-in/.superpowers/sdd/2026-09-17-c1b-sign-in-plan/`. Findings carry
**blocking / should-fix / nit**, inline. Counted from the review documents themselves on
2026-09-22 and reconciled against the ledger's per-task summary lines in `progress.md`, which agree:

| review | blocking | should-fix | nit |
|---|---|---|---|
| task 1 | 0 | 6 | 4 |
| task 2 | 0 | 0 | 2 |
| task 3 | 0 | 2 | 3 |
| task 4 | 0 | 0 | 1 |
| task 5 | 0 | 0 | 1 |
| task 6 | 0 | 0 | 2 |
| task 7 | 1 | 0 | 1 |
| final (F1-F12) | 0 | 7 | 5 |
| **total** | **1** | **15** | **19** |

**Set B has exactly one blocking finding, and that is the experiment's hardest constraint.** A stop
rule phrased as "stop if a blocker comes back a nit" can fire on one item; it is a tripwire, not a
measurement. Set A's Critical class (n=6) is the only usable stand-in for "serious", and even six is
thin. Say this out loud in any write-up: **59 findings across both sets is a corpus that can rule a
tool out and cannot rule one in.** At n=24, a three-class accuracy estimate carries roughly ±20
points at 95% confidence — 85% and 70% are not distinguishable.

**A third label exists and is better than either.** The controller did not simply accept the
reviewers' severities; it **ruled** on them, and `progress.md` records each ruling. F6 of task 1 was
reassigned to the plan (R-C1b-exec-2); F5's check-then-act was ruled correct and left alone
(R-C1b-exec-3); F2 of the final review became a code change (R-C1b-exec-6); F5 of the final review
was handed to the controller as it lay outside the stream (H5); F8 was recorded as a follow-up. So
each finding has a **disposition** in four values: *fixed in this round*, *ruled against*, *handed
off*, *deferred*. The development report's own framing — "agreement with the controller's ruling" —
means this label, not the reviewer's word, and it is the one worth predicting: it is the decision a
triage tool would be standing in for.

## 2. The one thing that needs Quinn's word

**Our own review prose leaves this machine.** The state sent for each finding contains repository
file paths, code excerpts, function names, and in several findings a description of a security
weakness and how it could be reached. TypeSafe's self-serve posture retains input for an unstated
period (`docs/notes/2026-09-22-jev-retention-recheck.md`: the Master Customer Agreement of
2026-09-19 grants a perpetual licence to derive telemetry and monitor abuse, with no retention
period stated).

Three things follow, and they are the gate:

1. **Quinn says yes, or the experiment does not run.** Not an agent's call.
2. **Route through OpenRouter, not TypeSafe direct.** The same re-check found
   `typesafe/jev-1.13` on OpenRouter's zero-retention endpoint list with `retainsPrompts: false`,
   enforceable per request with the `zdr` parameter. That is the difference between "retained for an
   unstated period" and "retained not at all", it costs nothing, and it is the posture the product
   already uses for every judgment.
3. **Findings naming a live secret, a token, a session or a credential are excluded from the
   corpus**, not redacted. Several C1b findings discuss the session JWT, the anon key and Credential
   Manager entries. None of them contains a secret *value* — but the rule in `CLAUDE.md` is that no
   secret goes in a prompt, and the cheapest way to honour it is to drop those findings and record
   how many were dropped. Expect a handful; the corpus can afford it less than it can afford a
   mistake.

## 3. Procedure

Everything below runs from a throwaway script in the session scratchpad. **Nothing is written into
the repository, nothing under `.claude/` is read or written, no hook is installed, no settings file
is edited, and no vault is touched.**

### Step 1 — extract the labelled set

Produce one JSONL file in the scratchpad, one line per finding:

```
{ "id": "A-C1", "set": "A", "source": "docs/reports/2026-09-17-c1b-sign-in-plan-review.md",
  "text": "<the finding's body, severity words stripped>",
  "severity": "critical", "disposition": "fixed" }
```

Two extraction rules decide whether the experiment measures anything:

- **Strip every severity word from `text`, and note how hard that is here.** In the task reviews
  **the severity is the first token of the finding**: task 1's read `1. **should-fix —
  cloud/supabase/functions/account/handler.ts:148-156.**`, tasks 2, 4, 5 and 6 open `1. **Nit** —`,
  and task 7's opens `1. **[blocking]**`. The final review puts it at the other end,
  `**Severity: should-fix.**`, and the plan review encodes it positionally as the `### Critical` /
  `### Important` / `### Minor` heading above each finding. **Four different placements, and a
  naive extraction leaks the label in all four.** If any survives into the state the experiment
  measures string matching and will score beautifully while proving nothing. This is by far the most
  likely way to get a false positive, which is why step 3's leak check is not optional.
- **Reconcile the count.** The table in section 1 was counted from the review documents and agrees
  with `progress.md`. Re-derive it during extraction anyway and stop on any disagreement — a corpus
  this small cannot absorb a miscount.

### Step 2 — freeze the questions before seeing any result

Write the question text once, commit it to the scratchpad file, and **do not edit it after the first
result is seen**. With 59 items there is no room for a train/test split, so the only defence against
tuning the questions to the answers is to forbid tuning.

```
state = {
  finding: "<the finding text, severity stripped>",
  file:    "<the primary file it names, if any>",
  context: "<the task brief's one-line description of that task>"
}

questions = {
  grade:    choice over ["critical", "important", "minor"]          // set A
  blocks:   noul  "this finding must be fixed before the branch merges"
  disposition: choice over ["fix now", "rule against", "hand off", "defer"]
}
```

All three evaluate in one request at one price, which is the property being tested.

### Step 3 — establish the baselines first

Run these before touching the model. They cost nothing and they are what the result has to beat:

- **Majority class.** Always "Minor" scores 10/24 = 42% on set A. Always "nit" scores 19/35 = 54% on
  set B. A model that cannot clear these is worse than a constant.
- **Keyword.** A grep for "security", "secret", "wrong", "contradicts", "must" against the severity.
  If a five-line grep matches the model, the model is not the interesting part.
- **The leak check.** Run the corpus once **with** the severity words deliberately left in. If the
  score does not fall substantially when they are stripped, the extraction is leaking the label and
  every other number is void.

### Step 4 — run it, once

Sequentially, logging for each item: the request, the full answer including the distribution, the
wall-clock latency, and the reported token usage. One pass. No re-runs with different wording.

### Step 5 — measure

- **Agreement** with the reviewer's severity (set A) and with the controller's disposition.
- **The full confusion matrix**, not the aggregate. The aggregate hides the only cell that matters.
- **The demotion cells specifically**: Critical graded Minor, blocking graded nit. These are the
  failures that make a triage tool worse than none.
- **Calibration of the returned probability** — a reliability diagram, if 24 items can support one,
  and otherwise just the raw pairs. This is the same question the event-kind note asks of the
  product (`docs/notes/2026-09-22-jev-event-kind-experiment-design.md` §4), on a corpus we already
  own, and it is the part of this experiment most likely to teach something transferable.
- **p50 and p99 latency, and total spend.**

## 4. Stop rules

Ordered, and any one of them ends it.

1. **The leak check fails** (step 3). Stop and fix the extraction; nothing else is meaningful.
2. **Any Critical or blocking finding is graded Minor or nit.** One is enough. A triage that demotes
   a blocker is worse than no triage, because the controller would stop reading.
3. **Agreement does not beat the majority-class baseline by more than the confidence interval** —
   roughly 20 points at this corpus size. Below that the result is uninformative, and the honest
   conclusion is "we do not know", not "it nearly worked".
4. **The probability is uninformative** — if the model's confidence does not separate its right
   answers from its wrong ones at all, then even a working classifier cannot be gated, and the
   escalate-when-uncertain pattern that makes these tools safe is unavailable.

**Pass condition, stated so it cannot be moved afterwards:** beats both baselines by more than 20
points, zero demotions of a Critical or blocking finding, and a probability that visibly separates
right from wrong.

**And a pass authorises nothing by itself.** The next step would be experiment 2, the shadow-mode
`PreToolUse` gate — which edits a Claude Code settings file and sends tool inputs off the machine.
That is Quinn's decision, separately, on its own evidence.

## 4a. Prior art that changes what this experiment should be compared against

Three findings arrived after this procedure was drafted. None invalidates it; two give it a better
baseline and a defensible governance anchor.

**A commercial code-review product already ran the obvious version and it failed.** Greptile's
write-up is blunt: asking a language model to judge its own review comments produced a judgment that
"was nearly random", and it made the pipeline slow. What worked instead was not a model call at all
— per-team embeddings of upvoted and downvoted comments, blocking a new comment on cosine similarity
to at least three unique downvoted ones. Their comment mix before filtering was **19% good, 2% flat
wrong, 79% nits**, and the address rate moved from **19% to over 55% in two weeks**; a later A/B
over more than a million pull requests moved it 52% to 66%. **So the cheap baseline for this
experiment is embeddings over our own graded findings, not a constant — and it may well win.**

**The task is independently known to be learnable.** A 2026 study over **31,073 review and feedback
pairs across 10,191 pull requests and 239 repositories** found 36.4% of agent review comments
accepted and 56.3% rejected, and that "lightweight learning-based methods achieve up to 76% F1
score" at predicting rejection. That is the number this experiment's result should be read against.

**And Google's governance gives the stop rule a defensible threshold.** Their static-analysis
platform allows results shown during code review "up to 10% effective false positives", and disables
an analyser that exceeds it until the authors improve it; for build-breaking checks "the effective
false positive rate must be essentially zero". **A sorter needs no false-positive budget; a gate
needs a measured one.** Section 4's pass condition should be read in that light: this experiment
tests a sorter, and nothing in it authorises a gate.

**A ready-made harness exists and should be preferred to a bespoke script for anything that becomes
a plugin.** `claude plugin eval` runs each case with and without a plugin and reports the delta,
with graders including tool use and tool ordering. If the replay ever graduates into something
installed, that is the measurement surface, not a scratchpad script.

## 5. Cost

Jev is $0.042 per million input tokens with output free (`docs.typesafe.ai/models`, as recorded in
the assessment's sources, read 2026-09-22). The corpus is 59 findings.

| | tokens | cost |
|---|---|---|
| minimal state (finding text ~500 tokens + questions ~300) | 59 × 800 = 47,200 | **$0.0020** |
| generous state (whole review file as context, ~3,300 tokens + questions) | 59 × 3,600 = 212,400 | **$0.0089** |
| both arms of the leak check, generous state | ~425,000 | **$0.018** |

**Under two cents for the whole experiment, every arm included.** Latency: TypeSafe states 70-500 ms
end to end and an independent Claude Code plugin measured p50 ~230 ms; 59 sequential requests is
roughly 15 to 30 seconds. One caveat, **unverified**: Jev bills `state` plus every question's
instructions and criteria, so a three-question request bills more than the state alone — which
moves these figures up, never down, and they have two orders of magnitude of headroom.

The money is not the constraint. **An hour of a person's time and a 59-item corpus are the
constraints**, and the corpus is the one that cannot be bought.

## 6. What this experiment does not touch

Stated explicitly, because the value of running it depends on it staying true:

- No `.claude/` file is read or written. No hook, no settings, no subagent definition, no skill.
- No product code, no `cloud/`, no `engine/`, no `app/`, no migration, no vault.
- Nothing is committed. The script and its output live in the session scratchpad and are deleted.
- No API key appears in the repository, a log, a prompt or a test name. It is read from the
  environment, once, by the throwaway script.
- The C1b worktree is read-only input. It is not modified and not committed from.

## Sources

In this repository, read 2026-09-22: `docs/reports/2026-09-17-c1b-sign-in-plan-review.md` (sets A's
findings and their headings); `.claude/worktrees/c1b-sign-in/.superpowers/sdd/2026-09-17-c1b-sign-in-plan/`
— `progress.md` (the per-review summary lines and the controller's rulings R-C1b-exec-2, -3, -5, -6
and hand-offs H2b, H5), `final-review.md` (F1-F12 and their severity lines), and the seven
`task-N-review.md` files; `CLAUDE.md` (the secret rule, the single-user rule);
`docs/reports/2026-09-22-jev-system-one-for-development.md` (experiment 1 as originally framed, and
the p50 ~230 ms figure with its source).

External figures — the $0.042 per million input tokens with output free, the 70-500 ms latency, and
the `choice` / `noul` primitives — are carried from the Sources section of
`docs/reports/2026-09-22-jev-system-one-assessment.md`. The OpenRouter zero-retention finding in
section 2 is from `docs/notes/2026-09-22-jev-retention-recheck.md`, which carries its own URLs and
read dates.
