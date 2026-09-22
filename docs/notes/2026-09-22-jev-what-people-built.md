# What people actually built with Jev in week one, and the three lessons that transfer

**Status: research note, 2026-09-22. Nothing decided, nothing installed.** Written on Quinn's
instruction to scour primary sources for the novel ways people are using this model, after Quinn
correctly challenged the earlier report's verdict. Five research passes covered GitHub, Hacker News,
Reddit, X, YouTube, LinkedIn, Product Hunt and engineering blogs. This note keeps what transfers to
Knowlu or to how we build it, and records what is measured against what is asserted.

**Read the source caveats in §6 before quoting any number here.** Much of the week-one material is
self-reported, some of it is probably promotional, and the single best-designed studies disagree
with each other.

## The three lessons

**1. Decompose the question. It is the strongest and most replicated finding of the week.** Asking
one broad question is markedly worse than asking several narrow ones and combining them in code.
Four independent measurements:

| task | one question | decomposed |
|---|---|---|
| phishing detection, 2,000 emails | 62.6% | **95.1%** |
| shell-risk gate, false positives | 14.5% | **1.8%** |
| skill routing, recall at 1 | 9.0% | **81.0%** |
| Japanese natural-language inference | 83.7% | **90.8%** |

**And the counter-case, which matters just as much.** One team measured the other direction: on a
security task, twelve weighted dimensions produced a **37.2% false-positive rate on hard benign
cases against 1.5% for the single call**, roughly twenty-five times worse. Their conclusion:
"Splitting a judgment into scored dimensions only helps when the single call is already weak;
elsewhere you pay 1.6-2.3x the tokens for a worse error profile."

This validates and qualifies the decomposition sketched in
`docs/notes/2026-09-22-jev-event-kind-experiment-design.md` §3 — turning the event prompt's drop
rules into separate questions is the right instinct, it is testable on the model we already pin, and
it should be measured rather than assumed.

**2. The accuracy belongs to your labels, not to the model.** The single most useful sentence anyone
wrote this week, from the phishing benchmark that produced the 62.6% to 95.1% jump: **"The 95% is
not Jev. It is Jev plus your labelled data plus a regression you maintain."** Their decomposition
fed five atomic signals into a logistic regression fitted on half the data. A non-AI regex baseline
on the same task scored 91.6%. This is the same conclusion
`docs/notes/2026-09-22-confidence-calibration-and-the-floor.md` §6a reaches from the academic
literature, arrived at independently by a practitioner with a benchmark.

**3. Price changes the shape of systems, not just their cost.** The most interesting build of the
week exists only because a check became cheap. An agent-memory tool checks **every stored memory
against every new event** — six yes/no questions per pair — because "a check is 150 ms and $0.00006.
So we check all of them", where memory products previously "check the ten nearest, or nothing". It
reports 89.2% strict agreement on 157 labelled cases with zero false invalidations, and about $7 a
month against $240 with a small language model at 200 memories and 200 daily events. Its best design
decision is one we already hold: **the memory text is never edited, a stale memory is marked and the
replacement stored verbatim.** That is our journal-first, never-rewrite rule, arrived at from the
other direction.

## 1. What the categories actually are

**Evals and judging** is where the measured wins are strongest. LangChain compared Jev against three
language-model judges on captured agent runs: Jev matched the oracle on all 500 repeated decisions,
against 99.8%, 96.4% and 80.0% for the three models, at $0.34 total against $28.17, with variance
two to three orders of magnitude lower. Their own caveat is that the dataset is five examples
repeated, not five hundred distinct ones. A separate study of **6,003 rubric checks across 1,203
answers** found agreement with frontier judges of 86.5% to 91.5% at $160 per million graded answers
against $33,000. The rebuttal on Hacker News is the part to keep: it is only 2.6 times cheaper than
one cheap frontier model the authors did not test, and the author conceded the point.

