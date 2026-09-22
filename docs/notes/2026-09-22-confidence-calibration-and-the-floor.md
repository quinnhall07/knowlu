# What `confidence` is worth, and why the 0.6 floor is on the wrong side of even

**Status: research note, 2026-09-22. Nothing decided, no code changed.** This is the companion note
`docs/notes/2026-09-22-jev-event-kind-experiment-design.md` §4 promised. It answers the question
that note left open — whether a self-reported `confidence` field means anything, and whether a
calibrated model would fix it — from the literature and from two independent measurements of Jev
published in the week after its launch. Not legal advice; not a claim about our own numbers, which
have never been measured.

**It is also the most actionable thing in the Jev research, and it has nothing to do with adopting
Jev.** Three of its four findings apply to the code as it stands, need no new provider, and cost
nothing.

## The four findings

1. **Jev's `confidence` is not a second opinion; it is a statistic computed from the distribution.**
   TypeSafe's confidence page says exactly that and no more: a flatter distribution means lower
   confidence, and the formula is deferred to "a separate cookbook" that does not yet exist. So the
   event-kind note's open question — which of the two returned numbers carries the calibration —
   **is answered in substance: there is no independent second signal to prefer.** Two further facts
   follow from the published response shape, and both matter: **a noul answer carries no
   `confidence` field at all**, only `choice` and `score` do; and because the formula is unpublished,
   **`confidence` cannot be reproduced or re-derived — only your own statistic recomputed from
   `probabilities`.** See §1 for a conflict in the sources about the formula.
2. **Independently measured, Jev's `choice` confidence is close to useless below the top band.** On
   8,801 constructed examples, choice accuracy "hovered 50–57% across all stated confidences
   50–95%; only the top band (95%+) proved informative at 90.2%". The event kind is a choice. **The
   one thing we would have adopted Jev for, measured, does not work at our operating point.**
3. **Our 0.6 gate sits in the worst band of a typical confidence distribution, and points the wrong
   way.** With the cost asymmetry the product already encodes, a calibrated gate belongs far below
   the midpoint, not above it. Section 4 does the arithmetic.
4. **Calibration and discrimination are different properties, and our field may already be fine at
   the second one.** A badly calibrated score can still rank well, and a gate needs ranking plus a
   correctly placed cut. Measuring which we have costs a few hundred labels we are already
   collecting.

## 1. What `confidence` is, in each system

**Ours** is a number the model writes into a JSON field because the schema asks for one
(`judge_prompts.ts:50-58`, `confidence: { type: "number" }`; the system template says only
"- confidence: 0 to 1."). It is a token sequence, not an estimate. Nothing in the pipeline has ever
checked it against an outcome.

**Jev's** is a statistic over the returned distribution. **The exact formula is disputed in my
sources and should be treated as unverified.** One research pass reported TypeSafe's documentation
giving `confidence = (K · p_max − 1) / (K − 1)` for a K-option choice, and an independent study
deriving `2 · p_top − 1` for the binary case, which is that formula at K=2. A second pass, reading
`docs.typesafe.ai/confidence.md` directly, found no formula at all — only that confidence is "a
statistic computed from the probability distribution", with the computation deferred to a cookbook
TypeSafe has not published. **Do not rely on the formula.** What is not disputed is the substance:
it is a function of the distribution, not a second judgment.

The practical consequence is a trap either way. **A threshold does not transfer between systems, and
TypeSafe's own documentation says it does not transfer between its own question types.** Their
example: the same question asked as a noul returned 0.22 while the equivalent yes/no choice returned
0.01 for `yes`. And a question with its own negation, asked as two nouls, returned 0.72 and 0.47 —
summing to 1.19. Their instruction is explicit: "Don't carry a threshold tuned on a Noul over to a
Choice, and don't hold the model to arithmetic identities between separate questions."

## 2. Is self-reported confidence calibrated? The literature says no, consistently

Direction and magnitude agree across the field. Verbalized confidence is miscalibrated and skewed
overconfident:

