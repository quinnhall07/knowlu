# The event-kind experiment — what a calibrated probability would replace, and what would have to be true first

**Status: design note, 2026-09-22. Nothing decided, nothing run, no code changed.** Written on
Quinn's instruction to design the one experiment the Jev assessment left open
(`docs/reports/2026-09-22-jev-system-one-assessment.md`, verdict: "watch for the event kind"). It
argues from that report, from `docs/notes/2026-09-16-inference-provider-and-model-scoping.md` and
its four reports, and from the code as it stands at `b9c3e1f`. Not legal advice.

## 0. The finding that has to lead: there is no corpus

The brief asked what "a paper comparison on the existing eval corpus (`cloud/eval`)" would measure.
**That corpus does not exist**, and no design that assumes it can be honest.

- `cloud/eval/seed/` holds one file, `README.md`. It is empty **by decision** — ruling R-C2-E12 of
  2026-09-14 declined the one read that would have filled it, and `cloud/eval/seed/README.md` says
  so in its first paragraph: "This directory is empty on purpose."
- `cloud/eval/loader.ts`'s `loadSeed` returns `[]` for that state rather than erroring, deliberately.
- `eval_cases` is the real corpus and it is empty too. `cloud/eval/run_eval.ts`'s own header says
  it: "Today the corpus is empty, so the gate costs nothing."
- `cloud/eval/thresholds.json` carries `event.weighted_exact_min = 0.75`, and its `_note` says every
  number in it is "placeholder-free-but-unmeasured" and must be replaced "from the first real run
  once cases exist".

So the event experiment is two experiments, not one, and the first has nothing to do with Jev.
Section 5 sets out the three lawful ways a corpus can come to exist, and section 4 names the one
measurement that can be taken **today, with no new provider and no spend**.

## 1. Where the 0.6 floor actually bites

The floor is one constant in two places, and only one of them governs events.

| what | where |
|---|---|
| `CONFIDENCE_FLOOR = 0.6`, the server's copy | `cloud/supabase/functions/_shared/judge_validate.ts:6` |
| applied to the task verdict | `judge_validate.ts:106` |
| **applied to the event verdict** | **`judge_validate.ts:127`** |
| applied to the email verdict | `judge_validate.ts:136` |
| `CONFIDENCE_FLOOR: f64 = 0.6`, the device's copy | `engine/src/judge.rs:29` |
| applied by the device — to the **task** path only | `engine/src/judge.rs:502` |

**The device never gates an event on confidence.** `engine/src/events.rs`'s `judge_roster` (`:380`)
takes whatever `judge_event` returns, re-checks the verdict word against `eventledger::VALID_VERDICTS`
(`:449`), and records it; the confidence appears only in the printed line, `events {uid}: {verdict}
({confidence:.2})` (`:462`). So for the event kind the 0.6 gate is **server-side and single**, which
is convenient: changing what that number means is one file, not two.

The order inside the event branch matters for any replacement. `judge_validate.ts:124-125` checks
**completeness first** — the verdict must be one of `obligation` / `opportunity` / `drop` and `why`
must be a non-empty line — and only then applies the floor. A missing `why` is `incomplete`; a
present `why` with a low number is `below floor`. Both return `ok: false`.

What a rejected event costs, end to end:

1. `judge_pipeline.ts` records a `judgments` row with `outcome: "low confidence"`,
   `cause: "below floor"`, and **the confidence value itself** (`confidence real not null check
   (confidence between 0 and 1)`, `cloud/supabase/migrations/20260911000100_judgment_service.sql:75`),
   then replies `verdict: null`.
2. The device's `cloudmodel.rs` `verdict_of` sees the null verdict, reads `cause`, and raises
   `ModelError::Failed("the judgment service answered below floor")`.
3. `judge_roster` catches it, writes a line, and **records nothing in the ledger**.

