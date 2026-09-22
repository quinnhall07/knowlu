# Judgment quality — a plan for stream J

**Status: PLAN, written 2026-09-22. Not executed. Nothing in it has been started.** It turns the ten
ideas from the 2026-09-22 Jev research into ordered work. Every idea here needs **no new inference
provider**, and adopting Jev is explicitly out of scope: that question was closed on measured
evidence, recorded in `docs/notes/2026-09-22-confidence-calibration-and-the-floor.md` §3.

**Written to survive a context compaction.** Every task names its files, its evidence and its
decision, so a fresh session can execute from this document alone.

## The six research notes behind this plan

| note | what it establishes |
|---|---|
| `docs/notes/2026-09-22-confidence-calibration-and-the-floor.md` | the floor is mis-set; fit a scorer, do not buy a better confidence |
| `docs/notes/2026-09-22-jev-event-kind-experiment-design.md` | the three defects in the event path; the corpus does not exist |
| `docs/notes/2026-09-22-jev-what-people-built.md` | decomposition is the most replicated win; the abstain evidence |
| `docs/notes/2026-09-22-jev-review-replay-experiment.md` | the review-triage procedure, and its real baseline |
| `docs/notes/2026-09-22-jev-retention-recheck.md` | retention is no longer the blocker; OpenRouter marks the endpoint |
| `docs/notes/2026-09-22-jev-in-the-development-workflow.md` | what may and may not be wired beside Claude Code |

## What makes this plan dynamic

Three tasks are gated on a measurement rather than on a date, and the plan branches at each. **Do
not pre-commit to the branch that has not been measured yet.**

- **B1, after T3.** Does decomposing the event rules beat the single call on T2's seed? Ship it if
  yes. If no, keep the single call, record the number, and the idea is closed rather than pending.
- **B2, after T6 has data.** Is our `confidence` calibrated, and does it rank? Three outcomes, three
  different next steps, set out under T6.
- **B3, after T0 and B2.** What does the floor become? Cannot be answered before both.

**A fourth dependency is external and not in our gift:** T6 and T7 need real corrections, which need
real accounts. Until then they are written and parked, not run. Say so rather than letting them look
late.

## Ordering, and why

```
T0 (Quinn)  cost matrices ─────────────────────────────┐
                                                       ├──> B3 ──> T5  the floor's value
T6  calibration harness (written now, parked) ──> B2 ──┤
                                                       └──> T7  the fitted scorer
T1  abstain verdict + the re-ask fix   (no data needed)
T2  synthetic event seed ──> T3  decomposition ──> B1
T4  email `due` reference date         (no data needed)
```

**T1, T2 and T4 touch disjoint files and can run in parallel.** T3 needs T2. T5 and T7 are blocked
on data that does not exist yet.

## Prerequisite check before any dispatch

1. **Stream collision.** These files are C2's, and C2 is merged. Confirm no active stream owns
   `cloud/supabase/functions/_shared/judge_*.ts`, `cloud/eval/**`, `engine/src/events.rs` or
   `engine/src/eventledger.rs` before starting. `HANDOFF.md` §2 is the register.
2. **Green baseline.** `cargo test --workspace` at 0 warnings, and the Deno checks, before the first
   commit. The one accepted warning line is the app's `.rsrc` linker message.
3. **A worktree**, per the repo's normal practice. Branch `j-judgment-quality`.

---

## T0 — the cost matrices *(Quinn's, blocking, about twenty minutes)*

**What.** Write down the full cost of every confusion for the event kind, three by three, and for
the email kind, five by five.

**Why it is first.** `cloud/eval/score.ts:29-38` already holds most of the event matrix:
`obligation->drop` costs 3, `task->information` costs 3, `information->task` costs 2, and every
other named mistake costs 1. What is missing is the unnamed cells and an explicit statement that the
diagonal is zero. Every threshold in this plan is derived from these numbers, so guessing them
propagates into T5 and T7.

**The one thing that makes it a twenty-minute job, not a research project.** These are not empirical
quantities. They are a statement of what Quinn cares about, in units of each other. "Missing a real
obligation is three times worse than a false alarm" is already the ruling; the rest is the same
question asked five more times.

**Output.** A short ruling in `HANDOFF.md`, and the matrix into `score.ts`'s `COST` map with the new
cells. **Not** a model change.

---

## T1 — the abstain verdict, and the re-ask defect

**Two defects, one fix, and they are the same shape.**

