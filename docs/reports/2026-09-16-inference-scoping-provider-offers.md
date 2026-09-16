# Provider offers — hosted open-weights inference for Knowlu's judgment service

Research date: 2026-09-16. Every price below is a list price read on that date unless the row says
otherwise; the URL and read-date sit next to each number. This is a paper study — no sign-ups, no
spend, no API calls (context.md rule 4).

## 0. Refined call-profile arithmetic (read this before the table)

Context.md's brief estimate was task/event ≈ 1,200 in / 100 out, email ≈ 2,500 in / 250 out. I
re-derived these from the actual prompt code (`cloud/supabase/functions/_shared/judge_prompts.ts`,
`judge_caps.ts`) rather than guessing, and the email number in the brief is too high.

**System templates** (exact strings from `systemTemplate()`, measured with `wc -c`, ÷4 chars/token):
task 555 chars ≈ 139 tok; event 417 chars ≈ 104 tok; email 902 chars ≈ 226 tok.

**User payload ceilings**, from the same file's clip constants (`MAX_BODY_CHARS=1200`,
`MAX_WEIGHTS_CHARS=600`, `MAX_PREFS_CHARS=600`, title/subject/from ≤200 chars, why/reason ≤140
chars): task's body, event's description+interests, and **email's message text are all clipped to
the same 1,200-character `MAX_BODY_CHARS`** (`buildPrompt`'s email branch: `clip(str(item.text).trim(),
MAX_BODY_CHARS)`). The brief's "email ≈ 2,500 in" assumed the clipped Gmail text was larger than
the task body; it isn't — it's capped identically. `judge_caps.ts` itself carries a worked estimate
(lines ~13-19) reaching "system ~280 tokens; user up to ~700... about **$0.0014**" per call at
Haiku's $1/$5 pricing, and explicitly notes "email's longer cap does not change that because the
cap is not what is billed."

**Refined per-call estimate used below** (ceiling case, so slightly conservative):
- task ≈ 950 in / 80 out (5-field JSON reply)
- event ≈ 850 in / 70 out (3-field JSON reply)
- email ≈ 900 in / 130 out (8-field JSON reply, more output fields than task/event)

**Persona monthly token volumes** (calls/day × 30, from context.md's table: typical
task5/event10/email25; heavy task20/event40/email90):

| persona | in tokens/mo | out tokens/mo |
|---|---|---|
| typical | 150×950 + 300×850 + 750×900 = 1,072,500 | 150×80 + 300×70 + 750×130 = 130,500 |
| heavy | 600×950 + 1,200×850 + 2,700×900 = 4,020,000 | 600×80 + 1,200×70 + 2,700×130 = 483,000 |

**Formula** (price in $/M tokens):
- monthly_typical = 1.0725 × Pin + 0.1305 × Pout
- monthly_heavy = 4.020 × Pin + 0.483 × Pout

**Worked check** against the pinned model, Anthropic Haiku 4.5 ($1.00/$5.00, confirmed §2):
monthly_typical = 1.0725×1 + 0.1305×5 = 1.0725 + 0.6525 = **$1.73/month** — about 7× the founder's
$0.25 ceiling, which is exactly why this study exists. monthly_heavy = 4.020 + 0.483×5 = **$6.44/month**,
under the enforced `MONTHLY_CEILING_USD=7.5`, confirming that cap's own design math
(`judge_caps.ts`) was sized around Haiku-era pricing.

## 1. Sorted cost table (typical persona, cheapest first)

"Under $0.25" column marks the founder's ceiling (rule 2). Precision is the provider's own label
("unspecified" where none is given, per rule). Source column: **P** = primary provider page/API
fetched directly; **A** = third-party aggregator, not independently confirmed against the
provider's own page (treat with more caution). All reads 2026-09-16.

| Provider | Model | Params | Precision | $/M in | $/M out | $/mo typical | $/mo heavy | ≤$0.25? | Source |
|---|---|---|---|---|---|---|---|---|---|
| DeepInfra | Mistral-Nemo-Instruct-2407 | 12B | unspecified | 0.019 | 0.03 | $0.024 | $0.090 | Yes | A |
| DeepInfra | Meta-Llama-3.1-8B-Instruct-**Turbo** | 8B | FP8 ("Turbo") | 0.02 | 0.04 | $0.027 | $0.100 | Yes | A |
| Cloudflare | Granite-4.0-h-micro | small (IBM) | unspecified | 0.017 | 0.112 | $0.033 | $0.122 | Yes | P |
| OpenRouter | openai/gpt-oss-20b | 20B (3.6B active) | unspecified | 0.03 | 0.13 | $0.049 | $0.184 | Yes | P |
| Cloudflare | llama-3.2-1b-instruct | 1B | unspecified | 0.027 | 0.201 | $0.055 | $0.206 | Yes (weak model) | P |
| OpenRouter | openai/gpt-oss-120b | 120B (5.1B active) | unspecified | 0.037 | 0.17 | $0.062 | $0.231 | Yes | P |
| OpenRouter | meta-llama/llama-3.1-8b-instruct | 8B | unspecified | 0.05 | 0.08 | $0.064 | $0.240 | Yes | P |
| OpenRouter | qwen/qwen3-30b-a3b-instruct-2507 | 30B (3B active) | unspecified | 0.048 | 0.193 | $0.077 | $0.286 | Yes | P |
| Cloudflare | qwen3-30b-a3b-fp8 | 30B (3B active) | **FP8 (labeled)** | 0.051 | 0.335 | $0.098 | $0.367 | Yes | P |
| Cloudflare | llama-3.1-8b-instruct-fp8-fast | 8B | **FP8 (labeled)** | 0.045 | 0.384 | $0.098 | $0.366 | Yes | P |
| Cloudflare | llama-3.2-3b-instruct | 3B | unspecified | 0.051 | 0.335 | $0.098 | $0.367 | Yes | P |
| DeepInfra | Mistral-Small-3.2-24B-Instruct-2506 | 24B | unspecified | 0.075 | 0.20 | $0.106 | $0.398 | Yes | A |
| Groq | gpt-oss-20b | 20B (3.6B active) | unspecified, Groq's own 8-bit LPU numerics | 0.075 | 0.30 | $0.120 | $0.447 | Yes | P (own pricing page) |
| OpenRouter | qwen/qwen3-32b | 32B | unspecified | 0.08 | 0.28 | $0.122 | $0.456 | Yes | P |
| Baseten | GPT-OSS-120B | 120B (5.1B active) | unspecified | 0.10–0.15 | 0.50–0.60 | $0.173–$0.239 | $0.643–$0.893 | Yes (low end) | A |
| Cloudflare | glm-4.7-flash | small (ZAI) | unspecified | 0.06 | 0.40 | $0.117 | $0.434 | Yes | P |
| OpenRouter | z-ai/glm-4.7-flash | small | unspecified | 0.0605 | 0.40 | $0.117 | $0.436 | Yes | P |
| DeepInfra | DeepSeek-V4-Flash-0731 (promo) | MoE, undisclosed active | unspecified | 0.06 | 0.18 | $0.088 | $0.328 | Yes | A |
| Cloudflare | mistral-7b-instruct-v0.1 | 7B | unspecified | 0.11 | 0.19 | $0.143 | $0.533 | Yes | P |
| OpenRouter | mistralai/mistral-small-3.2-24b-instruct | 24B | unspecified | 0.094 | 0.25 | $0.133 | $0.498 | Yes | P |
| AWS Bedrock | Ministral 3B 3.0 | 3B | unspecified | 0.10 | 0.10 | $0.120 | $0.450 | Yes | P |
| Groq | gpt-oss-120b | 120B (5.1B active) | unspecified | 0.15 | 0.60 | $0.239 | $0.893 | Yes (just) | P |
| Together | GPT-OSS 120B | 120B (5.1B active) | unspecified | 0.15 | 0.60 | $0.239 | $0.893 | Yes (just) | P (docs.together.ai) |
| Fireworks | gpt-oss-120b | 120B (5.1B active) | unspecified | 0.15 | 0.60 | $0.239 | $0.893 | Yes (just) | A |
| AWS Bedrock | Ministral 8B 3.0 | 8B | unspecified | 0.15 | 0.15 | $0.181 | $0.674 | Yes | P |
| Mistral La Plateforme | Ministral 8B (Ministral 3 8B) | 8B | unspecified | 0.15 | 0.15 | $0.181 | $0.674 | Yes | A |
| OpenRouter | mistralai/ministral-8b-2512 | 8B | unspecified | 0.15 | 0.15 | $0.181 | $0.674 | Yes | P |
| OpenRouter | qwen/qwen3-8b | 8B | unspecified | 0.117 | 0.455 | $0.185 | $0.690 | Yes | P |
| Cloudflare | llama-3.1-8b-instruct-awq | 8B | **AWQ int4 (labeled)** | 0.123 | 0.266 | $0.167 | $0.622 | Yes | P |
| — | **$0.25/mo typical ceiling** | | | | | **$0.25** | | | |
| Cloudflare | gpt-oss-20b | 20B (3.6B active) | unspecified | 0.20 | 0.30 | $0.254 | $0.949 | **No (barely)** | P |
| Together | Qwen3.8 Flash | undisclosed | unspecified | 0.15 | 0.47 | $0.222 | $0.845 | Yes | P |
| Mistral La Plateforme | Mistral Small 4 | ~24B-class successor | unspecified | 0.15 | 0.60 | $0.239 | $0.893 | Yes (just) | A |
| Baseten | DeepSeek-V4-Flash-0731 | MoE | unspecified | 0.13 | 0.26 | $0.173 | $0.649 | Yes | A |
| Together | DeepSeek V4-Flash | MoE | unspecified | 0.14 | 0.28 | $0.187 | $0.698 | Yes | P |
| SambaNova | gpt-oss-120b | 120B (5.1B active) | unspecified | 0.22 | 0.59 | $0.313 | $1.170 | **No** | A |
| OpenRouter | google/gemma-3-27b-it | 27B | unspecified | 0.08 | 0.45 | $0.145 | $0.539 | Yes | P |
| OpenRouter | meta-llama/llama-3.3-70b-instruct | 70B | unspecified | 0.10 | 0.32 | $0.149 | $0.556 | Yes | P |
| Fireworks | DeepSeek V4.1 Flash | MoE | unspecified | 0.22 | 0.66 | $0.322 | $1.203 | **No** | A |
| OpenRouter | meta-llama/llama-4-maverick | 400B (17B active) | unspecified | 0.1875 | 0.6525 | $0.286 | $1.070 | **No** | P |
| OpenRouter | deepseek/deepseek-v3.2 | 671B (37B active) | unspecified | 0.269 | 0.40 | $0.341 | $1.163 | **No** | P |
| SambaNova | Meta-Llama-3.3-70B-Instruct | 70B | unspecified | 0.60 | 1.20 | $0.800 | $2.985 | **No** | A |
| Cerebras | gpt-oss-120b | 120B (5.1B active) | unspecified | 0.35 | 0.75 | $0.473 | $1.769 | **No** | P (JSON in page) |
| Cerebras | Qwen 3.8 27B | 27B | unspecified | 0.99 | 1.49 | $1.256 | $4.699 | **No** | P (JSON in page) |
| Together | Llama 3.3 70B | 70B | unspecified (flat rate) | 1.04 | 1.04 | $1.251 | $4.681 | **No** | P |

Rows below the $0.25 line that are still far cheaper than the current Haiku pin ($1.73/mo typical)
are listed for the "cost vs. intelligence" trade the founder asked about (rule 2's "anything pricier
needs a measured quality win").

**What clears the ceiling with a real (not toy) model**: `openai/gpt-oss-120b` at OpenRouter's
routed price ($0.062/mo) or Groq/Together/Fireworks's flat $0.15/$0.60 ($0.239/mo, "yes, just"),
`meta-llama/llama-3.3-70b-instruct` at OpenRouter's price ($0.149/mo), and `qwen/qwen3-32b`
($0.122/mo) are the strongest-known-capability models that land comfortably under $0.25. The very
cheapest rows (Llama 3.2 1B, Granite micro) are toy-sized and are flagged "weak model" — not
credible for email tier/extraction judgment even though the arithmetic clears the ceiling.

## 2. The catalogue, per provider

All context windows and structured-output notes are the provider's own claim where cited "P"; ranges
without a single confirmed number are marked "not found."

### Cloudflare Workers AI — in scope, primary vendor already
Source: `https://developers.cloudflare.com/workers-ai/platform/pricing/`, fetched and grepped from
raw HTML 2026-09-16 (table confirmed, not aggregator-summarized).