| study | finding |
|---|---|
| Xiong et al., ICLR 2024 (arXiv:2306.13063) | Vanilla verbalized confidence, average expected calibration error: GPT-4 **0.180**, GPT-3.5 **0.377**, LLaMA-2 **0.436**, GPT-3 **0.520**. Values cluster "between 80% and 100%, often in multiples of 5" — the models imitate how people talk about certainty. |
| Groot & Valdenegro-Toro, TrustNLP@NAACL 2024 (arXiv:2405.02917) | Across six models: "both LLMs and VLMs have a high calibration error and are overconfident most of the time." |
| Chhikara, arXiv:2502.11028v3 | GPT-4o on SimpleQA: calibration error **0.450**; smaller LLaMA variants approaching 0.8. |
| Sanz-Guerrero et al., arXiv:2606.03437 | Models are up to **26% more confident** in their own answers than in identical answers attributed to a user. Instruction tuning alone adds **13.1%** calibration error. |
| GPT-4 Technical Report (arXiv:2303.08774), Figure 8 | "the pre-trained model is highly calibrated… **The post-training hurts calibration significantly.**" |
| Nyckel production study, 12 text-classification datasets | Raw self-reported confidence: calibration error **≈45%**. After a standard post-hoc fit: **8%**. On some datasets the raw relationship was *inverted* — higher stated confidence, lower accuracy. |

**One point is genuinely unsettled** and should not be quoted either way: whether verbalized
confidence beats the model's own token probabilities. Tian et al. (arXiv:2305.14975, EMNLP 2023)
found verbalized confidence better calibrated for RLHF'd models, "often reducing the expected
calibration error by a relative 50%"; Xiong et al. found it badly calibrated in absolute terms. Both
can be true, because post-training damages logprob calibration and beating a damaged baseline is a
low bar.

**The counterweight, and it matters.** Calibration is not the only useful property. In the closest
published analogue to this product — Khan Academy grading 2,100 high-school maths decisions with
small models (arXiv:2604.19781) — self-reported confidence achieved **AUROC 0.857** for "is this
judgment wrong" on one model, 0.757 on a second, and 0.678 on a third that emitted only **three
distinct confidence values**. So a self-reported field ranges from a genuinely useful ranker to
decorative, **depending on the model**, and the cheap way to find out which we have is to compute
AUROC on a few hundred labels. Our conclusion in the 09-16 capability note — "self-reported
confidence tracks commitment, not correctness" — is right about calibration and overstated about
ranking.

## 3. Jev, measured independently — and why this closes the event case

Two studies, both from the week after launch, both on `jev-1.13.0`. They are careful hobbyist work,
not a literature, and they agree.

**Study A — 8,801 constructed examples, 60/40 split, 11 question phrasings.**

| | noul | choice (binary) |
|---|---|---|
| raw calibration error | 0.117 | 0.115 |
| mean stated confidence vs actual accuracy | 79.0% vs 72.3% | 91.4% vs 76.1% |
| after isotonic regression | **0.008** | — |

The reliability table is the single most useful artefact in this research:

| stated band | n | mean stated | **actual accuracy** |
|---|---|---|---|
| 50-60% | 1,617 | 55.1% | **47.6%** |
| 60-80% | 2,703 | 70.7% | **54.3%** |
| 80-90% | 1,605 | 85.5% | 80.0% |
| 90-95% | 772 | 93.1% | 95.9% |
| 95-100% | 2,104 | 97.8% | **100.0%** |

Read the shape. **The middle band is where the number lies most**: 70.7% claimed against 54.3%
observed, barely better than a coin. The top band is trustworthy. The bottom band is *under* its own
stated value. **A gate at 0.6 sits precisely in the worst region of this curve.** The authors'
own recommendations are blunt: treat noul above about 90% as reliable, route the middle to review,
**"don't rely on Choice's raw confidence"**, and **"calibrate on your own labeled data before
setting any threshold."**

Against a Llama-3.1-8B baseline on 1,000 examples: Jev noul accuracy 0.743 and AUROC 0.828 against
0.722 and 0.719; after isotonic calibration, calibration error 0.022 against 0.021. **After
calibration the two are indistinguishable on honesty** — Jev's edge is discrimination and price, not
calibration, and calibration is what we would have been buying.

**Study B — 4,621 calls, ~$0.06 total, run 2026-09-19, and the one that transfers.** It separates
in-distribution from out-of-distribution:

| | public benchmarks (OpenBookQA, CommonsenseQA, HellaSwag) | synthetic out-of-distribution (900 rule-generated tickets) |
|---|---|---|
| accuracy | 86.1-94.2% | 75.1% |
| calibration error | 0.024-0.032 | **0.107** |
| ratio to the study's own noise floor | 1.0-1.7× | **4.4×** |