That last step has a consequence the floor's author may not have intended. The events spec's
one-verdict-per-uid-forever rule is enforced by `events.rs:408`, which filters the roster to uids
whose ledger entry has no verdict. A below-floor answer writes no verdict, so **the same event is
asked again on the next slot, and the slot after that, forever** — at full price each time, until
the model happens to answer above 0.6 or the event ages out of the feed. A confident wrong answer is
permanent and free; an unconfident right answer is impermanent and recurring. That asymmetry is
worth Quinn's attention independently of Jev.

**And the event verdict has no way to say "I cannot tell".** `EVENT_SCHEMA`
(`judge_prompts.ts:50-58`) enumerates exactly `obligation`, `opportunity` and `drop`. A model shown
an event whose obligation status is simply not in the text — which is most of them, since whether an
event obliges *this* student depends on their programme and their advisor — must still pick one of
the three. **The confidence floor is the only abstain mechanism in the design**, which is why it
carries so much weight and why it is worth getting right.

An independent audit published this week measured what a missing abstain option does to a decision
model's calibration: removing the "unknown" option moved accuracy on unanswerable items from 0.950
to **0.000** and calibration error from 0.023 to **0.793**. The same shape shows up in a pre-registered
benchmark where a task router answered one class 120 times out of 120, scoring 0% on the hard slice
at a calibration error of 0.93 — "confidently, systematically wrong — the exact failure mode no
accuracy leaderboard would catch". **Adding a fourth verdict, or an explicit unanswerable signal,
is a change worth considering whatever model answers.** It costs one enum member, one validator
branch and a ledger word, and it gives the floor something to do other than guess.

Two more facts the seam imposes:

- **The event ledger has no confidence field.** `eventledger::record_verdict` (`engine/src/eventledger.rs:405`)
  takes `uid, title, when, verdict, strength, why, task`, and `LedgerEntry` (`:134`) carries
  `uid, verdict, strength, why, proposed, declined`. A calibrated probability would have nowhere to
  live on the device without changing a file `CLAUDE.md` names as a contract with existing vaults.
  Server-side it already has a home: `judgments.confidence`.
- **The ledger tolerates an empty `why`; the validator does not.** `record_verdict:419` validates
  `why` only when it is non-empty and omits the field from the line when it is empty, while
  `judge_validate.ts:124` refuses an empty one as `incomplete`. So dropping the written rationale is
  a one-line server change and a product decision about the card — not a vault-format change.

## 2. What a calibrated probability would replace, precisely

Today's `confidence` is **a number the model writes into its own JSON**. The event schema
(`judge_prompts.ts:50-58`) asks for it as `confidence: { type: "number" }` and the system template
(`:108-119`) instructs only "- confidence: 0 to 1." Nothing measures whether 0.7 means seven times
in ten. The 09-16 capability report already distrusted it, in terms this note takes as its premise:
"self-reported confidence tracks commitment, not correctness".

Jev's `choice` type is the shape that fits: a pick from supplied options, returned **with a
probability distribution over them and a confidence 0-1**. The event kind is a three-way choice, so
the mapping is exact and lossless on the decision itself.

**Which number would become the gate was the open question, and it is now answered — against
adoption.** See `docs/notes/2026-09-22-confidence-calibration-and-the-floor.md`, written the same
day. Two findings settle it:

- **`confidence` is not a second opinion; it is arithmetic on the distribution.** TypeSafe's
  documentation gives `confidence = (K · p_max − 1) / (K − 1)` for a K-option choice, so for our
  three-way verdict the two numbers carry the same information. There was no better signal to pick.
- **Measured independently, `choice` confidence does not discriminate at our operating point.** On
  8,801 examples, choice accuracy "hovered 50-57% across all stated confidences 50-95%; only the
  top band (95%+) proved informative at 90.2%". And on an out-of-distribution task whose label
  depended on a policy absent from the input — structurally our problem, since whether an event
  obliges *this* student is usually not in the event description — calibration error was 4.4 times
  the study's own noise floor, the author's summary being "confidently wrong where the rule is
  unknowable".

**So the capability this note was designed to test does not hold.** Everything below stands as the
design that would have been run, and as the record of why it will not be.