**Routing and triage** is heavily adopted and unevenly evidenced. A clean before-and-after from an
engineering blog replaced a language-model router judge: median judgment time **1.55 s to about
0.3 s**, routing correctness 36 of 39 to **39 of 39**, at $0.000023 a judgment. A support-triage
study under load found cost per thousand correct on-time decisions flat at $0.048 across all arrival
rates against $24.48 for a small chat model at 40 tickets a second — but its own author notes Jev
was **not** the most accurate model tested (92.2% against 93.6%) and that "over 90% of the lateness
is queueing, not inference".

**Guardrails and tool gating.** The most careful result is a prompt-injection benchmark over 662
samples: **96.5% accuracy, area under the curve 0.9927, 325 ms median**, and adding deployment
context moved it from 89.7% to 96.5%. Vulnerable-code detection on the same repository is much
weaker, 71.5% at a 0.50 threshold. A shipped guardrail for another coding agent reports 3 holds per
1,000 calls over 18,075 guarded calls, and — to its credit — that only **2 of 13 labelled holds
stood, the other 11 clearing on retry**.

**Compaction and memory**, covered above, plus a gateway that ships relevance-based tool-result
pruning as a guardrail with a default threshold of 0.2 and **no published token reduction or latency
figure**.

**Reranking** is genuinely unresolved. The best-designed study, 14 datasets and 1,617 scored
questions, put Jev's four-level rubric at 0.692 against a commercial reranker's 0.691 and said so
plainly: "This establishes neither a winner nor equivalence." Query-weighted scoring actually favours
the commercial product. The cost gap is real: $0.45 per thousand queries at 422 ms against $2.51 at
844 ms.

**And the long tail is wild**: typed judgments as SQL functions over every row in SQLite, Postgres
and DuckDB; graph traversal where each node's outgoing edges become the options and beam search
ranks by sum of log-probabilities; a browser agent returning an operation and a target element in
one round trip; Doom, StarCraft, Pong, chess; a market maker deciding once per blockchain block; and
the most unexpected of all, deciding which attention blocks of *another model* to skip, for a
measured 40% faster generation.

## 2. The thresholds people actually chose, and why ours is interesting

| threshold | who | what it gates |
|---|---|---|
| **0.6** | pydantic-ai's merged `typesafe_tool_call_threshold`, as the **default** | tool calls |
| 0.2 | a gateway's tool-result relevance pruning, default | drop a stale result |
| 0.8, with a 0.2-0.8 human review band | an on-call pager | page immediately |
| **0.99** | PriorBench, pre-registered, 5,721 calls | everything |
| 0.831, fitted by conformal risk control | a certification harness | 84.75% auto-routed |
| half the lowest true-positive score, fitted | a practitioner's rule | a rejection line |

**Two of these deserve attention.** The first is that a widely used Python agent framework picked
**0.6** as its default for gating tool calls, independently of us — which is either mild
corroboration or evidence that 0.6 is simply what people reach for.

The second is the pre-registered benchmark's finding, which is the most uncomfortable number in this
research: **"Gate at 0.99 or not at all. Accuracy above threshold is flat from 0.50 to 0.95, then
jumps to 100% at 0.99, covering 60.2% of traffic."** If that shape holds generally, a gate anywhere
in the middle of the range is doing nothing at all, and the two best calibration studies agree the
middle is where calibration is worst. **Anyone picking 0.7 or 0.8 — or 0.6 — is picking a number
inside the unreliable zone.**

And one hard ceiling on any threshold: a certification study found the model "returns a top
probability of exactly 1.0 on **56.4% of answers, and 9 of those were wrong**". A separate benchmark
found **29 answers wrong at confidence exactly 1.00** and concluded "no threshold reaches those at
any setting". A gate cannot catch an error the model is certain about.

## 3. Where it fails, measured

Worth recording because these are the shapes our own kinds take.

- **Asked once, badly.** Phishing over 2,000 emails: Jev 62.6% against a small chat model's 81.3%,
  calibration error 0.154 against 0.097. It wins on speed and cost and loses clearly on accuracy.
- **Non-English degrades, and it is measured.** A paired audit on 600 items found accuracy 0.883 in
  English against **0.773 in Russian**, with calibration error moving 0.032 to 0.096. **Our task
  kind reads Spanish coursework bodies.** No Spanish figure exists, and this is the closest proxy.