The author's conclusion, verbatim: on public benchmarks "the probabilities needed essentially no
correction"; on the out-of-distribution task, **"Jev is as accurate as a strong zero-shot model can
be — and confidently wrong where the rule is unknowable."**

**That is our task, structurally.** The benchmarks where calibration held are ones the author notes
likely appeared in training data. The task where it broke was one whose label depended on an
organisational policy absent from the input. "Is this campus event an obligation **for this
student**" is exactly that: the answer depends on their programme, their advisor and their
commitments, and often is not in the event description at all. **Calibration is a property of a
model on a distribution, not a property of a model.** Buying a model that advertises calibration
does not buy calibration on our data.

**Study C — a third, independent leaderboard, and the most damaging result of the three.**
`jevals.com` states it is "not affiliated with TypeSafe AI" and runs seven models over three suites,
300 items each, five repeats, publishing a 10-bin calibration error per row:

| suite | model | accuracy | **calibration error** | $/1k | p50 |
|---|---|---|---|---|---|
| PubMedQA (noul) | Gemini 3.8 Flash | 0.925 | **0.0198** | 0.803 | 1,854 ms |
| | **Jev** | 0.913 | **0.0504** | **0.029** | **438 ms** |
| Banking77 (choice) | Gemini 3.8 Flash | 0.846 | **0.0319** | 1.368 | 1,636 ms |
| | **Jev** | 0.797 | **0.0981** | **0.043** | **467 ms** |
| HelpSteer2 (score) | **Jev** | 0.413 | **0.1966** | 0.036 | 478 ms |
| | label prior | 0.417 | 0.0000 | 0 | — |

**Jev is beaten on calibration error by a chat model's ordinary verbalized probabilities, on all
three suites.** The "calibrated by construction" story does not show up as a better number than the
self-reported field we already have. Its real edge in this data is price and latency: roughly 19 to
32 times cheaper and 3 to 4 times faster. And on the `score` suite nothing beats the label prior —
accuracy 0.413 against a base rate of 0.417, the site's own note being that no model clearly beats
guessing yet.

A fourth measurement, from the Kev project's own harness, puts a number on the failure mode study B
described. On a suite whose deciding evidence was deliberately removed, it reports how often each
model still answers at 0.9 confidence or above: **Kev-9B 0%, Jev 9%, Kev-8B 26%.** Nine percent of
unanswerable questions answered with high confidence is the behaviour that would quietly drop a real
obligation.

**Study D — the one academic evaluation, and the fairest statement of the trade.** Ibrahim & Zaki,
*Evaluating Decision Models for Text Annotation in Computational Social Science*, arXiv:2609.24574,
submitted 2026-09-21. It mirrors an established annotation benchmark over **18 tasks and 7,977
items**, comparing "the first commercial decision model and two open-weight counterparts against 19
frontier and open-weight language models". It does not name the vendor, so the identification is
strong but **unconfirmed**. Verbatim:

> "The decision model trails the per-task best LLM on 14 of 15 evaluation tasks, with a median
> deficit of **11.6 macro-F1 points**, at a median **44 times lower** measured cost. Its confidence
> is **better calibrated than the verbalized confidence of 16 of the 19 LLMs**, yet three frontier
> models show lower median calibration error (0.157 against 0.066). While items above 0.9 confidence
> are typically labeled accurately (median accuracy 0.815), on one task, empathy in peer-support
> dialogues, **the model reports high confidence while performing near chance**."

**This is the honest version of the whole story and it reconciles study C.** The calibration
advantage over self-reported confidence is real, and it holds against most models — but not against
the best, which is exactly what study C's leaderboard showed when Gemini 3.8 Flash beat Jev on
calibration error across all three suites. Meanwhile the accuracy deficit is large, the cost
advantage is large, and **one task in eighteen broke silently while still reporting high
confidence.** For a product where a missed obligation is the failure we price at 3, a silent break
on one category of item is the specific risk, and no aggregate number would have warned us.

**And TypeSafe's own jaggedness page names two of our three kinds.** It states that
`jev-1.13`'s "score levels are weak in numerical calibration" and advises against using score to
compute the exact magnitude of a number — that is the task kind's `effort_hours`. It states that
"accuracy falls as the state grows with content unrelated to the decision" — our event state carries
1,200 characters of description plus the student's interests. And it repeats the date weakness that
already ruled out the email kind.

**Conclusion for the event kind: the case is closed, and not on privacy.** The one capability worth
amending R8 for, measured independently, does not hold at our operating point.

## 4. The 0.6 floor points the wrong way

