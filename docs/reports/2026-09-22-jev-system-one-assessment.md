# Jev ("System One") — an assessment against Knowlu's judgment service

**Question (Quinn, 2026-09-22):** "look into using the new Jev 'system one' model for our product."

**Date:** 2026-09-22. **Status:** research report, nothing decided. Not legal advice.

**Consulted.** In the repository: `docs/specs/2026-09-09-knowlu-cloud-design.md` §1 (D1–D12) and its
2026-09-17 amendment; `docs/notes/2026-09-16-inference-provider-and-model-scoping.md` (the four
rulings, §2's token model, §4–§6's tables), its three reports and the host check; and the live code
— `cloud/supabase/functions/_shared/judge_{prompts,models,provider,openrouter,validate,pipeline}.ts`
with `migrations/20260916000100_provider_swap.sql` (today's pins). On the web: TypeSafe's docs and
legal pages, Cloudflare's and OpenRouter's listings, four independent benchmarks — URLs in §Sources.

## Verdict

**No for email, no for task, watch for event — do not adopt now, and do not open a provider-swap
task for it.** Jev is real: TypeSafe AI released it on 2026-09-15 as a "System One" model that
returns typed values with calibrated probabilities and *never returns text at all*, and that last
fact decides the matter. Knowlu's schemas require generated strings — email's `title` and `due`, and
the `why` and `importance_reason` lines in the vault and on the cards — and Jev can emit none of
them; its jaggedness page says it "reads dates as text, not as ordered quantities" and is "not a
calculator", exactly the email prompt's hardest requirement. Only the event kind fits cleanly — a
three-way choice plus a confidence — and there Jev's calibrated probability would genuinely improve
on the self-reported `confidence` the 2026-09-16 study distrusted and the `CONFIDENCE_FLOOR = 0.6`
gate depends on. But the prize is small: that kind costs 2.9 cents per student per month today
against 1.1 on Jev, while the whole service already sits at $0.237, inside Quinn's $0.25 ceiling.
Against that, Jev is proprietary weights (R8 as amended asks for open weights), its zero-retention
tier is enterprise-only and sales-gated (so ruling 1's strict class and the task/event floor both
fail), and its request shape is not a chat completion, so it needs a second prompt representation,
not a second client behind `complete`. Spend nothing now; re-read its legal pages if a self-serve
retention control ships. One finding is worth acting on regardless: its calibration result is the
strongest argument yet that Knowlu's own `confidence` field is the pipeline's weak link.

## What it is

**Maker, release, class.** TypeSafe AI, a San Francisco company founded in 2024, announced Jev on
2026-09-15 in limited early access behind a waitlist, with a reported $40M seed led by DCVC. It is a
"System One model", after Kahneman's fast, intuitive System 1: built for "fast, structured decisions
that software can use directly" rather than chat — and **not a fast or non-reasoning mode of a chat
model**, since such a mode still emits text and Jev emits none. Given a `state` (string, object or
array) and a map of typed `questions`, it evaluates every question in one parallel pass and returns
one typed answer each. The three question types are its whole output vocabulary [typesafe.ai blog;
Wikipedia; docs.typesafe.ai/api]:

- **noul** — a calibrated probability 0–1 that a proposition is true; the probability *is* the
  certainty, there being no separate confidence field.
- **choice** — a pick from up to 255 supplied options, with a distribution and a confidence 0–1.
- **score** — a rank on a supplied rubric of two to **at most ten** levels, with a legend, a
  distribution and a confidence.

**Sizes, weights, licence.** Not disclosed: no parameter count, no architecture beyond
"transformer-based, trained exclusively on synthetic data" by a method called Reinforcement Learning
for Calibrated Decisions. The weights are **proprietary** — no open-weights release and no model
licence to evaluate beyond the ordinary API terms [Wikipedia; typesafe.ai blog].

**Modalities, context, versions, latency.** Text only — strings, JSON objects, text arrays; no image
input "yet". 64k tokens per request, of which 32k caps state plus the longest question (Cloudflare's
listing states 32k); Knowlu's largest prompt is about 950 tokens, so context is no constraint. One
model, `jev-1.13.0`, aliased `jev-latest` and `jev-preview`. Latency is 70–500 ms against seconds
for a chat model — worth little here, the judge step being a background slot
[docs.typesafe.ai/models; developers.cloudflare.com; typesafe.ai blog].

## Where it runs and what it costs

**First party.** One endpoint, `POST https://api.typesafe.ai/v1/systemone`, carrying `state`,
`model` and a `questions` map. **$0.042 per million input tokens; output tokens are free** — no
output charge because there is no output text. Rate limits 250,000 tokens/second and 1,200
requests/minute, "adjusting dynamically" and changeable "without notice"; access is a waitlist. That
price undercuts the cheapest yardstick the 2026-09-16 study used (GPT-5 Nano, $0.05/M in) and is a
quarter of the Granite 4.2 8B input price Knowlu pays today [docs.typesafe.ai/models;
simonwillison.net, 2026-09-21].

**Hosted routes.** All re-sell one proprietary model; no precision is published:

| route | id | price | notes |
|---|---|---|---|
| Cloudflare Workers AI | `typesafe/jev` | not on the published pricing page (dashboard only) | 32k context; the model page links to TypeSafe's legal terms |
| OpenRouter | `typesafe/jev-1.13`, `typesafe/jev-latest` | not published on a fetchable page | listed **in beta**; a Vercel AI Gateway listing exists, unchecked |

Two things are **unverified** and matter: whether Cloudflare runs Jev on its own GPUs (its "we do
not store this data nor do we train models using it" then governing) or routes to TypeSafe — whose
own wording, "if you call Jev through Vercel, OpenRouter or Cloudflare, that platform's data policy
applies to your request **as well**", reads as additive; and whether OpenRouter marks the TypeSafe
endpoints zero-data-retention. The second is load-bearing: `assertPinnedRoute` refuses any row whose
route is not `zdr: true` with one pinned upstream, so without that flag no Jev row can be built.

**Does the strictest self-serve tier exist anywhere for it?** No. Zero data retention is offered
**to enterprise customers under a DPA**, via privacy@typesafe.ai — sales-gated, not a toggle
[docs.typesafe.ai/legal; opper.ai]: the posture the 2026-09-16 legal report found at Anthropic,
OpenAI and Google, and the reason that study moved to self-serve ZDR hosts. An open-weights sibling
exists and is out of scope: `kev` (0.8B, 4B, 9B on Qwen3.5, about $95 of H100 time, no Jev outputs
used) and `reflex` re-created the shape within days, Kev-9B scoring 0.822 against Jev's 0.857 on the
authors' development set — but both are self-hosted, which ruling 3 excludes [josharsh/kev;
jaredpalmer/kev; runtimewire].

## Capability evidence

**The fit test, against our own schemas in `judge_prompts.ts`**, field by required field:

| field (kind) | Jev type | fits? |
|---|---|---|
| `verdict` (event), `tier` (email), `course` (task, email) — closed sets | choice | yes |
| `importance` (task, email) — integer 1–5 | choice or score | yes |
| `effort_hours` (task, email) — a number 0.25–40 | score, **max 10 levels** | only as buckets — resolution lost |
| `confidence` (all three) | native, calibrated | yes, and better |
| `importance_reason` (task), `why` (event, email) — free lines | — | **no** |
| `title` (email) — a generated name, <200 chars | — | **no** |
| `due` (email) — a date resolved against the Date line | — | **no**, twice over |

Every one is `required` in its schema and `judge_validate.ts` fails an incomplete reply.
`importance_reason` is written into task frontmatter (`engine/src/approvals.rs` lists and quotes it)
and `why` rides event and email verdicts through `engine/src/cloudmodel.rs` — user-visible surface,
so dropping them is a product decision, not a provider swap.

**The date problem is not incidental.** The jaggedness page for `jev-1.13` (last reviewed
2026-09-17): "reads dates as text, not as ordered quantities. Asking which of two dates comes first,
how far apart they are, or whether one falls inside a window is unreliable"; "does not count
reliably"; "Jev is not a calculator"; and content "written to adversarially steer the model … can
move the answer". The email prompt's `due` rule asks for exactly the forbidden operation, on the one
kind that reads arbitrary inbound mail.

**Independent evidence, and it is mixed.** Competitive on classification, but not leading:

| benchmark (date) | task | Jev | best comparator |
|---|---|---|---|
| Jevals, 31,500 labelled decisions (2026-09-18) | noul (PubMedQA) | 69.0 | Gemini 3.8 Flash 73.0 |
| Jevals | choice (Banking77) | 67.8 | Gemini 3.8 Flash 74.1 |
| Jevals | score (HelpSteer2) | 9.2 | GLM-5.3 7.8 |
| dhruvmehra/jevbench, n=500 (2026-09-22) | AG News accuracy | 84.3–85.8% | fine-tuned BERT 91.0%, Claude Sonnet 5 89.6% |
| same | Banking77 accuracy | 76.4% | BERT 88.0%, Sonnet 5 77.4%, GPT-5-mini 73.6% |
| JevBench v1.3.0, 52 systems / 534 decisions | composite | 74.4 (1st) | SemIf 73.1, djev 73.0 |

That composite is a quarter each intelligence, calibration, speed and cost, so first place there is
largely price and latency; the n=500 runs carry ±2.5 points, and Jev leads no dataset there, tying
Sonnet 5 on SST-2 at 95.4% to 95.6% [jevals.com; dhruvmehra/jevbench; benchmarkheaven.com].

**Multilingual and schema adherence.** Knowlu's portals are English and Spanish; TypeSafe states
English is the primary training language and where accuracy is best, other languages "not equally
well", with no Spanish figure published — **unverified, and a real risk for the task kind**, whose
bodies come from VHL. Schema adherence is the one axis Jev wins outright, and unfairly: with no JSON
to parse, the study's worry about `gpt-oss-20b` failing compliance one call in seven cannot arise —
but that study also found compliance already above 95% for every candidate and the live gap to be
**value accuracy**, where Jev sits in the same band as the current pins, not above.

## Cost for Knowlu

From §2 of the 2026-09-16 note: typical persona 1,072,500 in / 130,500 out tokens a month (task
142.5K/12K, event 255K/21K, email 675K/97.5K); heavy 4,020,000 / 483,000. Today's pinned prices.

| kind | today's pin and price | today $/mo | Jev $/mo | usable on Jev? |
|---|---|---|---|---|
| task | granite-4.2-8b, $0.10 / $0.15 | 0.016 | 0.006 | no (`importance_reason`; `effort_hours` needs >10 levels) |
| event | granite-4.2-8b, $0.10 / $0.15 | 0.029 | 0.011 | only if `why` is dropped |
| email | qwen3.5-35b-a3b, $0.14 / $1.00 | 0.192 | 0.028 | no (`title`, `due`) |
| **all three, typical** | | **0.237** | **0.045** | hypothetical only |
| **all three, heavy** | | **0.870** | **0.169** | hypothetical only |

Realistic variants, both assuming the rationale strings are dropped or templated in code: event only
on Jev, $0.219 a month (saving $0.018); task and event on Jev, $0.209 (saving $0.028). **The
achievable saving is 1.8 to 2.8 cents per student per month** — 0.2–0.3% of the $9.99 price, against
a service already inside the $0.25 ceiling. Ruling 2 is satisfied and Jev offers no quality win;
`MONTHLY_CEILING_USD` would not move. One caveat, **unverified**: Jev bills `state` plus every
question's instructions and criteria, so real input tokens are probably above the 950 the study
measured — moving the Jev column up, never down.

## Legal and privacy

| requirement | TypeSafe's published position | verdict |
|---|---|---|
| no training on inputs | "We will not train or fine tune any artificial intelligence or machine learning models on your prompts or other Input" | **passes** |
| retention bounded, abuse only, ≤30 days (task/event class) | retained "for as long as reasonably necessary to provide you with the Services"; no period, abuse monitoring not addressed | **fails** — unbounded on its face |
| zero retention, self-serve (email class) | ZDR "for enterprise customers" under a DPA, via privacy@typesafe.ai | **fails** — sales-gated |
| human review | not addressed | **unverified** |
| a DPA, sub-processors, region | yes; list at trust.typesafe.ai/subprocessors; "The Services are hosted in the United States" | passes |
| age, commercial use | "We do not knowingly collect … personal data from children under 18 years of age"; ordinary API terms, proprietary weights, no model licence to review | passes |

**The email kind is out on privacy alone**, before `title` and `due`: Gmail text under Google's
Limited Use policy needs the strictest self-serve tier and TypeSafe has none. Under the 2026-09-16
legal report's reading, sensitive-scope Calendar events sit in the same class, and because `models`
pins per kind and not per origin, an event pin would carry Calendar-derived data to a provider with
unbounded retention. That leaves ICS-origin events and the task kind arguable at best — their "≤30
days" floor is not met either. The process cost stands whatever the unread DPA and MCA say: "Changes
to this policy" binds Knowlu to a new `privacy_version`, an email to every account and an in-app yes
**before the first request reaches a new company** — for the event kind alone, under two cents a
month.

## What would change in the code

**What the seam absorbs.** `modelFor` is a switch on `row.provider`, so `typesafe` is one case plus
one `API_KEY_ENV` entry, and the pin is a migration row; `judge_pipeline.ts` does not care who
answered.

**What it does not.** `ModelRequest` is `{model, system, user, schema, maxTokens, sampling, route}`;
Jev takes `{state, model, questions}` and returns `{answers, usage}` — no system/user split, no JSON
schema, no `max_tokens` (nothing is generated), and **no `reasoning` flag**, the switch the
2026-09-16 study added to `sampling` having no counterpart, so a Jev row's `sampling` would be `{}`.
So `buildPrompt` needs a sibling emitting a `questions` map per kind — a second prompt
representation, not a second client — and `promptHash`, which hashes `systemTemplate(kind)` plus
`schemaFor(kind)`, must hash that map instead or log a hash of a prompt never sent; `prompt_version`
becomes `event-3-jev`, `grammar_version` loses its meaning, and `assertPinnedRoute` being
OpenRouter-specific, a direct TypeSafe client weakens the zero-retention guarantee from a
per-request assertion to a contract — what that function exists to prevent. `judge_validate.ts`
needs a Jev path; the email prompt's reference-date rule cannot be ported at all, needing a
deterministic TypeScript date parser instead; and `run_eval.ts`'s field-by-field metrics make a Jev
variant missing two fields not comparable to the stored results.

**Paper versus live, if Quinn wants one anyway.** Paper first, as ruling 4 requires: read the DPA
and MCA for a bounded retention figure and a human-review clause; confirm whether OpenRouter marks
the endpoint ZDR; then re-run the event kind's stored eval cases as three-option `choice` questions
against the same labels, scoring `weighted_exact` and, separately, the **calibration** of the
returned probability — the only number worth the effort. If it is good, the live step is the event
kind on Quinn's own vault after P3, old pin against new on the same items.

## Open questions

1. Does the DPA or the Master Customer Agreement state a retention period? Neither was read in full.
   Does Cloudflare run `typesafe/jev` on its own GPUs (its no-store policy governing) or route to
   TypeSafe? Its model page links to TypeSafe's legal terms, which suggests routing. Unverified.
2. Does OpenRouter mark the TypeSafe endpoints zero-data-retention? Sources conflict, and
   `assertPinnedRoute` makes the answer binary for us. Any Spanish evaluation? None found.
3. Would Quinn accept losing `why` and `importance_reason` as written rationale, replaced by a
   probability and a templated line? A product question, and the precondition for Jev anywhere here.
4. Separately from Jev: `confidence` is a self-report gated at 0.6, already doubted by the
   2026-09-16 study. Measuring our own calibration needs no new provider and no spend.

## Sources

All read 2026-09-22 unless the item names its own date.

- https://typesafe.ai/blog/introducing-system-one-models-and-jev — launch post (2026-09-15).
- https://en.wikipedia.org/wiki/Jev_(AI_model) (maker, proprietary weights, synthetic training
  data); https://simonwillison.net/2026/Sep/21/jev/ (independent write-up: API shape, price).