What it cannot replace: `why`. Jev emits no text, and the event verdict's `why` is a required field
server-side, rides through `cloudmodel.rs`'s `judge_event`, and is written into the ledger line as
`why:"…"`. Three ways out, priced in section 8:

1. **Drop it.** One server-side validator change; the ledger already accepts it; the card loses a
   line of explanation. A product decision, and Quinn's.
2. **Template it in code.** If the drop rules become their own questions (section 3), the template
   writes itself: "dropped: audience is faculty/staff" is a deterministic string over a Noul that
   fired, and it is *more* honest than a generated sentence because it names the actual reason.
3. **Decide cheap, narrate expensive.** Jev picks, and the current pinned model writes a `why` only
   for the items that survive. This is the hybrid the wider literature calls a cascade, and
   section 8 shows it saves about half a cent a month, which is not a reason to build it.

## 3. The request shape — it is not a chat completion

This is the part that makes Jev a second prompt representation rather than a second client.

**Today.** `ModelRequest` is `{model, system, user, schema, maxTokens, sampling, route}`.
`judge_openrouter.ts`'s `openRouterBody` (`:37`) turns it into a `/chat/completions` body: a
two-message array, `max_tokens`, `response_format: {type: "json_schema", strict: true}`, and the
row's `route` as OpenRouter's `provider` object.

**Jev.** `POST https://api.typesafe.ai/v1/systemone` with `{state, model, questions}`, answering
`{answers, usage}`. No system/user split. No JSON schema — there is no JSON to constrain. No
`max_tokens`, because nothing is generated. No `reasoning` switch, so the pinned row's `sampling`
would be `{}` — and `sampling` is the column the 09-16 swap added specifically to carry a thinking
model's off switch.

**And it is not reachable through the endpoint our client already speaks.** Jev returns nothing from
OpenRouter's `/api/v1/models` listing, and its model record carries
`"output_modalities":["decisions"]`, `"has_text_output":false` and `"supported_parameters":[]`. So
`response_format` and every other chat parameter are inapplicable, and structured outputs neither
covers it nor is needed. OpenRouter instead exposes two dedicated endpoints,
`POST /api/v1/systemone` and `POST /api/alpha/decisions`, which take the same body as TypeSafe's own
and add `id`, `provider` and **`usage.cost`**. Two practical notes: OpenRouter caps the context at
32,000 tokens against TypeSafe's documented 64k, and its request schema accepts a `provider` routing
object, so `assertPinnedRoute`'s pin has somewhere to live after all.

A faithful sketch of the event kind, from the fields `cloudmodel.rs`'s `event_request` (`:430`)
already sends and the rules `systemTemplate("event")` already states:

```
state = {
  title, start, end, source, organizer, location, url,
  description (clipped to 1200 chars), categories, audiences, series_uid,
  interests            // from heuristics_seed, clipped to 600
}

questions = {
  verdict:  choice over ["obligation", "opportunity", "drop"]
  audience: noul  "this event is aimed at faculty, staff, alumni or graduate students"
  standing: noul  "this is a standing exhibit, an office-hours block or a recurring drop-in"
}
```

The three questions evaluate in one pass at one price, which is the design's genuine advantage over
a chat model and the thing worth testing. Note what it does to the prompt: two of the event
template's five rules stop being instructions the model must remember and become **separate
questions with their own probabilities**, which code can combine deterministically. A drop that
fires because `audience > t` is auditable, has a templated `why`, and can be overridden by a rule in
`judge_rules.ts` without touching a prompt. That is a better architecture than the one we have, and
— this is the point — **most of it is available without Jev**: the same decomposition can be asked
of Granite 4.2 8B as three schema fields on the call we already make.

**One caveat on that sketch, from TypeSafe's own documentation.** Questions in a request are
independent: "one answer does not become context for another question", and the model is not
coherent across them. Their example is a question and its own negation, asked as two nouls, coming
back 0.72 and 0.47 — summing to 1.19. So the three probabilities above may be combined by a
deterministic rule, which is TypeSafe's own recommended step, but they may **not** be treated as a
joint distribution or expected to obey arithmetic identities. A combining rule has to be written
and tested as a rule, not derived as algebra.