**Defect A: the event verdict cannot decline.** `EVENT_SCHEMA`
(`cloud/supabase/functions/_shared/judge_prompts.ts:50-58`) enumerates exactly `obligation`,
`opportunity`, `drop`. Whether an event obliges a particular student usually is not in the event
text. The model must answer anyway.

**Defect B: a declined answer is asked again forever.** `engine/src/events.rs:439-444` pushes a line
and continues when the service returns no verdict, writing nothing. `events.rs:408` filters the
roster to uids whose ledger entry has no verdict. So the item returns next slot, and the slot after,
at full price. For a real obligation the student sees nothing, which `score.ts` prices as the most
expensive single error in the suite.

**The evidence for fixing A.** An independent audit this week measured a decision model with and
without an "unknown" option: accuracy on unanswerable items moved from 0.950 to **0.000** and
calibration error from 0.023 to **0.793**. A pre-registered benchmark found a router answering one
class 120 times out of 120, scoring zero on the hard slice. Both are cited in
`docs/notes/2026-09-22-jev-what-people-built.md` §3.

**CHECKPOINT J-1, Quinn's, before any code.** Two designs, and I recommend the first.

- **(a) A fourth verdict word, `unsure`.** Added to `EVENT_VERDICTS` (`judge_validate.ts:20`) and to
  `eventledger::VALID_VERDICTS`. `record_verdict` writes it, so the uid has a verdict and is never
  re-asked, which closes defect B for free. The surface does not show unsure items, or shows them in
  a separate low-priority group.
- **(b) Leave the verdict set alone and record the abstention in a new ledger field.** More code,
  and it invents a second mechanism for something the verdict field already expresses.

**The contract consequence Quinn should weigh, and it is mild.** The ledger is a vault contract.
Adding a word is additive, never a rename, so it does not break the rule in `CLAUDE.md`. An older
engine reading a newer vault would find `unsure` outside `VALID_VERDICTS` and leave the entry's
verdict unset, which means it would re-ask the item. That is graceful degradation, not corruption,
and it is worth stating in the commit message.

**Test first.** A test that a below-floor event is not re-asked on the next run is the one that
fails today and must pass after.

**Files.** `judge_prompts.ts`, `judge_validate.ts`, `engine/src/events.rs`,
`engine/src/eventledger.rs`, and the surface's event rendering. **Not** a migration.

---

## T2 — the synthetic event seed

**What.** Twenty to forty hand-written cases in `cloud/eval/seed/events.jsonl`.

**Why.** `cloud/eval/seed/` holds only a README and is empty **by ruling R-C2-E12 of 2026-09-14**,
which declined the one read that would have filled it. `cloud/eval/run_eval.ts`'s own header says
"Today the corpus is empty, so the gate costs nothing." `cloud/eval/thresholds.json` says every
number in it is unmeasured. **The eval gate has never scored anything.** Until it does, no pin
decision and no prompt change can be defended.

**The shape is already specified** and the loader enforces it. `cloud/eval/schema.ts` requires four
top-level keys, a `seed-` prefixed id, `request` exactly as `engine/src/cloudmodel.rs`'s
`event_request` builds it, and `theirs: {verdict}`. `scrubViolations` refuses any `@` token, any
URL, any Windows or POSIX home path, any run of seven or more digits, and any string over 200
characters. `cloud/eval/seed/README.md` carries a worked example of an event case.

**Coverage the cases must have**, one or more each:
- each of the two stated drop rules: an audience of faculty, staff, alumni or graduate students; a
  standing exhibit, an office-hours block or a recurring drop-in
- a clear obligation, a clear opportunity, a clear drop
- **the unanswerable case** that T1 adds a verdict for, if J-1 chose design (a)
- an event whose description is empty, and one clipped at the body limit
- a Spanish-language description, since the task kind reads Spanish coursework and no Spanish figure
  exists for any candidate model

**Honest limit, to be written into the file's own header.** These measure whether a model applies
our stated rules. They are not the real distribution, and at this size a three-class estimate
carries roughly twenty points of error at ninety-five percent confidence. **They can rule a change
out. They cannot rule one in.**

**Then replace the thresholds.** Run `run_eval.ts` once and set `event.weighted_exact_min` from what
was measured, one clear step below it, and delete the `_note` key or update it to say when.

---

## T3 — decompose the event rules *(depends on T2)*