- https://docs.typesafe.ai/models (versions, context, price, rate limits, languages);
  https://docs.typesafe.ai/api (`/v1/systemone` schema, question types, 255-option/10-level limits);
  https://docs.typesafe.ai/model-jaggedness/jev-1.13 (the weaknesses, last reviewed 2026-09-17).
- https://docs.typesafe.ai/legal, https://typesafe.ai/legal/privacy-policy,
  https://typesafe.ai/legal/data-processing — training, retention, ZDR tier, region, age, DPA.
- https://developers.cloudflare.com/ai/models/typesafe/jev/ (`typesafe/jev`, 32k context),
  https://developers.cloudflare.com/workers-ai/platform/pricing/index.md (not listed),
  https://www.cloudflare.com/trust-hub/responsible-ai/ (no-store statement).
- https://openrouter.ai/typesafe (`typesafe/jev-1.13`, `jev-latest`, in beta),
  https://openrouter.ai/docs/guides/features/zdr (its ZDR filter);
  https://opper.ai/provider/typesafe (ZDR enterprise-only, GDPR DPA, training posture).
- https://jevals.com/ (31,500 human-labelled decisions, release 2026-09-18);
  https://github.com/dhruvmehra/jevbench and its `docs/results/2026-09-22-n500-summary.md`
  (accuracy, ECE, latency, cost against BERT and two LLMs);
  https://benchmarkheaven.com/jev-models, https://jevbench.xyz/ (JevBench v1.3.0 composite).
- https://github.com/jaredpalmer/kev, https://github.com/josharsh/kev,
  https://runtimewire.com/article/jared-palmer-kev-qwen35-decision-models — the open re-creation.