What the seam absorbs, and what it does not:

| | |
|---|---|
| `judge_provider.ts`'s `modelFor` | absorbs it — one `case`, one `API_KEY_ENV` entry |
| `judge_pipeline.ts` | absorbs it — it never asks who answered |
| `judge_models.ts` row | mostly absorbs it — `sampling: {}`, `grammar_version` loses its meaning, `max_tokens` unused |
| `buildPrompt` (`judge_prompts.ts:142`) | **needs a sibling** emitting a `questions` map per kind |
| `promptHash` (`:215`) | **breaks.** It hashes `systemTemplate(kind) + "\0" + JSON.stringify(schemaFor(kind))`. Neither exists on a Jev row, so it would log a hash of a prompt never sent. It must hash the questions map instead |
| `assertPinnedRoute` (`judge_openrouter.ts:21`) | **does not transfer.** It refuses to send unless the row's route is `order` of one, `allow_fallbacks: false`, `zdr: true`, `require_parameters: true`. A direct TypeSafe client has no such object, so the zero-retention guarantee drops from a per-request assertion to a contract — exactly what that function exists to prevent |
| `judge_validate.ts` | needs an event path taking a distribution rather than a `confidence`, and either a templated `why` or a relaxed completeness check |
| `cloud/eval/run_eval.ts` | a Jev variant missing `why` is still comparable on `weighted_exact`, which scores `verdict` alone (`score.ts:92`) — the one kind where this is true |

`rank` never calls a model (Knowlu spec decision 11) is untouched throughout: every line above is in
the judge step.

## 4. What the comparison would measure

Three families, and only the first two need Jev.

**(a) Decision quality — the metric we already have.** `score.ts`'s `weighted_exact` for the event
kind, which is not accuracy: it prices the confusion matrix with `COST` (`score.ts:29-38`), where
`obligation->drop` costs 3 and every other named mistake costs 1, and a missing answer also costs 3
(`:44`). The gate is `event.weighted_exact_min = 0.75`, provisional. Run the same cases through the
current pin and through Jev and compare. This is the number that decides whether Jev is *competent*
here.

**(b) Calibration — the number worth the effort.** Take the probability each system attaches to the
verdict it chose, and ask whether it means anything.

- **Reliability diagram.** Bin the predictions by probability, and in each bin plot the mean
  predicted probability against the observed fraction correct. A calibrated system sits on the
  diagonal.
- **Expected calibration error.** `ECE = Σ_b (n_b / N) · | acc_b − conf_b |` over the bins — the
  average gap between claimed and observed, weighted by how many predictions fall in each bin.
- **Brier score.** `(1/N) Σ (p_i − y_i)²`, where `y` is 1 when the prediction was right. It moves
  with both calibration and discrimination, so report it beside ECE, never instead of it.

**(c) The floor's own operating point — and this one needs no Jev, no new provider and no spend.**
The question the 0.6 gate exists to answer is: *of the answers we throw away, how many were right,
and of the answers we keep, how many were wrong?* Everything needed is already logged.
`judgments` carries `confidence`, `outcome`, `cause` and `fields` — and `fieldsOf`
(`judge_pipeline.ts:71`) drops only `confidence`, `why`, `importance_reason` and `title`, so
**`fields.verdict` is there**. `corrections` carries `judgment_id` and `theirs`. A join gives
(claimed probability, was it right) pairs for every judgment a student has corrected.

Measure that first. If the current `confidence` turns out to be roughly calibrated, the event case
for Jev is over. If it is badly calibrated, the fix may still not be Jev: post-hoc calibration on
logged data (Platt scaling, isotonic regression, histogram binning) re-maps a miscalibrated score
onto observed frequencies using nothing but our own labels, and would apply to all three kinds
rather than the one kind Jev can serve. `docs/notes/2026-09-22-confidence-calibration-and-the-floor.md`
covers that literature, and its §4 shows the floor itself is very likely mis-set.

