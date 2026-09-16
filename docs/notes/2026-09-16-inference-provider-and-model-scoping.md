# Inference provider and model scoping — a cheaper open-weights pin, per kind, within the privacy floor

**Status: DRAFT for Quinn's ruling, 2026-09-16.** Written after C2 merged, from three research
reports commissioned the same day (preserved as
`docs/reports/2026-09-16-inference-scoping-{legal-floor,provider-offers,model-capability}.md`;
every price, policy sentence and benchmark number below carries its URL and read date there). This
note argues from the signed cloud design (`docs/specs/2026-09-09-knowlu-cloud-design.md`) and
proposes an amendment to its ruling **R8** (Anthropic as the launch provider). Nothing here is
decided until Quinn rules in §8; nothing here is legal advice.

## 1. What Quinn asked, and the four rulings the study ran under

The ask (2026-09-16): a cheaper, non-Anthropic, open-weights model; the best privacy the setup allows;
retention is negotiable where it buys price; every provider's offers compared; quantization
considered; the newest models; optimise for intelligence and cost. Four clarifying rulings framed the
work:

1. **The privacy floor is split by data class.** Task and event judgments may use a provider that keeps
   prompts up to 30 days for abuse monitoring (no training, no human review). Email judgments stay at
   the strictest self-serve tier, bound by Google's Limited Use policy and by what the privacy page
   already promises.
2. **The cost rule** is the best model under **$0.25 per student per month** at the typical persona;
   pricier needs a measured quality win.
3. **Sources**: hosted open-weights APIs, with the majors' cheapest tiers as yardsticks. No self-hosting.
4. **Validation**: a paper study now; the live comparison later on Quinn's own vault, after P3.

## 2. The cost reality, corrected

The provider report re-derived the call profile from the prompt code rather than from the brief's
estimate. Every kind clips its text to the same 1,200 characters, so an email call is not larger than a
task call; the per-call ceiling is about 950 tokens in and 80 to 130 out. With the personas from the
brief (a typical student: 5 task, 10 event, 25 email judgments a day; a heavy one: 20, 40, 90):

| | tokens in / month | tokens out / month | formula ($/M prices) |
|---|---|---|---|
| typical | 1,072,500 | 130,500 | 1.0725 × in + 0.1305 × out |
| heavy | 4,020,000 | 483,000 | 4.020 × in + 0.483 × out |

**The current pin, Haiku 4.5 at $1.00 / $5.00, costs $1.73 a month for a typical student and $6.44 for
a heavy one** — 17 percent of revenue at the typical persona, seven times the ceiling Quinn set, and the
number the enforced `MONTHLY_CEILING_USD = 7.5` was sized around. An earlier message from the
controller called the model bill "well under a dollar"; that was wrong, and this table replaces it.

## 3. The legal floor, per data class (from the legal report)

The report's three findings that move the boundaries, then the verdicts.

- **Zero data retention is self-serve and free at several open-weights hosts**: Groq (a Data Controls
  toggle, a DPA that incorporates automatically, US-only storage), Fireworks (the default for open
  models), Together (a settings toggle), Baseten (the product default), Nebius (a mode), Cloudflare
  Workers AI ("we do not store this data nor do we train models using it"), and AWS Bedrock (a ZDR
  model by default, with model providers given no access). The majors gate it behind sales: Anthropic
  (30 days by default, up to two years on a detected violation), OpenAI (approval), Gemini paid tier
  ("a limited period", unstated) and Vertex (30 days). **So the current Anthropic self-serve pin is
  not on the "zero-retention terms" the design's §5.2 asserts.** That sentence in the spec is wrong
  today, whatever is decided here.
- **Google's Limited Use binds Calendar data as hard as Gmail**: the policy applies "to data derived
  from both Sensitive and Restricted scopes", and `ingest-calendar` reads events with the sensitive
  `calendar.readonly` scope. Ruling 1's loose bucket therefore holds only for events that came from an
  ICS feed. The `models` table pins per kind, not per origin, so an `event` pin covers both origins.