**Elkan's theorem** (IJCAI 2001) gives the cost-optimal threshold for a binary decision. With a zero
diagonal it collapses to:

```
t* = C_FP / (C_FP + C_FN)
```

The derivation is one line. Let `p` be the probability the positive case is true. Acting costs
`(1−p)·C_FP` in expectation; not acting costs `p·C_FN`. Act when `p ≥ C_FP / (C_FP + C_FN)`.

**Our cost asymmetry is already written down.** `cloud/eval/score.ts:29-38` prices
`obligation->drop` at **3** and every other named event mistake at **1**. Substituting:

```
t* = 1 / (1 + 3) = 0.25
```

**A gate at 0.6 is the optimal gate for a cost ratio of about 2 : 3** — that is, for a product that
fears false alarms *more* than misses. Ours fears misses three times more. The floor was picked by
intuition and it is on the wrong side of even.

**Two qualifications, and they are not decoration.**

1. **The number must be a calibrated probability before the formula means anything.** Applied to a
   raw self-reported field, 0.25 is as arbitrary as 0.6. Calibrate first, then compute.
2. **Our floor is an abstention, not a negative prediction**, so the comparison is not "act versus
   don't act" but "act on the model's answer versus fall back", and the fallback is not free. **For
   the event kind the fallback is close to the worst outcome we price.** A below-floor event writes
   no verdict (`engine/src/events.rs:439-444`), and `judge_roster` filters the roster to uids with
   no verdict (`:408`), so the student sees nothing at all — which for a real obligation is
   indistinguishable from `obligation->drop`, the cell costing 3. **The floor's abstention on an
   obligation costs what the product's own scorer calls its most expensive single error.** That is
   reasoning from our cost table, not a measurement, and it is the argument for treating the floor
   as a live defect rather than a tuning parameter.

The right instrument is a risk-coverage curve: accuracy among the judgments kept, against the
fraction kept. Pick the point, do not argue about the threshold.

**For the three-way event verdict and the five-tier email decision, the binary formula does not
apply.** The general rule is to choose the action minimising `Σ_y P(y|x) · C(a, y)`, which needs a
full K×K cost matrix rather than one ratio. `score.ts`'s `COST` map is already most of one. Finishing
it is a twenty-minute product conversation and it retires every future threshold argument.

## 5. What a given number of labels buys

Per decision kind, always — the two Jev studies found the miscalibration *direction* differs by
question type, and per-model optimal thresholds in the Khan Academy study were 0.77, 0.80 and 0.99
for three models on the same task.

| | **50 labels** | **200 labels** | **1,000 labels** |
|---|---|---|---|
| what you can honestly conclude | whether the field is degenerate | direction and rough size of miscalibration; whether it ranks | a real calibration-error estimate, a usable reliability diagram, a defensible threshold |
| metric to compute | mean stated confidence against observed accuracy, as one pair. No bins | smoothed calibration error, or 3-5 equal-mass bins with a stated caveat | 10 equal-mass bins, debiased estimator, plus Brier and AUROC |
| discrimination check | count distinct confidence values; three or fewer means the field is noise | **AUROC** — at or above 0.80 is a usable gate, below 0.70 the gate is cosmetic | AUROC with a bootstrap interval, and the risk-coverage curve |
| calibration map to fit | none; fitting on 50 is not worth it | temperature or Platt; Dirichlet or vector scaling for the three-way | fit both Platt and isotonic, compare on held-out data |
| how to set the threshold | do not tune it; widen to a three-band policy instead | calibrate, then **derive** it from the cost matrix | derive, then validate with five-fold cross-validation |

Two facts that bound the small end. Equal-mass bins are less biased than equal-width ones, and the
standard estimator is biased worst for well-calibrated models — at 200 samples a measured 12% "could
either correspond to 5% or 8%". And a bin with 25 items has a binomial standard error near 9 points
at 75% accuracy, so **a bin under about 30 items is decoration**. Fewer bins is the correct response
to fewer labels; the kernel-smoothed estimator removes the bin choice entirely and is the right tool
below a few hundred.

**The cheapest labels we will ever get are user corrections, and the product already collects
them.** `corrections` carries `judgment_id` and `theirs`; `judgments` carries `confidence`,
`outcome`, `cause` and — because `fieldsOf` (`judge_pipeline.ts:71`) drops only `confidence`, `why`,
`importance_reason` and `title` — the verdict itself in `fields`. A join gives exactly the pairs
every row of the table above needs. Nothing new has to be built.