- **Dates and counting.** The vendor's own weakness page says it "reads dates as text, not as ordered
  quantities" and "is not a calculator". A practitioner got "10 business days from June 1" wrong and
  found the fix is to extract the fields, which it did 12 times out of 12, and let ordinary code do
  the arithmetic. That is the email kind's `due` rule exactly.
- **It cannot abstain.** Put plainly on Hacker News: "if the user input is outside of the range of a
  boolean, it's forced to hallucinate. **It can't abstain.**" And measured: removing an "unknown"
  option moved accuracy on unanswerable items from 0.950 to 0.000 and calibration error from 0.023
  to 0.793. Our event schema has no such option, which is now recorded as a finding in the
  event-kind note.
- **No explanation, and someone said the quiet part.** Simon Willison: "**If Jev marks something as
  spam, which content signals tipped it off?** … I really hope nobody uses Jev to rank job
  applicants — that floating point number could conceal all manner of unseen bias baked into the
  models." Knowlu ranks a student's obligations and shows them a written `why`. Trading that
  sentence for a float is a product decision, and this is the argument against it stated better than
  I stated it.
- **The headline speed-up is an Amdahl illusion.** The vendor claims 193.6 times faster. A
  measurement on the company's own published pipeline fork found that routing **one** step through
  it gave **15.9% faster end to end and 30.1% lower cost per ticket**. Both numbers can be true. Only
  the second one is the number a system owner cares about.
- **And the most honest writeup of the week is a retraction.** A practitioner first measured "5000x
  cheaper" for web-form filling, then found the early runs "only worked at all because the harness
  fed Jev a hand written plan per form… The time and tokens to produce that plan were never counted.
  That's how you get 5000x." Rerun with planning counted: **2.7 times slower and 1.9 times more
  expensive.** "The cost sits in the planning, not in the clicks."

## 4. The adoption signal, and what it is worth

Real integrations shipped fast: a merged model provider in pydantic-ai, an adapter merged into
another evaluation tool that deliberately bypasses the usual chat abstraction because Jev does not
fit it, a partner package in LangChain's main repository, a classifier option in a popular proxy's
auto-router, and a first-party agent skill. A platform reported it as "the fastest-adopted model in
gateway history", about 13% of paid teams within 24 hours — **while it was free on that platform
through 25 September**, which confounds the number and which the platform does not mention.

**The most instructive adoption fact is a negative one.** A community plugin wired Jev into seven
decision points of a coding agent — skill triggering, intent, loop continuation, model routing,
context pruning, completion verdict and task size — with a shadow-versus-active switch and a
promotion ticket. Its activated set is **empty**. Every point runs in shadow, logging what it would
have decided, and nothing has been promoted. That is the right shape of caution, and it is also an
accurate picture of how strong the evidence is.

## 5. What I would take from this into our own work

Three things, none of which requires adopting anything.

1. **Decompose the event prompt's rules into separate scored fields on the model we already pin**,
   and measure it. Lesson 1 says this is the highest-value change available, lesson 1's counter-case
   says measure rather than assume, and the pinned model can answer extra schema fields today.
2. **Give the event verdict a way to say "I cannot tell".** §3's abstention evidence and the
   event-kind note now both point at it. One enum member.
3. **Fit a scorer on our own corrections rather than shopping for a better `confidence`.** Lesson 2
   is a practitioner finding it independently; the calibration note reaches it from the literature.

And for how we build rather than what we build, two patterns are worth copying whoever provides the
model: **cheap deterministic prefilter before any model call**, which every well-built guardrail
does, and **shadow mode with an explicit promotion step**, which the most thorough coding-agent
integration is still sitting in.

## 6. Source quality, stated honestly