- **Nothing redacts the email prompt.** The pipeline's `scrub()` is used only by the issues handler;
  the first 1,200 characters of a message body reach the provider verbatim, magic links and reset
  tokens included. Under zero retention that is transient; under a 30-day log it is not.

The verdicts:

| data class | minimum provider terms | providers meeting it self-serve |
|---|---|---|
| LMS task data (title, body ≤1,200 chars, course, weights, preferences) | no training; retention only for abuse and ≤30 days; no discretionary human review; a DPA or service-provider terms | every provider in the table except Mistral (trains by default on some products) and the Gemini free tier (trains and human-reviews) |
| Events from an ICS feed | as task data | as task data |
| Events from the Google Calendar API | as Gmail | as Gmail |
| Gmail message text | no training anywhere in the chain; human review only for security; retention as short as offered, which self-serve means **zero**; the provider nameable on the privacy page | Groq, Together, Fireworks, Baseten, Nebius (ZDR on), Cloudflare Workers AI, AWS Bedrock (models outside its abuse-detection exception list) |
| Consented corrections as eval cases | as task data (no message text is ever stored, by construction) | any of the above — but never a corpus of live inbox text |

Two more consequences the report draws. A 30-day abuse log **is** compatible with Limited Use for
Gmail (the policy has no retention clause and both transfer and retention fall inside its security
exception); it is Knowlu's own published sentences that would have to change, and the report drafts
each replacement. And the privacy page's "Changes to this policy" clause binds Knowlu to a new
`privacy_version`, an email to every account and an in-app yes **before the first request goes to a
new company** — a provider swap is a product event, not a config change, and cheap only while the
account count is one.

OpenRouter is a router, not a processor: the set of companies touching a request changes per request,
which the page's "a new company on the list" clause cannot describe. Usable for price discovery and
eval, not as the production path.

## 4. The providers and prices that survive both filters (from the provider report)

All list prices as of 2026-09-16, primary-sourced unless marked (A) for aggregator. Monthly figures are
the typical persona for the whole student on that one model; a per-kind mix is computed in §6.

| provider | model | precision declared | $/M in | $/M out | typical $/mo | strict class? |
|---|---|---|---|---|---|---|
| Cloudflare | qwen3-30b-a3b-fp8 (30B MoE, 3B active) | fp8 | 0.051 | 0.335 | 0.098 | yes |
| Cloudflare | gemma-4-26b-a4b-it | unspecified | 0.10 | 0.30 | 0.146 | yes |
| Cloudflare | glm-5.3-flash | unspecified | 0.15 | 0.50 | 0.226 | yes |
| Cloudflare | gpt-oss-20b (20B MoE, 3.6B active) | unspecified | 0.20 | 0.30 | 0.254 | yes |
| Cloudflare | gpt-oss-120b (120B MoE, 5.1B active) | unspecified | 0.35 | 0.75 | 0.473 | yes |
| Cloudflare | llama-3.3-70b-instruct-fp8-fast | fp8 | 0.293 | 2.253 | 0.608 | yes |
| Groq | gpt-oss-20b | Groq's own 8-bit numerics | 0.075 | 0.30 | 0.120 | yes |
| Groq | gpt-oss-120b | as above | 0.15 | 0.60 | 0.239 | yes |
| Together | DeepSeek V4-Flash (MoE) | unspecified | 0.14 | 0.28 | 0.187 | yes |
| Together | Qwen3.8 Flash | unspecified | 0.15 | 0.47 | 0.222 | yes |
| Together / Fireworks | gpt-oss-120b | unspecified | 0.15 | 0.60 | 0.239 | yes |
| Baseten | gpt-oss-120b (A) | unspecified | 0.10–0.15 | 0.50–0.60 | 0.17–0.24 | yes |
| DeepInfra | Mistral-Small-3.2-24B (A) | unspecified | 0.075 | 0.20 | 0.106 | task/ICS only (no DPA found) |
| Mistral | Ministral 8B / Small 4 (A) | unspecified | 0.15 | 0.15–0.60 | 0.18–0.24 | **no** (training default) |
| Anthropic (the pin) | Haiku 4.5 | n/a | 1.00 | 5.00 | 1.73 | **no** at self-serve |