**What.** Turn the two drop rules in `systemTemplate("event")`
(`judge_prompts.ts:108-119`) from prose the model must hold in mind into their own scored schema
fields, and combine them in code.

**Why this is the highest-value model change available.** Four independent measurements this week
found large gains from exactly this move. From
`docs/notes/2026-09-22-jev-what-people-built.md` §"The three lessons":

| task | one question | decomposed |
|---|---|---|
| phishing detection | 62.6% | 95.1% |
| shell-risk gate, false positives | 14.5% | 1.8% |
| skill routing, recall at 1 | 9.0% | 81.0% |

**And the counter-case, which is why this task is measured and not assumed.** One team found twelve
weighted dimensions produced a false-positive rate roughly twenty-five times worse than a single
call on a task where the single call was already strong. Their conclusion: decomposition helps when
the single call is weak, and costs you elsewhere.

**Three further payoffs beyond accuracy.** A drop that fires on a named rule can carry a **templated
`why` that states the actual reason**, which is more honest than a generated sentence. It becomes
overridable by a promoted rule in `judge_rules.ts` without touching a prompt. And it decouples the
rules from the verdict, so a rule can change without a `prompt_version` bump rippling through every
kind.

**Important: this runs on the model already pinned.** Granite 4.2 8B answers extra schema fields
today. No provider, no migration, no privacy-page change, no new party.

**BRANCH B1.** Score the decomposed prompt and the current prompt against T2's seed on
`weighted_exact`. Ship the decomposition only if it wins by more than the seed can explain by noise.
If it loses or ties, **record the number in the note and close the idea** rather than leaving it
open.

**`prompt_version` and `prompt_hash` move**, which is the point of having them.

---

## T4 — the email `due` reference date

**What.** `judge_prompts.ts`'s email template asks the model to resolve a relative deadline against
the Date line. The 2026-09-16 scoping note already called this "the single most dangerous line",
because a mis-resolved "by Friday" becomes a well-formed wrong date the validator writes into the
vault silently.

**What changed since: the fix is now corroborated.** The pattern found independently this week is to
have the model extract the fields and let ordinary code do the arithmetic. One practitioner reported
a model getting a business-day calculation wrong while extracting the fields correctly twelve times
out of twelve. The vendor documentation for every model in this class says the same thing about
dates. **Replace the prompt rule with a deterministic TypeScript date resolver** and keep the model
on extraction.

**Independent of everything else in this plan**, and independent of Jev. Can run in parallel with
T1 and T2.

---

## T6 — the calibration harness *(write now, park until there is data)*

**What.** A query and a metrics script that answers: does our `confidence` mean anything?

**The data is already logged and needs nothing new.** `judgments` carries `confidence`, `outcome`,
`cause` and — because `fieldsOf` (`judge_pipeline.ts:71`) drops only `confidence`, `why`,
`importance_reason` and `title` — the verdict itself in `fields`. `corrections` carries
`judgment_id` and `theirs`. The join gives pairs of claimed probability against observed
correctness, per kind.

**Compute, in this order, and stop early if the first answer is bad:**

1. **The count of distinct confidence values.** Three or fewer and the field is decoration. One
   study found a model emitting exactly three values, and its ranking ability was near useless.
2. **AUROC** for "was this judgment wrong". At or above 0.80 the field is a usable ranker; below
   0.70 the gate is cosmetic whatever its threshold.
3. **Calibration**, only if 1 and 2 pass. Use the kernel-smoothed estimator below a few hundred
   labels, because it removes the bin-count decision that is the main source of small-sample
   nonsense. A bin under about thirty items is decoration: at three-quarters accuracy its standard
   error is near nine points.

**BRANCH B2, three outcomes:**
- **The field does not rank.** The floor is theatre. Replace it with the rules tier and a fixed
  policy, and T5 and T7 both vanish.
- **It ranks but is miscalibrated.** Fit a map on our own logs. **Dirichlet or vector scaling** is
  the right default for a three-way verdict; per-class Platt if only one number per class is
  available. Not isotonic below about a thousand labels, though one recent study disputes that and
  the answer is to fit both and compare held out.
- **It ranks and is roughly calibrated.** The worry dissolves; go straight to T5.

**Per kind, always.** Miscalibration direction differs by question type in every study that split
it. **And re-fit on every prompt change** — one study saw calibration error range from 0.064 to
0.160 across eleven phrasings of the same question. `prompt_version` and `prompt_hash` are already
logged per judgment, so the invalidation is detectable.