## 6. The fixes, none of which needs a provider

All operate on logged pairs and fit a one-dimensional map from the reported number to a real
probability.

| method | fits | labels | note |
|---|---|---|---|
| temperature scaling | one scalar | hundreds | never changes the predicted class, only the confidence |
| Platt scaling | two scalars | hundreds | robust at small n; works on a reported number when logits are unavailable |
| Dirichlet / vector scaling | a multiclass map | hundreds to ~1,000 | **the right default for a genuinely three- or five-way decision** — it calibrates the whole simplex |
| isotonic regression | any monotone step function | classically ≥1,000 | more accurate, more prone to overfit |
| conformal prediction | a quantile of a nonconformity score | ~100 for ±5 points of coverage slack | gives a *set* with a coverage guarantee — the honest answer when two tiers are both plausible |

One live disagreement, flagged rather than resolved: the classical guidance says isotonic needs
about a thousand points, while the independent Jev study found isotonic matching or beating Platt at
every size from 20 upward. Note that isotonic at n=20 still produced a calibration error of 0.089,
which is bad in absolute terms. **Fit both, compare on held-out data, and trust neither paper on our
distribution.**

And re-calibrate on every prompt change: across 11 question phrasings the same study saw raw
calibration error range from 0.064 to 0.160. A prompt edit invalidates the map. `prompt_version` and
`prompt_hash` are already logged per judgment, so the invalidation is detectable.

## 6a. The strongest result in this research: fit a scorer, do not ask the model

One finding replicates from 2020 to 2026 and is worth more than everything above: **a separate
scorer trained to predict the model's errors beats the model's own confidence, consistently.**

- Kamath, Jia & Liang, *Selective QA under Domain Shift*, arXiv:2006.09462: "Our method answers
  **56% of questions while maintaining 80% accuracy**; in contrast, directly using the model's
  probabilities only answers **48%** at 80% accuracy." Eight points of coverage, free.
- ASPIRE, arXiv:2310.11689: on one benchmark, area under the accuracy-coverage curve 91.23% to
  92.63% and AUROC 74.61% to 80.25%.
- A 2026 production study of financial straight-through processing, arXiv:2609.20110: native
  verbalized confidence "could clear only 0.1%-7.0% of fields", while a decomposed per-channel
  score "auto-approves **49-72% of fields** while holding the empirical error of the accepted tier
  at or below the target". AUROC moved from 0.54-0.74 to 0.90-0.99.

And two 2026 results on why the model's own number is the wrong input: across 30 models and three
families, "a more dispersed verbal confidence distribution can carry useful rank information, but it
does not make the scores calibrated" (arXiv:2608.28382); and re-eliciting confidence for the *same
fixed answers* with equivalent prompts "flips 4% to 9% of decisions at a 0.8 threshold"
(arXiv:2609.20541).

**For us this means the target is not a better `confidence` from a better provider.** It is a small
fitted model over the features we already log — kind, source, course, title prefix, the verdict
itself, the stated confidence — trained on corrections to predict "was this judgment wrong". That is
the same table §5 already points at, and `judge_rules.ts`'s `features()` already extracts most of
the inputs.

**A caution that applies to the gate specifically.** A gate tells you commitment risk, not whether
more work would help. One 2026 study found confidence-gated retrieval raised accuracy among
committed answers by up to 41 points while **overall** accuracy rose up to 15 points on one dataset
and **fell up to 17 points on another** (arXiv:2608.26846): "calibration can make commitment risk
interpretable, but it does not estimate the expected benefit of another retrieval." Knowing an
answer is unreliable is not the same as knowing what to do instead.

## 7. What to do, ranked

1. **Write down the event kind's 3×3 cost matrix and the email kind's 5×5.** `score.ts`'s `COST` map
   is most of it. No code, no spend, and it is the precondition for every threshold decision.
2. **Measure our own `confidence`, per kind, from `judgments` joined to `corrections`.** Start with
   the count of distinct values and AUROC. If the field does not rank, the floor is cosmetic and
   should be replaced by the rules tier rather than tuned.
3. **Treat the floor as a defect, not a parameter.** Its abstention on an event costs what the
   scorer prices as the worst single error, and the abstention repeats forever because no verdict is
   written. Either lower it on a calibrated score, or make an abstention record something.
4. **Fit a calibration map once there are a few hundred labels per kind**, starting with Dirichlet
   or vector scaling for the three-way verdict.