Three cautions from the report's own "could not verify" list: Groq's Llama 3.1/3.3 rows may have moved
to enterprise-only on 2026-08-26 (four aggregators say so; Groq's docs still list them) and need a
key-in-hand check; Vertex, Bedrock, Nebius and Hyperbolic prices were not reliably extracted; and no
published benchmark measures quantization's effect on JSON-schema-constrained classification, which is
exactly Knowlu's task. On quantization generally: fp8 is near-lossless at every size; int4 (AWQ/GPTQ)
costs about 1 point on 70B-class models and about 7 points on 8B-class ones on the published
instruction-following evidence, so the cheapest rows are the ones most exposed, and a provider that
does not declare precision should be assumed to quantize.

## 5. What each kind needs, and which models meet it (from the model-capability report)

The capability report read the three prompts, the schemas, the pipeline's tiers and the eval's metrics
before touching a benchmark. Its conclusion: **capability is not the constraint at this size class; two
operational details are, and the prompts are the higher-leverage work.**

**What each kind demands.** Task is the easiest: closed-vocabulary classification and a few short fields
from a title, a course and a clipped body. Event is rule application over structured fields with an
asymmetric cost: the prompt gives three drop rules and one obligation rule, a small model reads "drop"
as the default, and `score.ts` prices an obligation judged as a drop at 3, the most expensive single
error in the suite. Email is the hardest and not close: reading a clipped message, deciding "do
nothing" for the majority of mail, extracting eight fields, and resolving a date. The single most
dangerous line is `judge_prompts.ts:116`: `due` has no reference date and no relative-date rule, so "by
Friday" becomes a well-formed wrong date that the validator writes into the vault silently. That is a
prompt defect to fix now, whatever the provider.

**The evidence that matters most** is the Structured Output Benchmark (arXiv:2604.25359, 2026-04-29,
5,000 schema-plus-ground-truth records), because it measures value-level correctness of
schema-conforming JSON, which is exactly what Knowlu's validator cannot see. Structural compliance is
solved (most models above 95 percent JSON pass); value accuracy sits 15 to 30 points lower and is what
the eval's `weighted_exact` and `importance_exact` will measure. On that board:

| model | JSON pass | value accuracy | open weights |
|---|---|---|---|
| GLM-4.7 | 0.972 | 0.830 | yes |
| **Qwen3.5-35B (3B active)** | 0.974 | **0.828** | **yes** |
| GPT-5.4 | 0.999 | 0.825 | no |
| Gemini-2.5-Flash | 0.983 | 0.822 | no |
| Claude-Sonnet-4.6 | 0.984 | 0.809 | no |
| Gemma-3-27B | 0.975 | 0.803 | yes |
| Gemma-4-31B | 0.945 | 0.798 | yes |
| Qwen3-30B | 0.988 | 0.778 | yes |
| IBM Granite 4.0 | 0.985 | 0.761 | yes |
| **gpt-oss-20B** | **0.858** | 0.693 | yes |

Two consequences. **gpt-oss-20b is disqualified**: it fails schema compliance one call in seven, which
the pipeline records as no verdict at all, and gpt-oss-120b's instruction-following score (IFBench
0.695) does not argue for the larger one. And **Qwen3.5-35B-A3B is the strongest open-weights model on
the board, effectively tied for first overall**, from a mixture-of-experts model with three billion
active parameters that prices like an 8B. On instruction following (IFBench, vendor-reported) IBM's
Granite 4.2 8B leads its size at 0.793, Qwen3.5-27B scores 0.765 and Qwen3.5-35B-A3B 0.702; no IFBench
figure exists for any yardstick, so open models rank against each other but not against Haiku on that
axis. On general reasoning the 30B-class open models beat Haiku 4.5 by 5 to 20 points (MMLU-Pro 80.0 /
GPQA 64.6 for the pin).