**The blocker, stated plainly.** There are no real corrections yet. Write the harness, test it on
synthetic rows, and park it. **Do not report this task as late; report it as waiting on users.**

---

## T5 — the floor's value *(blocked on T0 and B2)*

**The arithmetic, for when both land.** With a zero diagonal, the cost-optimal gate for a binary
decision is `C_FP / (C_FP + C_FN)`. At the ruling of three against one that is **0.25**, not 0.6. A
gate at 0.6 is optimal for a cost ratio of about two to three, which is a product that fears false
alarms more than misses. Ours is the opposite.

**Two qualifications that stop this being a one-line change, and both must be handled.**

1. **The number must be calibrated first**, which is B2. Applied to a raw self-reported field, 0.25
   is as arbitrary as 0.6.
2. **Our floor abstains rather than predicts.** The comparison is not act against do-not-act, it is
   act against fall back, and the fallback is not free. After T1 the fallback is an `unsure` record;
   before T1 it is silence, which costs what the scorer prices at 3.

**So the instrument is a risk-coverage curve**, not a threshold argument: accuracy among the
judgments kept against the fraction kept. Pick the point on the curve.

**One uncomfortable external finding to test against.** A pre-registered benchmark of 5,721 calls
found accuracy flat from 0.50 to 0.95 and only jumping at 0.99. If our curve has that shape, a gate
anywhere in the middle is doing nothing at all, and the honest answer is a two-band policy rather
than a better number.

---

## T7 — the fitted scorer *(blocked on B2, the deepest idea here)*

**What.** A small model over features we already log, trained on corrections to predict "was this
judgment wrong", replacing the model's own `confidence` as the gate input.

**Why it beats buying a better confidence, replicated from 2020 to 2026.** One study answered 56% of
questions at 80% accuracy where the model's own probabilities answered 48%. Another moved area under
the accuracy-coverage curve and AUROC by several points. A 2026 production system went from clearing
under seven percent of fields on verbalized confidence to clearing half to three-quarters at the
same error target.

**The inputs already exist.** `judge_rules.ts`'s `features()` extracts `created_by`, `title_prefix`,
`organizer`, `source`, `series_uid`; `judgments.fields` carries them alongside the verdict; and the
stated confidence is a feature rather than the answer.

**The practitioner version of the same finding**, from a phishing benchmark that went from 62.6% to
95.1% by decomposing and fitting a regression: *the accuracy is not the model, it is the model plus
your labelled data plus a regression you maintain.* **That sentence is the thesis of this whole
plan.**

---

## E — the experiments, each with its stop rule

These are **planned work, not optional extras.** Each has a stop rule written before it runs, so a
disappointing result closes the question rather than leaving it open. A result that closes an idea
is a good outcome and should be written up as one.

### E1 — the decomposition A/B *(part of T3, the primary experiment)*

Score the decomposed event prompt and the current one against T2's seed on `weighted_exact`.
**Stop rule:** ship only if the decomposition wins by more than the seed size can explain. Twenty to
forty cases carry roughly twenty points of error, so a two-point win is noise. If it loses or ties,
record the number and close the idea.

### E2 — the review-triage replay *(specified, needs one word from Quinn)*

Full procedure in `docs/notes/2026-09-22-jev-review-replay-experiment.md`: 59 graded findings from
the C1b corpus, replayed offline against their reviewer severity and the controller's disposition.
Costs under two cents and under a minute.

**Three things it needs, in order.** Quinn's word that our own review prose may leave the machine,
which is the gate. Route through OpenRouter rather than TypeSafe direct, because that endpoint is
marked as retaining nothing. And drop, rather than redact, any finding that names a secret, a token
or a session.

**Its baseline changed after the note was written** and §4a of that note carries the correction: the
comparator is no longer a constant but embeddings over our own graded findings, which a commercial
review product used to move its address rate from roughly a fifth to over half. **The cheap baseline
may simply win, and that is a valid and useful result.**

**Stop rules:** the leak check first, because the severity word sits in four different placements and
a naive extraction scores beautifully while proving nothing. Then any demotion of a Critical or
blocking finding ends it. Then failure to beat the majority class by more than the confidence
interval means the answer is "we do not know", not "it nearly worked".

### E3 — the calibration measurement *(T6, written now, run when there is data)*

The query, the metrics and the three branches are under T6. **Write and test it against synthetic
rows now** so that the day real corrections exist it is one command, not a project.