**The threshold itself is the deeper problem.** 0.6 was picked by intuition, and the right
threshold for an asymmetric cost is not a round number: with a cost of 3 for a missed obligation
against 1 the other way, the expected-cost-minimising operating point sits well below the midpoint,
and it can only be chosen once the probability is calibrated enough to mean something. A calibrated
probability is not an end in itself — it is the precondition for choosing the gate on purpose.

## 5. The corpus — three lawful sources, ranked

None of this runs without labelled event cases. What is permitted:

1. **Hand-written synthetic seed cases** (available now). `cloud/eval/schema.ts` accepts them: a
   `seed-` prefixed id, `request` exactly as `event_request` builds it, `theirs: {verdict}`, and
   nothing that trips `scrubViolations` (no `@` token, no URL, no path, no run of 7+ digits, no
   string over 200 chars). Twenty to forty cases covering the three drop rules and their edges are a
   day's work and would make the suite's own gate real for the first time. **They measure whether a
   model applies our stated rules — not what the real distribution looks like.** That is a genuine
   limit, and it is also exactly what a rules-into-Nouls decomposition needs to be tested against.
2. **Quinn's own vault, after P3** — the live comparison ruling 4 already schedules, old pin against
   new on the same items. This is the only source with a real distribution, and it is one student's.
3. **`eval_cases` from consented corrections** — the designed path, gated on the (c) opt-in whose UI
   is C4. Nothing to do here until that ships.

An honest sample-size note: at 24 cases a three-class accuracy estimate carries roughly ±20 points
at 95% confidence, and a reliability diagram needs more than that before its bins mean anything. A
paper comparison on a hand-written seed can rule a model *out*; it cannot rule one *in*.

## 6. Retention — what would have to be true before one event reached Jev

The 09-16 legal report split the event kind by origin, and the split is what governs:

- **Events from an ICS feed** sit in the task class: no training, retention for abuse only and
  ≤30 days, no discretionary human review, a DPA.
- **Events from the Google Calendar API** are sensitive-scope data under Google's Limited Use
  policy, which "applies to data derived from both Sensitive and Restricted scopes" — the same class
  as Gmail, needing the strictest self-serve tier.

**This section was revised after the re-check of the same day**
(`docs/notes/2026-09-22-jev-retention-recheck.md`), which changed the answer. Two positions have to
be kept apart, because they differ:

| requirement | **TypeSafe direct** | **via OpenRouter** (the production path) |
|---|---|---|
| no training on inputs | stated: will not train or fine-tune on your Input | `"training": false` on the endpoint |
| retention bounded, abuse only, ≤30 days | "as long as reasonably necessary"; no period stated. The MCA of 2026-09-19 grants a perpetual telemetry and abuse-monitoring licence | `"retainsPrompts": false` — **stronger than the loose class asks** |
| zero retention, self-serve | enterprise-only, under a DPA, via `privacy@typesafe.ai` | a per-request `zdr` parameter — **self-serve** |
| no discretionary human review | not addressed | moot if nothing is retained |
| a DPA, sub-processors, region | yes; US hosting | as today's OpenRouter pins |

So **the transport-level retention objection to ICS-origin events falls away**, because R8 as
amended already routes through OpenRouter with `zdr: true` asserted per request by
`assertPinnedRoute`. Two cautions travel with that, both recorded in the re-check: OpenRouter
disclaims its own accuracy ("not a definitive source of third party data policies"), and the
commitment, if it exists, runs between OpenRouter and TypeSafe rather than between Knowlu and
TypeSafe — the same reliance today's CoreWeave and DeepInfra pins already carry, on a company whose
own published default is weaker.

**The binding constraint is now the origin, not the terms.** `models` is keyed by `kind`
(`judge_models.ts:27-30`, `models?kind=eq.${kind}`), not by origin, and the 09-16 note took that
deliberately: "routing by origin would add code for no saving". A single `event` row pointing at Jev
would carry Calendar-derived, Limited-Use-bound text down the same path as ICS text. The strict
class may well be satisfiable through OpenRouter's self-serve zero retention — ruling 1 asks for
"retention as short as offered, which self-serve means zero", and that is what the flag reports —
but it rests on an unseen contract, and that is not the evidence a Limited-Use commitment should
stand on.