**The picks, per kind.** Task and event: **IBM Granite 4.2 8B** (2026-08-25, Apache 2.0, dense, fp8 or
bf16 and never int4) first, for its instruction following, its explicit non-thinking mode and German as
a supported language; its uncertainty is that the 4.2 generation is not on the structured-output board
and 4.0 scored mid-pack. Email: **Qwen3.5-35B-A3B** (2026-02, Apache 2.0, fp8). If one model must serve
all three kinds, Qwen3.5-35B-A3B. The large tier (GLM-5.3-Flash, DeepSeek V4 Flash) fits the ceiling on
list price but is a first-party Chinese API unless a US host serves it, which the legal filter decides.
Do not build confidence-triggered escalation: self-reported confidence tracks commitment, not
correctness, and three schema facts stand in its way; splitting by kind is the same benefit for three
migration rows and no code, and a structural retry on a truncated or failed reply belongs inside the
adapter.

**Quantization.** fp8 is near-lossless at every size. int4 (AWQ, GPTQ) costs about one point on
70B-class models and about seven on 8B-class ones on the published instruction-following evidence, so
the cheapest rows are the most exposed. A provider that does not declare precision should be assumed
to quantize; ask, and pin the precision beside the model id.

**Two gaps in the seam, both blocking before any pin.** First, every credible open-weights candidate is
thinking-on by default, and against `max_tokens` of 256/256/640 a thinking model returns a length stop
on every call, which the pipeline records as truncated and the device reports as a failed judgment: it
would look like a total outage. The `models.sampling` column is already free-form JSON spread into the
request, so a pinned row can carry the provider's off switch as soon as the adapter widens the
TypeScript `Sampling` type; no migration. Second, vLLM issue #18819 (open) breaks guided JSON on Qwen
models exactly in the non-thinking plus JSON-schema configuration Knowlu needs, and was reproduced on
quantized builds. So the first act against any provider is ten schema-constrained non-thinking requests
with the JSON checked, and `grammar_version` stays in the row because a GBNF fallback already exists.

**Hosting still to reconcile.** The provider report priced today's catalogues and did not find the two
picks at a self-serve zero-retention host (Cloudflare serves the previous generation, `qwen3-30b-a3b-fp8`
at $0.051 / $0.335 and a Granite 4.0 micro; Together lists a Qwen3.8 Flash; Groq's catalogue centres on
gpt-oss). Finding which of Groq, Together, Fireworks or Cloudflare serves Granite 4.2 8B and
Qwen3.5-35B-A3B self-serve, with zero retention, at fp8 and at what price is the swap task's first
step. The priced fallbacks already in §4 are `qwen3-30b-a3b-fp8` at Cloudflare for task and event
(value accuracy 0.778) and DeepSeek V4 Flash at Together or GLM-5.3-Flash at Cloudflare for email.

## 6. The mixes that fit the ceiling

Per-kind pins let the strict class carry the strongest model the budget allows while the cheap classes
ride a smaller one. Using §2's token volumes split by kind (typical: email 675K in / 97.5K out; event
255K / 21K; task 142.5K / 12K):

| mix | email + Google-Calendar events | task + ICS events | typical $/mo | heavy $/mo | new parties |
|---|---|---|---|---|---|
| **A. one host, Groq** | gpt-oss-120b ($0.15/$0.60) | gpt-oss-20b ($0.075/$0.30) | 0.23 | 0.84 | one (Groq) |
| **B. one host, Cloudflare** | gpt-oss-20b ($0.20/$0.30) | qwen3-30b-a3b-fp8 ($0.051/$0.335) | 0.23 | 0.87 | none |
| **C. split** | gpt-oss-120b at Groq | qwen3-30b-a3b-fp8 at Cloudflare | 0.22 | 0.82 | one (Groq) |
| D. stay | Haiku 4.5 | Haiku 4.5 | 1.73 | 6.44 | none |