### E4 — plugin eval, only if a Claude Code change is ever considered

`claude plugin eval` runs each case with and without a plugin and reports the delta, with graders
including tool use and tool ordering. **If anyone ever proposes a hook or plugin, this measures it
rather than arguing about it.** The standing recommendation today is not to install one, and
`docs/notes/2026-09-22-jev-in-the-development-workflow.md` §6 says why.

---

## The open lane — research, experiments and ideas beyond this plan

**This plan is a floor, not a ceiling.** Quinn has said explicitly that subagents are welcome to do
more research, run more experiments, and propose better ideas for optimising the app. Nothing below
needs a new plan document to get started.

**What is invited.**

- **Research briefs on anything in the judgment path**: prompts, schemas, the rules tier, the
  promotion job, the eval harness, the ranking inputs the judgments feed.
- **New experiments**, including ones that contradict this plan. If the measurement says T3 is the
  wrong idea, that is the plan working.
- **Optimisation ideas for the app itself** beyond judgment quality: the scheduler, the surface, the
  wizard, onboarding, the read model, cost, startup time, the slot's wall clock.
- **Re-checks of anything here that has a date on it.** The Jev material is a week old and several
  claims are marked unverified or disputed in the notes. Those flags are invitations.
- **Two ideas already recorded and unowned**, either of which a subagent may pick up: pre-selecting
  the wizard's course mapping from the vault's slugs, and weighting rule promotion by the confidence
  already stored in `judgments`.

**How a proposal lands.** A research pass writes a note in `docs/notes/` or a report in
`docs/reports/`, dated, LF, every number carrying a URL and a date, anything unverified marked
unverified. If it proposes work, it names the task, the stop rule and what it would cost. Quinn
rules at the checkpoint. **A note that closes an idea is worth as much as one that opens it.**

**The standing rules a proposal must respect**, which are the repo's, not this plan's:

- No single-user assumptions. Nothing names a person's vault, machine, account or credential.
- No secret in the repo, a log, a prompt or a test name. Keys come from the environment.
- `rank` never calls a model. Judgment is the separate `judge` command.
- The eight frozen fixtures are never regenerated; the three Rust-generated read-model references
  only in a commit whose diff shows the change and whose message says why.
- Deterministic checks stay deterministic. Arithmetic is not a job for a model.
- Tests first, in the foreground, at zero warnings.

**Three boundaries that still need Quinn rather than a subagent.**

1. **Anything under `.claude/`** — a hook, a plugin, a settings file, a subagent definition.
2. **Anything that sends repository content, review prose or tool inputs off this machine.** E2 is
   gated on exactly this.
3. **Adopting a new inference provider**, which is a privacy-page version bump, an email to every
   account and an in-app yes before the first request. Jev specifically was closed on measured
   evidence, and re-opening it needs new measurement rather than new enthusiasm.

## Fidelity ledger

| idea | task | status |
|---|---|---|
| cost matrices | T0 | Quinn's, blocking |
| abstain verdict | T1 | needs CHECKPOINT J-1 |
| below-floor re-asked forever | T1 | same fix |
| synthetic event seed | T2 | ready |
| decompose the event rules | T3 / **E1** | ready once T2 lands |
| email `due` reference date | T4 | ready, independent |
| measure our own calibration | T6 / **E3** | write now, run when users exist |
| the floor's value | T5 | gated on T0 and B2 |
| the fitted scorer | T7 | gated on B2 |
| review-triage replay | **E2** | specified, gated on Quinn's word |
| plugin eval, if a Claude Code change is proposed | **E4** | standing, not scheduled |
| wizard course mapping, promotion weighting | open lane | unowned, may be picked up |
| anything not listed here | open lane | invited |

## What Quinn is asked for, in order

1. **T0**, the cost matrices. Blocking, and about twenty minutes.
2. **CHECKPOINT J-1**, the abstain design. Recommendation: the fourth verdict word.
3. **E2's gate**: may our own review prose leave this machine? One word, and it unblocks an
   experiment costing under two cents.
4. Later, at **B1** and **B2**, a look at a measured number rather than a decision in advance.

Nothing else in this plan needs a ruling, and nothing in it needs a new provider, a migration, a
privacy-page change or a new party on the sub-processor list.

**And the plan is deliberately incomplete.** The open lane above is a standing invitation, not a
formality: a subagent that returns with a better idea than anything in this document has done the
job right, and the ordering here should be revised rather than defended.