5. **Do not adopt Jev for the event kind.** Section 3 closes it on measured evidence, which is a
   better reason than the privacy reason that no longer applies.

## Sources

All read 2026-09-22 by the research agent that produced this note's evidence base; each carries its
own date above where the source states one.

**Calibration science.** Guo, Pleiss, Sun & Weinberger, "On Calibration of Modern Neural Networks",
arXiv:1706.04599 (ICML 2017) — the definitions, the expected-calibration-error formula, temperature
and Platt scaling. Kumar, Liang & Ma, "Verified Uncertainty Calibration", arXiv:1909.10155 (NeurIPS
2019) — sample-complexity rates and the debiased estimator. Roelofs, Cain, Shlens & Mozer,
"Mitigating Bias in Calibration Error Estimation", arXiv:2012.08668 — equal-mass bins, and the
n=200 error bar. Błasiok & Nakkiran, "Smooth ECE", arXiv:2309.12236 — the kernel-smoothed estimator.
Kull et al., "Beyond temperature scaling", arXiv:1910.12656 — Dirichlet calibration. Zadrozny &
Elkan, KDD 2002, and Niculescu-Mizil & Caruana, ICML 2005 — isotonic and the ≥1,000 rule. Murphy,
*J. Applied Meteorology* 12:595-600, 1973 — the Brier decomposition into reliability and resolution.

**LLM confidence.** Xiong et al., arXiv:2306.13063 (ICLR 2024). Tian et al., arXiv:2305.14975
(EMNLP 2023). Groot & Valdenegro-Toro, arXiv:2405.02917. Chhikara, arXiv:2502.11028v3.
Sanz-Guerrero, Mager & von der Wense, arXiv:2606.03437. Zhao et al., arXiv:2604.01457 (COLM 2026).
GPT-4 Technical Report, arXiv:2303.08774, Figure 8. The Nyckel production study,
`nyckel.com/blog/calibrating-gpt-classifications/`.

**Thresholds.** Elkan, "The Foundations of Cost-Sensitive Learning", IJCAI 2001 — quoted via the
mlr-org cost-sensitive tutorial's reproduction, which attributes it explicitly; the original PDF
would not render. Flores et al., arXiv:2506.14540 — the indifference condition and the sentence that
makes calibration load-bearing. Hernández-Orallo, Flach & Ferri, arXiv:1112.2640 (JMLR 13) —
threshold-choice methods. Saito & Rehmsmeier, *PLOS ONE* 10(3):e0118432, 2015 — precision-recall
over ROC under imbalance. Angelopoulos & Bates, arXiv:2107.07511 — conformal prediction and the
calibration-set-size table. Angelopoulos, Bates et al., "Learn then Test", arXiv:2110.01052.

**Jev specifically.** TypeSafe's own pages: `docs.typesafe.ai/concepts/system-one`,
`docs.typesafe.ai/confidence.md`, `docs.typesafe.ai/patterns/confidence-routing.md`, and
`docs.typesafe.ai/model-jaggedness/jev-1.13.md` (the score-calibration, state-size and date
weaknesses). Independent measurement: `github.com/AnthusAI/Jev-Calibration` (study A, the
reliability table) and `github.com/scienthoon/jev-ood-calibration` (study B, run 2026-09-19, the
in-distribution against out-of-distribution split). `github.com/fstandhartinger/jevbench` defines a
calibration methodology but its README does not carry the numbers.

**Cascades, for §4's framing.** Burleigh, arXiv:2604.19781 (the Khan Academy grading study, AUROC
figures and the threshold-selection recipe). Chen, Zaharia & Zou, "FrugalGPT", arXiv:2305.05176.
Jitkrittum et al., "When Does Confidence-Based Cascade Deferral Suffice?", arXiv:2307.02764 (NeurIPS
2023) — the three named failure modes of deferring on a cheap model's uncertainty.

**Caveat carried forward from the research.** The Jev calibration evidence is two independent
hobbyist studies, one week old, on one model version, one of them on a single constructed dataset.
They are careful — study B established an empirical noise floor by re-sampling a perfectly
calibrated model 200 times, which is better practice than many published papers — but they are not a
literature. TypeSafe has published no reliability diagram, no calibration error, no Brier score, and
no paper describing the training method it names. Their claim that "higher confidence means higher
accuracy" is corroborated in direction by both studies; "monotone" and "calibrated" are different
claims, and only the weaker one has evidence.