**Verdict: ICS-origin events are now reachable on the terms; Calendar-origin events are not, until
the pin can tell them apart.** What would have to become true, in order:

1. The distribution question of section 2 resolves in favour of the distribution being the
   calibrated quantity. (Free. Read the docs.)
2. Either the `models` table gains an origin dimension, or Calendar-origin events are excluded from
   the Jev path by construction. (A migration and a pipeline change — and the 09-16 note's reason
   for not doing it, "no saving", is still true.)
3. The OpenRouter ZDR flag is re-checked at the moment of pinning, not once: it is the term
   `assertPinnedRoute` would be enforcing, and a flag that appeared can disappear.
4. `privacy_version` bumps, every account is emailed, and each says yes in the app **before the
   first request** — the privacy page's own "Changes to this policy" clause. Cheap at one account,
   never cheaper again.

None of this makes the case *for* Jev. It removes one objection of several, and the remaining ones —
no generated text, proprietary weights against R8's open-weights wording, and a saving of 1.8 cents
a month — are sufficient on their own.

## 7. Which of Quinn's rulings this would ask to amend

The brief asked this precisely, so here it is ruling by ruling.

**Ruling 1 — the privacy floor, split by data class. Not amended, after the re-check — and that is
the change.** Before the re-check this note expected ruling 1 to be the blocker, because TypeSafe's
own zero-retention tier is sales-gated. But ruling 1 is about the terms a *provider* offers, and the
provider of record under R8 as amended is OpenRouter, which offers zero retention self-serve per
request and reports this endpoint as retaining nothing. So ICS-origin events meet the loose class
and arguably the strict one, with no amendment at all. **What ruling 1 still forbids is sending
Calendar-origin events down a pin that cannot distinguish them** — and the honest answer there is to
make the pin origin-aware or leave the Calendar half where it is, not to amend the ruling.

**Ruling 2 — the $0.25 per student per month ceiling. Not amended, and not the argument.** The
service sits at $0.237; the achievable saving is 1.8 cents (event only). Ruling 2 is satisfied
today, so it cannot be a reason to adopt — and `MONTHLY_CEILING_USD` (currently 2.0,
`judge_caps.ts`) would not move.

**Ruling 3 — hosted APIs only, no self-hosting. Not amended.** Jev is hosted. Its open
re-implementation, Kev, is self-hosted and therefore out of scope by this ruling, not by preference.

**Ruling 4 — a paper study now, the live comparison later on Quinn's own vault after P3. Not
amended.** This note is the paper study's design; section 5 is why the paper half cannot be run yet.

**R8 as amended — the real amendment.** Quinn ruled option 1 of the 09-16 note on 2026-09-16, whose
own words are "Amend R8 to 'an **open-weights** provider on self-serve zero-retention terms, models
pinned per kind, the eval suite picks'". Jev is **proprietary weights** with no published size,
architecture or model licence. Adopting it for any kind asks Quinn to amend R8 a second time, to
something like "open weights preferred; a proprietary model permitted for a capability no
open-weights model provides". That is a real trade and it should be made for a real capability — not
for 1.8 cents.

**One rule not in question.** `rank` never calls a model, and every line of this design sits in the
`judge` step. Nothing here touches it.

## 8. Cost, honestly

From the 09-16 note's typical persona, the event kind alone: 255,000 input and 21,000 output tokens
a month.

| arrangement | arithmetic | $/student/month |
|---|---|---|
| today (Granite 4.2 8B, $0.10 / $0.15) | 0.255 × 0.10 + 0.021 × 0.15 | **0.029** |
| Jev alone, `why` dropped ($0.042 in, output free) | 0.255 × 0.042 | **0.011** |
| hybrid: Jev decides, the pin writes `why` for non-drops (assume 40%) | 0.011 + (0.114 × 0.10 + 0.0096 × 0.15) | **0.024** |