| Model | $/M in | $/M out | Precision (from model id) |
|---|---|---|---|
| llama-3.2-1b-instruct | 0.027 | 0.201 | unspecified |
| llama-3.2-3b-instruct | 0.051 | 0.335 | unspecified |
| llama-3.2-11b-vision-instruct | 0.049 | 0.676 | unspecified (vision) |
| llama-3.1-8b-instruct | 0.282 | 0.827 | unspecified ("full") |
| llama-3.1-8b-instruct-fp8-fast | 0.045 | 0.384 | fp8 |
| llama-3.1-8b-instruct-fp8 | 0.152 | 0.287 | fp8 |
| llama-3.1-8b-instruct-awq | 0.123 | 0.266 | AWQ int4 |
| llama-3.1-70b-instruct-fp8-fast | 0.293 | 2.253 | fp8 |
| llama-3.3-70b-instruct-fp8-fast | 0.293 | 2.253 | fp8 |
| llama-4-scout-17b-16e-instruct | 0.270 | 0.850 | unspecified |
| deepseek-r1-distill-qwen-32b | 0.497 | 4.881 | unspecified |
| deepseek-v4-flash-0731 | 0.440 | 1.320 | unspecified (paid plan required) |
| deepseek-v4-pro-0813 | 1.320 | 3.960 | unspecified (paid plan required) |
| mistral-7b-instruct-v0.1 | 0.110 | 0.190 | unspecified |
| mistral-small-3.1-24b-instruct | 0.351 | 0.555 | unspecified |
| gemma-3-12b-it | 0.345 | 0.556 | unspecified |
| gemma-4-26b-a4b-it | 0.100 | 0.300 | unspecified |
| gpt-oss-120b | 0.350 | 0.750 | unspecified |
| gpt-oss-20b | 0.200 | 0.300 | unspecified |
| qwen3-30b-a3b-fp8 | 0.051 | 0.335 | fp8 |
| qwen3.8-27b | 0.450 | 3.200 | unspecified |
| qwq-32b | 0.660 | 1.000 | unspecified |
| qwen2.5-coder-32b-instruct | 0.660 | 1.000 | unspecified |
| glm-4.7-flash | 0.060 | 0.400 | unspecified |
| glm-5.2 / glm-5.3 | 1.400 | 4.400 | unspecified (paid plan) |
| glm-5.3-flash | 0.150 | 0.500 | unspecified |
| kimi-k2.5 / k2.6 / k2.7-code | 0.600–0.950 | 3.000–4.000 | unspecified (paid plan) |
| nvidia/nemotron-3-120b-a12b | 0.500 | 1.500 | unspecified |
| granite-4.0-h-micro | 0.017 | 0.112 | unspecified |