**Weight measured studies far above volume.** A Hacker News commenter alleged a coordinated
promotion campaign: "All LLM subreddits are getting flooded by Jev posts, many of which are made by
new accounts that only talk about Jev." The sweep is consistent with the volume claim — roughly 170
posts across 31 subreddits in seven days, and at least ten near-identical forks of one "awesome"
list with 76 to 1,300 stars each. **Intent is unverifiable and I make no claim about it.** But it
means the number of enthusiastic posts carries no information, and only the studies that publish a
protocol do.

**The two independent leaderboards disagree.** One ranks Jev first overall of 52 systems, by 1.3
points, on a composite that includes cost and speed. The other scores capability alone and has it
**losing to a cheap frontier model on both question types**, with rubric scoring at 9.2 out of 100
beside a note that no model clearly beats guessing. Both are in the calibration note.

**Unresolved contradictions**, recorded rather than adjudicated: the context window is documented as
64k by the vendor and 32,000 by two of the three platforms serving it; option-order sensitivity is
reported as drastic by two practitioners and measured at **zero argmax flips in 400 tests** by a
third; and whether "output tokens are free" is published at all — one pass quoted it verbatim from
the vendor's models page, another could not find any output rate anywhere. The cost tables in the
event-kind and replay notes assume free output, which is corroborated but not universally locatable.

**Tool limits that shaped the evidence.** X returns a payment-required error to automated fetching,
so every post quoted in the research came from search-result text and none was opened. Reddit was
reached only through its syndication feeds, so **no vote or comment count appears anywhere in this
note**. YouTube pages did not render, so sixteen video titles and channels were confirmed but no
video was watched and nothing is claimed about their content. GitHub code search is behind a
sign-in wall, so the scale of adoption is estimated from topic pages and community directories
rather than counted. Community directory rows are the posters' own claims; one directory says so
itself.

## Sources

Everything above was read on 2026-09-22 by one of five research passes. Rather than reproduce two
hundred links, the load-bearing ones by claim:

**The decomposition finding**: `anisselbd/jev-phishing-bench` (2,000 emails, the 62.6% to 95.1%
result and the "not Jev, plus your labelled data" conclusion); `govindup63/skillpick` (the skill
routing figures); and the counter-case at `agentjournal.dev/blog/llm-judge-vs-feature-extraction/`,
2026-09-17.

**Calibration**: `donttrustme.ai/assay-001.html` (pre-registered, protocol frozen 2026-09-17 before
any query; the two calibration errors and zero type errors in 8,576 responses); `jevbench.xyz` run
002 (the 29 wrong at confidence 1.00); `github.com/nikkoxgonzales/jev-certify` (conformal risk
control, the 56.4% resolution floor); `jujumilk3/jev-calibration-audit` (the missing-abstain
collapse); `AHTOOOXA/jev-cyrillic-audit` (the non-English degradation);
`github.com/AnthusAI/Jev-Calibration` and `scienthoon/jev-ood-calibration`, both also cited in the
calibration note.

**Evals and routing**: `langchain.com/blog/jev-agent-evals-langsmith` (2026-09-20);
`goodstartlabs.com/research/verification-is-the-bottleneck` (2026-09-15) and its Hacker News
rebuttal; `dev.classmethod.jp` (2026-09-20, the router judge swap).

**Memory and compaction**: `github.com/chopratejas/invalidate`; `docs.litellm.ai/blog/typesafe-jev-compaction`
(2026-09-18).

**Reranking**: `github.com/anessbelbati/jev-rerank-bench`.

**The failure writeups**: the retracted 5000x benchmark (Reddit, r/ClaudeAI, 2026-09-17); the
Amdahl measurement (`thecherrycreeknews.com`, 2026-09-21); Simon Willison's black-box objection
(`simonwillison.net/2026/Sep/21/jev/`); the Hacker News launch thread (item 49717558, 1,960 points,
511 comments) for the schema-guarantee and cannot-abstain arguments.

**Vendor claims, marked as such**: `typesafe.ai/blog/introducing-system-one-models-and-jev`
(2026-09-15) and `docs.typesafe.ai/model-jaggedness/jev-1.13` (last reviewed 2026-09-17), which is
the vendor's own honest list of nine failure modes and is better than most third-party writing.