Every mix but D also lets `MONTHLY_CEILING_USD` fall from 7.5 to about 2, which is what actually caps a
runaway account. In all three the `event` kind takes the strict provider whole: the 300 event calls a
month cost cents either way, and routing by origin would add code for no saving.

## 7. Three fixes that are Knowlu's own, whatever is chosen

1. **The training-export filter misses Calendar-API rows.** `20260911000200_google.sql:100` exports
   `judgments where origin <> 'gmail_api'`; rows derived from the Google Calendar API are sensitive-scope
   data under the same Limited Use terms and must be excluded too. One migration.
2. **The email prompt is not scrubbed.** Run the existing `scrub()` (or a narrower token-and-link
   pattern) over the clipped message text before the call. One pipeline change and a test.
3. **The spec's zero-retention sentence and the privacy page.** Correct §5.2 to say what the pinned
   provider's terms actually are; on any provider change, bump `privacy_version`, name the new company,
   send the email and take the in-app yes before the first call; and replace the page's "No note bodies"
   bullet with the report's draft, which is already inaccurate about the 1,200 characters a task call
   carries.

## 8. Decision menu for Quinn

Ruling R8 of the cloud design is what changes. Three options, the recommendation first.

1. **Per-kind pins on one zero-retention host (recommended).** Granite 4.2 8B for task and event,
   Qwen3.5-35B-A3B for email, at whichever of Groq, Together, Fireworks or Cloudflare serves both
   self-serve with zero retention at fp8; the swap task's first step finds out and prices it, expected
   between $0.10 and $0.25 a month for a typical student. At most one new party on the privacy page;
   the event kind takes the strict tier whole. Amend R8 to "an open-weights provider on self-serve
   zero-retention terms, models pinned per kind, the eval suite picks".
2. **Cloudflare only, with today's catalogue.** `qwen3-30b-a3b-fp8` for every kind (about $0.10 a
   month, value accuracy 0.778) or with `glm-5.3-flash` for email (about $0.19). No new party, no new
   DPA, the existing JSON mode; one generation behind on the email pick, and precision undeclared on
   most rows.
3. **Stay on Anthropic.** $1.73 a month for a typical student, seven times the ceiling; the spec's
   zero-retention sentence gets corrected to what the self-serve tier actually is, and the privacy
   page's Anthropic bullet gains the 30-day fact.

Whatever the ruling, three fixes ship first (§7), the `due` prompt line is repaired, and the lawyer's
two questions (§9) are asked before the Gmail scope goes to verification. What the study needs from
Quinn now is the number; the provider's API key on staging replaces P1 once the swap task names it.

## 9. What happens after the ruling

One SDD task, "the provider swap": a second client behind the existing `complete` interface (an
OpenAI-compatible endpoint with `response_format: json_schema` for Groq, Together and Fireworks;
Cloudflare's own JSON mode is the same shape), the dependency builder choosing the client by the
`models` row's provider, a migration re-pinning the three kinds with provider, model id, precision note
and prices, the eval runner on the same client, the CI secret renamed, and the three fixes in §7. Two
things only Quinn sets: the provider's API key on staging and, for the eval gate, in GitHub. The first
quality gate is Quinn's own vault after P3: the old pin and the new one judged side by side on the same
items, before the privacy-page version bump makes the swap real for anyone else.

Open for a lawyer (the legal report's §6, eight questions; the two that decide the email pin): whether
restricted-scope verification's "delete user data on request" reaches a sub-processor's abuse log the
founder cannot purge — if it does, zero retention is the only configuration where "Delete my data" is
unqualifiedly true for Gmail; and whether a student can validly consent to third-party processing of a
mailbox their university owns.