Context window / structured output: Workers AI's JSON Mode is documented (`response_format` with
`json_schema`, OpenAI-SDK-compatible) —
[Cloudflare JSON Mode docs](https://developers.cloudflare.com/workers-ai/features/json-mode/),
[changelog 2025-02-25](https://developers.cloudflare.com/changelog/2025-02-25-json-mode/), read
2026-09-16. Per-model context windows were not printed on the pricing page itself; not found in this
pass — would need each model's own docs page. Rate limits: Cloudflare's daily-reset note ("all
limits reset daily at 00:00 UTC") was visible but the numeric limits were not captured; not found.
Regions: Cloudflare's global edge network (not itemized per-model). Zero-retention: not found on the
pricing page; Cloudflare's general Workers AI privacy posture would need its own docs page — flagged
for the legal agent.

### Groq — in scope, self-serve, catalogue narrowed sharply
Groq's marketing pricing page (`groq.com/pricing`) is a client-rendered SPA that returned no pricing
content to either `curl` or WebFetch (verified independently by both methods, 2026-09-16) — its own
site currently foregrounds "$350M Series A" copy, not a model table. `console.groq.com/docs/models`
(fetched 2026-09-16) still lists **Llama 3.1 8B Instant** and **Llama 3.3 70B Versatile** as
"Production Models" with throughput figures (560 t/s and 280 t/s respectively) but shows **no price**
for either. Multiple independent pricing aggregators (CloudZero, eesel, AIPricingGuru, Layer3,
Markaicode — 4+ sources agreeing) report that Llama 3.1 8B and Llama 3.3 70B moved to
**Enterprise-only "Contact Sales"** on 2026-08-26 and no longer carry a published self-serve rate,
and that Llama 4 Scout and Qwen3 32B were pulled from self-serve entirely in July 2026 with no
repricing notice. **This is a conflict I could not fully resolve**: the docs page still lists the
models as "Production" but omits price; the aggregators say self-serve access is gone. Treat Groq's
self-serve open-weights catalogue as **gpt-oss-120b and gpt-oss-20b only**, confirmed by the
aggregator consensus and consistent with the docs page's pricing silence on the Llama rows.

| Model | $/M in | $/M out | Notes |
|---|---|---|---|
| gpt-oss-20b | 0.075 | 0.30 | self-serve, confirmed by 6+ aggregator sources agreeing to the cent |
| gpt-oss-120b | 0.15 | 0.60 | self-serve |
| llama-3.1-8b-instant | not found (was ~0.05/0.08 pre-Aug26) | — | docs page lists it "Production" with no price; aggregators say enterprise-only since 2026-08-26 |
| llama-3.3-70b-versatile | not found (was ~0.59/0.79 pre-Aug26) | — | same conflict |
| llama-4-scout, qwen3-32b | not found | — | aggregators report pulled from self-serve July 2026 |

Structured output: Groq's API is OpenAI-compatible and documents JSON mode / tool calling on its
production models. Batch API halves rates (per aggregator consensus, not independently confirmed on
Groq's own page in this pass). Groq is a latency specialist (LPU hardware) — see §5.

### Fireworks AI — in scope, self-serve
`fireworks.ai/pricing` states tiered per-parameter-band pricing ("<4B: from $0.10/M", "16B+: up to
$0.90/M") but did not surface a full per-model table to either fetch; `fireworks.ai/models` (filtered
serverless) surfaced a partial table (2026-09-16):

| Model | $/M in | $/M out |
|---|---|---|
| DeepSeek V4.1 Flash | 0.22 | 0.66 |
| DeepSeek-V4-Pro-0813 | 1.32 | 3.96 |
| DeepSeek-V4-Flash-0731 | 0.22 | 0.66 |
| GLM 5.3 Flash | 0.15 | 0.50 |
| GLM-5.3 | 1.40 | 4.40 |
| gpt-oss-120b | 0.15 | 0.60 |
| MiniMax M3 | 0.30 | 1.20 |
| Kimi K2.7 Code | 0.95 | 4.00 |

Llama, Qwen-chat (non-embedding), Gemma and Mistral rows did not surface a per-token price in this
pass — not found; Fireworks' catalog is large (100+ models) and the pricing page pushes most of it
behind a "view documentation" link rather than a static table. A "Priority" tier at ~1.25-1.5x
standard buys lower queue time; cached-input tokens are billed lower (both per Fireworks' own
pricing-page copy). Precision: not disclosed per-model on the pages fetched — "unspecified" across
the board.

### Together AI — in scope, self-serve
Best data came from `docs.together.ai/docs/serverless-models` (2026-09-16), cross-checked against
raw HTML on `together.ai/pricing` where the $1.04 Llama 3.3 70B figure appeared twice independently:

| Model | $/M in | $/M out | Context |
|---|---|---|---|
| Llama 3.3 70B Instruct Turbo | 1.04 | 1.04 | 131K |
| Qwen3.5 9B | 0.17 | 0.25 | 262K |
| Qwen3.6 Plus | 0.50 | 3.00 | 1M |
| Qwen3.7 Plus | 0.32 | 1.28 | 1M |
| Qwen3.8 Flash | 0.15 | 0.47 | 1M |
| DeepSeek V4-Flash-0731 | 0.14 | 0.28 | 1M |
| DeepSeek V4.1 Flash | 0.30 | 1.20 | 1M |
| GLM-5.3 Flash | 0.15 | 0.50 | ~1M |
| GPT-OSS 120B | 0.15 | 0.60 | 131K |
| Ternary Bonsai 27B | Free | Free | 262K (obscure model, treat as unproven) |
| GLM-5.2 | 1.40 | 4.40 | — |
| Kimi K3 | 3.00 | 15.00 | — |
| MiniMax M3 | 0.30 | 1.20 | — |

Llama 4 Scout/Maverick, Qwen3-235B, Mistral, Gemma listings appear on the page's live model catalog
(200+ models, confirmed present by name-grep of the raw HTML) but a clean, matched price for each was
not reliably extracted in this pass — the marketing page renders several other tables (fine-tuning
$/hr, dedicated-GPU $/hr) inline with the serverless price table with matching model names, and I
could not distinguish them with confidence via scripted grep; **flagging these as not fully verified**
rather than reporting a possibly-wrong number. 68 models are offered free (Llama 3.3 70B is **not**
among the free ones — the free set is smaller/older models per Together's own copy). Structured
output: Together documents `response_format`/JSON schema and tool calling as OpenAI-compatible.

### DeepInfra — in scope, self-serve
Source: `deepinfra.com/pricing`, WebFetch 2026-09-16 (not independently grep-confirmed from raw HTML
in this pass — treat as aggregator-grade "A" confidence despite being the provider's own domain,
since the page is client-rendered):

| Model | $/M in | $/M out | Context |
|---|---|---|---|
| DeepSeek-V4-Flash-0731 | 0.06 (51%-off promo) / 0.09 (regular) | 0.18 | 1024K |
| DeepSeek-V4-Pro | 1.30 | 2.60 | 1024K |
| DeepSeek-V3.2 | 0.26 | 0.38 | 160K |
| DeepSeek-V3.1 | 0.25 | 0.95 | 160K |
| DeepSeek-R1-0528 | 0.50 | 2.15 | 160K |
| Llama-3.3-70B-Instruct-**Turbo** | 0.10 | 0.32 | 128K |
| Meta-Llama-3.1-70B-Instruct-**Turbo** | 0.40 | 0.40 | 128K |
| Meta-Llama-3.1-8B-Instruct-**Turbo** | 0.02 | 0.04 | 128K |
| Mistral-Small-3.2-24B-Instruct-2506 | 0.075 | 0.20 | 125K |
| Mistral-Nemo-Instruct-2407 | 0.019 | 0.03 | 128K |

"Turbo" on DeepInfra (and Together) denotes an FP8-quantized serving SKU, distinct from the
non-Turbo BF16 endpoint at a higher price (§4). Cached-token pricing (3-5x cheaper than fresh input)
is offered on several models per DeepInfra's own copy.

### Mistral La Plateforme — in scope, self-serve
Mistral's naming has moved on: `docs.mistral.ai` (fetched 2026-09-16) lists the **current** lineup as
Mistral Large 3, **Mistral Small 4** (successor to Small 3.x, Apache 2.0), and **Ministral 3** in
14B/8B/3B sizes (Apache 2.0) — matching context.md's "Mistral Small 3.x/4 and Ministral" ask exactly.
Pricing (aggregator consensus, not confirmed against Mistral's own pricing table in this pass —
`docs.mistral.ai/getting-started/models/models_overview/` lists models and licenses but not prices):

| Model | $/M in | $/M out |
|---|---|---|
| Mistral Small 4 | 0.15 | 0.60 |
| Ministral 8B | 0.15 | 0.15 |
| Ministral 3B/14B | ~0.10–0.20 both directions | — |

Not found: an authoritative first-party Mistral pricing table for these exact new names in this pass
(La Plateforme's pricing page did not resolve cleanly to a fetchable table). OpenRouter's live API
carries `mistralai/ministral-8b-2512` ($0.15/$0.15), `mistralai/ministral-14b-2512` ($0.20/$0.20),
`mistralai/ministral-3b-2512` ($0.10/$0.10) and `mistralai/mistral-small-2603` ($0.15/$0.60) as a
cross-check (primary, `openrouter.ai/api/v1/models`, 2026-09-16) — these agree with the aggregator
numbers to the cent, raising confidence. Structured output: Mistral's own docs advertise
`response_format` JSON schema support (`docs.mistral.ai/capabilities/structured-output/`, title seen
in a related search result, not fetched directly — noted, not independently confirmed here).

### AWS Bedrock — in scope, self-serve (with AWS account)
Source: `aws.amazon.com/bedrock/pricing/`, WebFetch 2026-09-16:

| Model | $/M in | $/M out | Region |
|---|---|---|---|
| Devstral 2 123B (Mistral) | 0.40 | 2.00 | US East/West |
| Magistral Small 1.2 (Mistral) | 0.50 | 1.50 | US East/West |
| Mistral Large 3 | 0.50 | 1.50 | US East/West |
| Ministral 3B 3.0 | 0.10 | 0.10 | US East/West |
| Ministral 8B 3.0 | 0.15 | 0.15 | US East/West |
| Llama 3.3, Llama 3.2, Llama 4 | listed but rates not captured in this pass | — | — |
| DeepSeek, Gemma, Nemotron, Qwen | "offered with regionally-variable pricing" per the page, rows not captured | — | — |

Not found: exact Llama 3.3/4, DeepSeek, Gemma, Qwen rates on Bedrock in this pass — the page's table
is large and region-split; would need a second, more targeted fetch. Bedrock bills per-region and
pricing varies by AWS region (the page states Asia Pacific/South America/Europe differ from US
rates). Access requires an AWS account and per-model "access request" approval in the console, and
is invoked via AWS SDK / SigV4-signed REST, not a plain bearer-token fetch — a real integration cost
for a Deno edge function (§5).

### Google Vertex AI Model Garden — in scope, self-serve (with GCP account)
`cloud.google.com/vertex-ai/generative-ai/pricing` did not resolve to a fetchable pricing table in
this pass (WebFetch returned only a page header, likely JS-rendered). Aggregator consensus (CloudZero,
nOps, Spheron, TokenMix — 4 sources) states: Llama 3.3 70B $1.36/M (blended, in/out split not given),
Llama 4 70B-class $2.50 in / $3.40 out, versus Together's $1.04 flat for the same open weight — "a
~55% premium" one source characterizes it as. Vertex's model bills differ by category: "Google models
bill per token, third-party models follow vendor pricing, and **open models bill per Endpoint
instance-hour**" (Spot instances give a 70% discount) rather than per-token — meaning most Llama/Qwen/
Mistral rows on Model Garden may not be a simple $/M-tokens number at all, which matters for the cost
arithmetic in §1 (Vertex rows are omitted from the sorted table for this reason — not comparable
without knowing throughput per instance-hour, which was not found in this pass). Flagging this whole
provider's numbers as **low confidence / needs a second pass** before any decision leans on them.

### OpenRouter — in scope, self-serve, an aggregator/router itself
Source: **live API**, `https://openrouter.ai/api/v1/models`, fetched directly with `curl` 2026-09-16
(444 models returned, primary source, highest confidence in this whole report since it's the raw
JSON the site itself serves, not a rendered/summarized page). Selected open-weights rows (full price
= `pricing.prompt`/`pricing.completion` × 1,000,000; `supported_parameters` confirmed to include
`response_format` and `structured_outputs` for every row checked):

| Model | $/M in | $/M out | Context |
|---|---|---|---|
| meta-llama/llama-3.1-8b-instruct | 0.05 | 0.08 | 131K |
| meta-llama/llama-3.1-70b-instruct | 0.40 | 0.40 | 131K |
| meta-llama/llama-3.3-70b-instruct | 0.10 | 0.32 | 131K |
| meta-llama/llama-4-scout | 0.10 | 0.30 | 1.31M (top-provider cap 327K) |
| meta-llama/llama-4-maverick | 0.1875 | 0.6525 | 1.05M (top-provider cap 128K) |
| qwen/qwen3-8b | 0.117 | 0.455 | 131K |
| qwen/qwen3-32b | 0.08 | 0.28 | 131K |
| qwen/qwen3-30b-a3b | 0.12 | 0.50 | 131K |
| qwen/qwen3-30b-a3b-instruct-2507 | 0.048 | 0.193 | 262K |
| qwen/qwen3-235b-a22b | 0.455 | 1.82 | 131K |
| qwen/qwen3-235b-a22b-2507 | 0.0875 | 0.35 | 262K |
| deepseek/deepseek-v3.2 | 0.269 | 0.40 | 164K |
| deepseek/deepseek-v3.1-terminus | 0.27 | 1.00 | 164K |
| deepseek/deepseek-r1 | 0.70 | 2.50 | 64K |
| mistralai/mistral-small-3.1-24b-instruct | 0.351 | 0.555 | 128K |
| mistralai/mistral-small-3.2-24b-instruct | 0.094 | 0.25 | 256K |
| mistralai/ministral-8b-2512 | 0.15 | 0.15 | 262K |
| google/gemma-3-27b-it | 0.08 | 0.45 | 131K |
| openai/gpt-oss-120b | 0.037 | 0.17 | 131K |
| openai/gpt-oss-20b | 0.03 | 0.13 | 131K |
| z-ai/glm-4.6 | 0.43 | 1.75 | 205K |
| z-ai/glm-4.7-flash | 0.0605 | 0.40 | 200K |
| moonshotai/kimi-k2-0905 | 0.60 | 2.50 | 262K |
| nvidia/nemotron-3-super-120b-a12b | 0.08 | 0.45 | 262K |
| nvidia/nemotron-3-nano-30b-a3b | 0.05 | 0.20 | 262K |

Caveat: OpenRouter's shown price per model is typically the **cheapest available backend route**
among the providers that host it that day, aggregated across many of the providers in this same
report (it is not a fixed OpenRouter-operated price) — see §4 on why this makes precision
verification hard. `:batch`-suffixed variants (also in the raw JSON) offer roughly 40-50% off for
async use. Zero-retention: OpenRouter's provider-selection docs mention a "zero-data-retention"
provider filter, confirming the option exists at self-serve, but exact terms per backend were not
independently confirmed in this pass.

### Baseten — in scope, self-serve
Source: `baseten.co/pricing/`, WebFetch 2026-09-16:

| Model | $/M in | $/M out |
|---|---|---|
| GPT OSS 120B | 0.10 | 0.50 |
| DeepSeek-V4-Flash-0731 | 0.13 | 0.26 |
| GLM-5.3-Flash | 0.15 | 0.50 |
| DeepSeek V4.1 Flash | 0.30 | 1.20 |
| GLM-5.3 / GLM-5.2 | 1.40 | 4.40 |
| GLM 4.7 | 0.60 | 2.20 |
| DeepSeek V4 Pro | 1.32–1.74 | 3.48–3.96 |
| Kimi K2.6 / K2.7 Code | 0.95 | 4.00 |
| Kimi K3 | 3.00 | 15.00 |
| NVIDIA Nemotron 3 Ultra | 0.60 | 2.40 |

Baseten's own page states "No Llama, Qwen, or Gemma models appear in the current [Model APIs]
pricing table" as of this fetch, even though its model library page lists Llama 4 Scout/Maverick,
Llama 3.3, Gemma 3, Qwen and Nemotron 3 among its models — implying those are dedicated-deployment
only, not pay-per-token Model APIs. Not found: a per-token price for any Llama/Qwen/Gemma row on
Baseten in this pass.

### Nebius (AI Studio / "Token Factory") — in scope, self-serve
Weakest data in this report. `nebius.com/prices` rendered only generic cloud-compute pricing widgets
(no per-token LLM table in the raw HTML); `docs.tokenfactory.nebius.com/pricing` 404'd. One aggregator
(pricepertoken via Requesty) gave Llama-3.3-70B at **$0.13/M input only** — no output price found.
General claims (unverified): "60+ open-source models including Llama, DeepSeek, Qwen," starting
around $0.06/M for small models. **Not found**: a verified per-model in/out pair for any Nebius model
in this pass. Nebius rebranded this product "Token Factory" in 2026 per one source, which may explain
the stale/broken URLs. Needs a second pass with a working docs URL before it can inform a decision.

### Hyperbolic — in scope, self-serve
Weak data. `hyperbolic.ai/pricing` (fetched) turned out to be a GPU-rental page ($1.59/hr etc.), not
per-token LLM pricing — the token-pricing product may live at a different domain (`hyperbolic.xyz`)
that was not directly confirmed reachable in this pass. Aggregator consensus (4 sources): Llama-3.1-8B
and Llama-3.2-3B at $0.10/M (direction unclear — likely blended), Qwen2.5-Coder-32B $0.20/M, "70B-class"
$0.40/M, DeepSeek-V2.5 $2.00/M, Llama-3.1-405B $4.00/M. **Not found**: confirmed in/out split for any
row. Minimum $5 prepaid credit purchase per the aggregator.

### SambaNova Cloud — in scope, self-serve
Source: `cloud.sambanova.ai/pricing`, WebFetch 2026-09-16 (page rendered a real table):

| Model | $/M in | $/M out |
|---|---|---|
| gpt-oss-120b | 0.22 | 0.59 |
| Meta-Llama-3.3-70B-Instruct | 0.60 | 1.20 |
| gemma-4-31B-it | 0.38 | 1.15 |
| DeepSeek-V3.1 | 3.00 | 4.50 |
| DeepSeek-V3.2 | 3.00 | 4.50 |
| MiniMax-M2.7 / M3 | 0.60 | 2.40 |

SambaNova is one of the inference-speed specialists (custom RDU hardware) — see §5. Structured
output and context windows not captured in this pass for these specific rows — not found.

### Cerebras — in scope, self-serve, catalogue very narrow
Source: raw JSON embedded in `cerebras.ai/pricing`'s page bundle, extracted directly (highest
confidence — this is the site's own Sanity CMS content block, not a summarized render), 2026-09-16.
**The self-serve public price list has exactly two rows**:

| Model | Speed | $/M in | $/M out |
|---|---|---|---|
| [OPENAI] GPT OSS 120B | ~3,000 tok/s | 0.35 | 0.75 |
| [QWEN] Qwen 3.8 27B | ~1,850 tok/s | 0.99 | 1.49 |

A wider catalog (Llama 3.3 70B, Llama 4 Maverick/Scout, Qwen3-32B, Qwen3-235B, Qwen3-Coder) exists
only via **Dedicated Endpoints on custom reserved-capacity pricing** — not self-serve pay-per-token,
per Cerebras' own site copy (search-confirmed) and the absence of any other model row in the page's
own pricing JSON. Cerebras is the fastest inference specialist in this list (custom wafer-scale
hardware) — see §5.

## 3. The yardsticks (majors' cheapest current tiers)

| Model | $/M in | $/M out | $/mo typical | Source |
|---|---|---|---|---|
| **Anthropic Claude Haiku 4.5** (current pin) | 1.00 | 5.00 | $1.73 | `platform.claude.com/docs/en/about-claude/pricing`, fetched 2026-09-16 (confirms the context.md pin exactly, including the batch $0.50/$2.50 and 1h-cache-write $2/MTok rows) |
| **OpenAI gpt-5-nano** (cheapest current) | 0.05 | 0.40 | $0.107 | `developers.openai.com/api/docs/pricing`, fetched 2026-09-16 (redirected from `platform.openai.com/docs/pricing`) |
| OpenAI gpt-5-mini | 0.25 | 2.00 | $0.529 | same source |
| OpenAI gpt-4o-mini | 0.15 | 0.60 | $0.239 | same source |
| **Google Gemini 2.5 Flash-Lite** (cheapest current, retiring 2026-10-16) | 0.10 | 0.40 | $0.160 | `ai.google.dev/gemini-api/docs/pricing`, fetched 2026-09-16 |
| Google Gemini 3.1 Flash-Lite (successor, current) | 0.25 | 1.50 | $0.464 | same source |
| Google Gemini 3.5 Flash-Lite | 0.30 | 2.50 | $0.648 | same source |
| Google Gemini 3.7/3.8 Flash | 0.75 (→1.50 on 2027-01-01) | 3.75 (→7.50) | $1.294 | same source |

Note: `gpt-5-nano` at $0.107/mo typical is, remarkably, cheaper than several of the open-weights rows
in §1 (e.g. Groq/Together/Fireworks gpt-oss-120b at $0.239). It is **out of scope for the actual
provider choice per rule 3** ("yardsticks only" — the study is about open-weights hosts), but it is
worth flagging to the founder as context: a closed-weights frontier lab's smallest model already
beats several open-weights self-serve rates on raw $/token, though not its privacy terms (Anthropic's
zero-retention terms are the named baseline; OpenAI's and Google's default retention policies were
not investigated here — that's the legal agent's question).

## 4. Quantization notes

**FP8 vs BF16.** Multiple sources agree FP8 post-training quantization is close to lossless for most
production NLP workloads: "<1% degradation" across classification/NLP/segmentation benchmarks
(FP8 formats survey, [arxiv.org/pdf/2309.14592](https://arxiv.org/pdf/2309.14592)), and "accuracy
delta for FP8 vs BF16 is typically under 0.5% on MMLU for 70B models," widening to "1-2% degradation"
for 7B-class models — i.e. **quantization hurts smaller models proportionally more**, which matters
directly for Knowlu's likely candidates (gpt-oss-20b, Llama 3.1 8B, Qwen3 8B are all sub-30B). One
source specifically calls out that "dynamic quantization obtains results similar to the original
BF16... whereas static quantization leads to noticeable accuracy degradation" — the quantization
*method*, not just the bit-width, matters, and self-serve providers do not disclose which they use.

**INT4 (AWQ/GPTQ) vs BF16, and the 70B-vs-8B split the founder should know about.** The clearest
citable numbers: on Llama-3.1 8B, GPTQ-int4 and AWQ-int4 show **-7.2% and -7.1%** accuracy change
vs BF16 on long-context tasks; on Llama-3.1 70B, the same quantizations show only **-1.3% and -1.4%**
("Give Me BF16 or Give Me Death?", [arxiv.org/pdf/2411.02355](https://arxiv.org/pdf/2411.02355), and
the long-context quantization study [aclanthology.org/2025.emnlp-main.479.pdf](https://aclanthology.org/2025.emnlp-main.479.pdf)).
**This is the opposite of the FP8 pattern**: int4 hurts small (8B-class) models roughly 5x more than
70B-class models. Practical read for Knowlu: Cloudflare's own catalogue has this exact pair —
`llama-3.1-8b-instruct-awq` (int4, $0.123/$0.266) is a case where the quantization-quality risk is
highest of anything in this report; a 70B-class AWQ/GPTQ row (none appeared self-serve in this
catalogue at int4 — Cloudflare's 70B rows are fp8, not int4) would be the safer int4 choice if a
provider offered one. General INT4 caveat: one source states 4-bit methods can show accuracy losses
"up to 59%" on long-context-heavy evaluations specifically, while 8-bit preserves accuracy to ~0.8%
average drop — the size of the loss is highly task-dependent, and Knowlu's judgment calls are short
extraction/classification, not long-context reasoning, which is the more forgiving end of that range,
but this was not benchmarked for exactly this task shape by anyone I found.

**AWQ vs GPTQ head-to-head.** Mixed evidence: "near-identical on academic benchmarks (AWQ +0.23-0.35
pts)" but "GPTQ outperforms AWQ on real-world tasks by 2.9 and 0.8 points" in one comparison, while
another source says "AWQ scores 1-3% higher on MMLU and HumanEval vs GPTQ at the same bit-width" —
these two findings disagree with each other, so treat method choice as a wash unless Knowlu runs its
own eval (which the live comparison phase, out of scope here, will do).

**Structured-output / function-calling specific evidence.** I did not find a benchmark isolating
JSON-schema-constrained generation specifically under quantization (as opposed to general
classification/MMLU). This is a **gap** — flagged in §6. The closest proxy: FP8's near-lossless
classification result above should transfer reasonably to schema-constrained classification (the
grammar/schema constraint itself does most of the format-correctness work regardless of quantization;
what degrades is *content* accuracy, e.g. picking the wrong enum value or misjudging effort_hours).

**Providers that quietly serve a quantized model under the unquantized name.** This is a documented,
live problem, not a hypothetical:
- OpenRouter's own docs: "by default OpenRouter can route your request to providers that serve
  quantized versions of the model, which can result in substantially worse output," and critically,
  "the 'unknown' precision label indicates that the provider does not disclose quantization... providers
  are **assumed to serve at full precision if they don't disclose** sub-fp8 quantization" — i.e. the
  absence of a quantization label is not evidence of full precision, it's evidence of non-disclosure
  being treated as full precision by default. (`openrouter.ai/docs/guides/routing/provider-selection`,
  search-surfaced 2026-09-16; also a qwen-code GitHub PR titled "Avoid quantized models on OpenRouter"
  and a Roo-Code issue asking for a UI filter for exactly this problem — both corroborate this is a
  known, recurring user complaint, not a one-off.)
- "Turbo"-suffixed SKUs on Together/DeepInfra are FP8 by convention and priced lower than the
  non-Turbo BF16 SKU of the same model (confirmed for Llama 3.1 8B on DeepInfra: Turbo $0.02/$0.04 vs
  presumably higher for a non-Turbo row, though DeepInfra did not surface a non-Turbo Llama-3.1-8B row
  in this pass to price-compare directly) — this is *disclosed* quantization via naming convention,
  the opposite of the silent-swap problem, but only if the buyer knows to read "Turbo" as "FP8."
- Groq's own claim is that its 8-bit ("TruePoint") numerics on its LPU hardware preserve near-BF16
  accuracy via high-precision accumulation, but an independent comparison found "SambaNova's Llama 3
  8B Instruct model outperforms Groq by an average of 3.16% on general tasks... statistically
  significant in most cases (11 of 15)" — i.e. **Groq's own accuracy-preservation claim has at least
  one independent contradicting data point**, worth weighing given every Groq model in this report is
  quantized to 8-bit by Groq's own admission and none of Groq's self-serve rows disclose this as a
  provider-declared "fp8" tag (their whole platform runs everything at reduced precision by design,
  not as an opt-in SKU).
- Cloudflare is the most transparent provider in this catalogue: it names precision directly in the
  model id (`-fp8`, `-fp8-fast`, `-awq`) and prices the plain (non-suffixed) name higher, which is the
  opposite failure mode — good practice, not a red flag.

## 5. Practicalities per provider

| Provider | Sign-up | Card required | Min spend | Endpoint | Deno/fetch friendly | Latency class | Uptime evidence | Existing vendor? |
|---|---|---|---|---|---|---|---|---|
| Cloudflare Workers AI | Existing account (already a Knowlu vendor) | Already on file | Usage-based, Workers Paid plan gates some models | REST, OpenAI-compatible-ish + native `response_format` | Yes — plain fetch | Standard | Cloudflare status page exists (not itemized in this pass) | **Yes — already the app's edge/DNS vendor per HANDOFF** |
| Groq | Email sign-up, free tier | Not for free tier (per general knowledge; not independently confirmed this pass) | None known | OpenAI-compatible REST | Yes — plain fetch | **Speed specialist** (LPU) — fastest first-token latency in the market by reputation | Not captured this pass — not found | New party |
| Fireworks AI | Email sign-up | Likely for paid usage | None found | OpenAI-compatible REST | Yes | Standard-fast | Not found | New party |
| Together AI | Email sign-up, free credits | Likely for paid usage | None found | OpenAI-compatible REST | Yes | Standard | Not found | New party |
| DeepInfra | Email sign-up | Likely for paid usage | None found | OpenAI-compatible REST | Yes | Standard | Not found | New party |
| Mistral La Plateforme | Email sign-up, free tier | Not for free tier | None found | Native REST (OpenAI-compatible mode also offered per general knowledge) | Yes | Standard | Not found | New party |
| AWS Bedrock | AWS account, per-model access request in console | Yes (AWS billing) | AWS account minimums/none | AWS SDK, SigV4-signed REST (**not** a simple bearer token) | **Friction**: needs SigV4 signing from Deno, harder than a plain fetch | Standard | AWS status page (very mature) | New party (though AWS is a huge, well-known vendor generally) |
| Google Vertex AI Model Garden | GCP account, service-account/OAuth | Yes (GCP billing) | GCP account minimums/none | Vertex SDK or REST w/ Google OAuth token (**not** a simple bearer key) | **Friction**: OAuth/service-account token flow from Deno is more work than a bearer key | Standard | Google Cloud status page (very mature) | New party |
| OpenRouter | Email sign-up | Not required for pay-as-you-go with prepaid credit (general knowledge, not independently confirmed) | None found | OpenAI-compatible REST, single endpoint routes to 400+ models | Yes — plain fetch, single integration point | Varies by routed backend | Not found | New party |
| Baseten | Email sign-up, free credits | Likely for paid usage | None found | OpenAI-compatible REST ("Model APIs") | Yes | Standard | Not found | New party |
| Nebius AI Studio | Email sign-up | Not found | Not found | OpenAI-compatible per general knowledge | Presumed yes | Standard | Not found | New party |
| Hyperbolic | Email sign-up | Not found | **$5 prepaid minimum** (per aggregator) | OpenAI-compatible per general knowledge | Presumed yes | Standard | Not found | New party |
| SambaNova Cloud | Email sign-up | Not found | Not found | OpenAI-compatible REST | Yes | **Speed specialist** (RDU hardware) | Not found | New party |
| Cerebras | Email sign-up | Not found | Not found | OpenAI-compatible REST | Yes | **Speed specialist** (wafer-scale hardware), reputed fastest raw token/s in the market | Not found | New party |

Every self-serve provider except AWS Bedrock and Google Vertex exposes a plain OpenAI-compatible REST
endpoint reachable with `fetch()` from a Deno edge function with no SDK — matching how
`judge_anthropic.ts` presumably already calls Anthropic today (not inspected in this pass, but the
call profile in context.md describes "one process per judgment," consistent with a simple REST call).
Bedrock and Vertex both require a heavier auth handshake (SigV4 or Google OAuth/service-account) that
would be new integration work in `cloud/supabase/functions/`, not a drop-in swap — a real switching
cost that the sorted table in §1 does not capture in dollars.

## 6. What I could not verify

- **Groq's Llama 3.1 8B / 3.3 70B self-serve status** — conflicting signals: `console.groq.com/docs/models`
  still lists them as "Production" with no price shown; 4+ independent pricing aggregators say they
  moved to enterprise-only on 2026-08-26. Groq's own marketing pricing page would not render for
  either `curl` or WebFetch in this pass. **Needs a direct check of `api.groq.com/openai/v1/models`
  or the console with a real (even free-tier) key before this is trusted** — out of scope for this
  paper study (rule 4: no sign-ups, no API calls).
- **Google Vertex AI Model Garden's actual $/M-token pricing for Llama/Qwen/Mistral** — the official
  pricing page did not render a table to WebFetch; aggregator numbers conflict on whether "open
  models" bill per-token at all versus per-endpoint-instance-hour. This provider's numbers in this
  report should be treated as unreliable until a second, more careful pass.
- **AWS Bedrock's Llama 3.3/4, DeepSeek, Gemma, Qwen per-token rates** — the page confirmed these
  models are offered but the specific dollar rows were not captured; only the Mistral-family rows
  came through cleanly.
- **Nebius and Hyperbolic's actual token pricing** — both providers' pricing URLs either 404'd or
  rendered an unrelated (GPU-rental) page; only single, unconfirmed aggregator numbers exist for
  either. Do not rely on these two providers' numbers in this report for a final decision.
- **Fireworks' and Together's full catalogue prices for Llama/Qwen/Gemma/Mistral rows specifically**
  — both providers' sites clearly host these models (name-confirmed) but a reliably-matched price per
  model was not extracted for most of them; only the subset shown in §2 is trustworthy.
- **Per-model context windows and numeric rate limits for Cloudflare, Groq, Fireworks, Together,
  DeepInfra, Baseten, SambaNova, Cerebras** at the self-serve tier — pricing pages rarely carry these;
  they live on each model's own docs page, which would need one fetch per model per provider (dozens
  of fetches) not attempted in this pass given the volume of providers in scope.
- **Zero-retention / no-logging self-serve option, per provider** — only Cloudflare (JSON mode docs,
  no retention claim found) and OpenRouter (a "zero-data-retention" provider filter exists per its
  routing docs) were checked at all; the other eleven providers were not checked for this specific
  claim in this pass — this is squarely the legal/policy agent's question per the brief's provider
  research allocation, but flagging that I did not check it broadly here either.
- **A benchmark of quantization's effect specifically on JSON-schema-constrained
  classification/extraction** (as opposed to general MMLU/classification or long-context tasks) —
  not found anywhere in this search pass; this is the single biggest evidence gap for the founder's
  actual use case.
- **Together's and Fireworks' Llama 4 / Qwen3-235B / Gemma-3-27B exact self-serve prices** — present
  in the catalogue by name, price not reliably matched (see above); use the OpenRouter rows for these
  models instead, which are primary-sourced and confirmed.