So: **Jev alone saves 1.8 cents a month and costs a written rationale. The hybrid saves half a
cent and keeps it.** Against a $9.99 subscription, both are rounding. And one caveat pushes the Jev
column up, never down: Jev bills `state` **plus every question's instructions and criteria**, so a
three-question event request bills more input than the ~850 tokens the current single prompt does —
**unverified by how much**, and worth measuring before any of these numbers is quoted again.

The conclusion is the assessment's and this note does not disturb it: **cost is not the reason to do
this.** Calibration is the only thing that could be, and section 4(c) can test the premise for free.

## 9. Stop rules, and what this note recommends

**Do not open a provider-swap task for Jev, and do not run this experiment.** Step 1 below was the
gate and it has already failed, on published measurement rather than on our own spend.

1. **Establish which number Jev calibrates.** **Done, 2026-09-22, and it fails
   (`docs/notes/2026-09-22-confidence-calibration-and-the-floor.md` §1 and §3).** `confidence` is a
   derived statistic of the distribution, so there was never a second signal; and the `choice`
   type's confidence was measured as uninformative between 50% and 95%, on a model whose calibration
   degrades sharply on tasks whose label is not in the input. **The event case closes here**, and on
   better grounds than the privacy argument that no longer applies.
2. **Measure our own calibration**, from `judgments` joined to `corrections`. No provider, no spend,
   no ruling amended. This is now the whole of the work, and it was always the part worth doing.
3. **If it is badly calibrated, fit a calibration map on our own logs.** Dirichlet or vector scaling
   for the three-way verdict. It needs no new party on the privacy page, no `privacy_version` bump,
   and it fixes all three kinds rather than the one Jev could have served.
4. **Write the synthetic event seed** (20-40 cases) regardless. It makes `thresholds.json` real, it
   is a day's work, and every future pin decision needs it.

Three findings stand on their own, and none of them depends on Jev:

- **The 0.6 floor is on the wrong side of even.** The product's own cost table prices
  `obligation->drop` at 3 against 1 for every other event mistake, and the cost-optimal gate for a
  3:1 asymmetry is 0.25, not 0.6. The companion note does the arithmetic and states the two
  qualifications that stop it being a one-line change.
- **A below-floor event is re-asked forever** (section 1), and its abstention costs what the scorer
  calls its most expensive single error, because the student sees nothing at all. A confident wrong
  answer is permanent and free; an unconfident right one is invisible and recurring. Worth a ruling.
- **The event prompt's drop rules would be better as separate questions with their own
  probabilities** (section 3) — and that can be done on the model we already pin, as extra schema
  fields, with no new provider at all.

## Sources

In this repository, read 2026-09-22 at `b9c3e1f`: `cloud/supabase/functions/_shared/judge_validate.ts`,
`judge_prompts.ts`, `judge_pipeline.ts`, `judge_models.ts`, `judge_provider.ts`, `judge_openrouter.ts`,
`judge_caps.ts`; `cloud/supabase/migrations/20260911000100_judgment_service.sql`;
`cloud/eval/{schema.ts,loader.ts,score.ts,run_eval.ts,thresholds.json,seed/README.md}`;
`engine/src/{judge.rs,events.rs,eventledger.rs,enrich.rs,cloudmodel.rs}`;
`docs/specs/2026-09-09-knowlu-cloud-design.md` §1 and the Amendment 2026-09-16;
`docs/notes/2026-09-16-inference-provider-and-model-scoping.md` §§1-9;
`docs/reports/2026-09-16-inference-scoping-legal-floor.md`;
`docs/reports/2026-09-22-jev-system-one-assessment.md`.

Every external figure about Jev in this note — the price of $0.042 per million input tokens with
output free, the `choice` / `score` / `noul` primitives and their limits, and the
`POST /v1/systemone` request shape — is carried from the Sources section of
`docs/reports/2026-09-22-jev-system-one-assessment.md`, where each one has its URL and read date.
**Every retention figure in section 6 comes instead from
`docs/notes/2026-09-22-jev-retention-recheck.md`**, read the same day, which supersedes the
assessment on that point and carries its own URLs and dates.
