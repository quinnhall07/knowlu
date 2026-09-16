# Host check — Cloudflare Workers AI, Groq, OpenRouter for the two picks (2026-09-16)

Paper study only: no sign-ups, no API keys, no API calls, no spend. Every fact below carries its
URL and read date (all reads 2026-09-16 unless noted). "Not found" where I could not verify from a
primary source; aggregator-only facts are marked **[A]** and flagged for independent confirmation.

Picks: **IBM Granite 4.2 8B** (instruct, 2026-08-25) and **Qwen3.5-35B-A3B** (instruct, 2026-02).
Substitutes checked where a pick is absent: any Granite 4.x, any Qwen3.5/3.6/3.8 with ~3B active
params, `qwen3-30b-a3b`, and for email GLM-5.3-Flash / DeepSeek V4 Flash.

Cost formula (from `docs/reports/2026-09-16-inference-scoping-provider-offers.md` §0): typical
$/month = 1.0725 × price_in + 0.1305 × price_out; heavy = 4.020 × price_in + 0.483 × price_out
(prices per million tokens). Founder's ceiling: **≤ $0.25/student/month at the typical persona**.

---

## Cloudflare Workers AI

Source for the catalogue: `https://developers.cloudflare.com/workers-ai/models/`, model pages under
`https://developers.cloudflare.com/workers-ai/models/<slug>/`, and the pricing page (all fetched
2026-09-16). **Granite 4.2 8B is not on Cloudflare.** The catalogue's newest Granite is
**Granite 4.0 H-Micro**, a much smaller model than the 8B pick — not a size-comparable substitute,
but the nearest Granite Cloudflare has. **Qwen3.5-35B-A3B is not on Cloudflare either**; the
catalogue's closest match is the named substitute `qwen3-30b-a3b-fp8` (30B/3B active, labeled FP8).
GLM-5.3-Flash and DeepSeek V4 Flash (0731) are both present, gated behind the Workers Paid plan.

| Model id | Precision | Context | $/M in | $/M out | $/mo typical | $/mo heavy | JSON schema | Thinking off | ZDR |
|---|---|---|---|---|---|---|---|---|---|
| `@cf/ibm-granite/granite-4.0-h-micro` (substitute — no 4.2 on this host) | unspecified | 131,072 | 0.017 | 0.112 | $0.033 | $0.122 | Not on the documented JSON Mode list (below) | No thinking mode mentioned on the model page — not found | Not found on model page |
| `@cf/qwen/qwen3-30b-a3b-fp8` (substitute, exact match to the named `qwen3-30b-a3b`) | **fp8 (labeled)** | 32,768 | 0.051 | 0.335 | $0.098 | $0.367 | Not on the documented JSON Mode list | Model page lists "Reasoning" as a capability but documents no `enable_thinking`/`reasoning_effort` parameter — not found | Not found on model page |
| `@cf/z-ai/glm-5.3-flash` (email substitute) | unspecified | 1,310,720 | 0.15 | 0.50 | $0.226 | $0.844 | Not on the documented JSON Mode list | `reasoning_effort` param with low/medium/high — **no documented "off"/"none" value**, so thinking can be *reduced*, not confirmed *disabled* | Not found on model page; gated behind Workers Paid plan |
| `@cf/deepseek-ai/deepseek-v4-flash-0731` (email substitute) | unspecified | 1,310,720 | 0.44 | 1.32 | $0.644 | $2.406 | Not on the documented JSON Mode list | "Reasoning" listed as a capability; no toggle documented — not found | Not found on model page; gated behind Workers Paid plan |

**JSON schema, the actual gate.** Cloudflare's JSON Mode docs
(`https://developers.cloudflare.com/workers-ai/features/json-mode/`, read 2026-09-16) name exactly
six supported models: `@cf/meta/llama-3.3-70b-instruct-fp8-fast`, `@cf/meta/llama-3-8b-instruct`,
`@cf/meta/llama-3.1-8b-instruct`, `@hf/nousresearch/hermes-2-pro-mistral-7b`,
`@hf/thebloke/deepseek-coder-6.7b-instruct-awq`, `@cf/deepseek-ai/deepseek-r1-distill-qwen-32b`.
**None of the four rows above are on that list.** The page adds: "Workers AI can't guarantee that the
model responds according to the requested JSON Schema" and the feature "currently doesn't support
streaming." Every model page above does show a generic `response_format` object in its API schema,
so a request with `response_format: {type: "json_schema", ...}` will not be rejected outright, but
Cloudflare does not document or warrant schema adherence for any of these four models. Treat JSON
schema support here as **undocumented, not confirmed absent** — the honest label per the study's
own rule is "unspecified."

**Zero data retention.** Cloudflare's data-usage page
(`https://developers.cloudflare.com/workers-ai/platform/data-usage/`, read 2026-09-16) states: "Cloudflare
does not use your Customer Content to (1) train any AI models made available on Workers AI or (2)
improve any Cloudflare or third-party services, and would not do so unless we received your explicit
consent." It does not state a retention *duration* for the raw inference request/response outside of
customer-controlled storage (R2/KV/DO/Vectorize) — **not found**. Separately, **AI Gateway logging is
ON by default**, not off: `https://developers.cloudflare.com/ai-gateway/observability/logging/`
(read 2026-09-16) states "Logs, which include metrics as well as request and response data, are
enabled by default for each gateway," with a per-gateway settings toggle and a `cf-aig-collect-log`
header to override per-request. This only applies if Knowlu routes through AI Gateway; direct Workers
AI API calls are not addressed by that page — not found whether they log by default. No page
confirms a retention *duration* for either path (contrast Groq's explicit "up to 30 days" — see
below).

---

## Groq

Source: `https://console.groq.com/docs/models` (fetched 2026-09-16, primary). **Neither pick, nor
any named substitute, is served by Groq.** The current self-serve catalogue is:
`llama-3.1-8b-instant`, `llama-3.3-70b-versatile`, `openai/gpt-oss-20b`, `openai/gpt-oss-120b`,
`openai/gpt-oss-safeguard-20b`, `qwen/qwen3.8-27b` (27B **dense**, not ~3B active — does not match
the "~3B active" substitute rule), `minimaxai/minimax-m2.7`, `groq/compound`, `groq/compound-mini`,
plus audio/guard models. No IBM Granite model, no GLM model, no DeepSeek model appears anywhere in
this listing. `qwen/qwen3.8-27b` is the only line item in the same model family as either pick, and
it fails the substitute's active-parameter criterion, so no row below is a genuine match — it is
included as the closest thing Groq has, clearly marked.

**Conflicting signal on the Llama rows.** Four-plus pricing aggregators (CloudZero, eesel,
Layer3, Markaicode) report Llama 3.1 8B and Llama 3.3 70B moved to "Enterprise, contact sales" on
2026-08-26 with no published self-serve rate **[A]**, while the primary docs page still lists both as
"Production Models" with context/output-token figures and no price shown either way. I could not
resolve this conflict from a primary source; it does not affect the two picks since neither Llama
model is a candidate substitute.

| Model id | Precision | Context | $/M in | $/M out | $/mo typical | $/mo heavy | JSON schema | Thinking off | ZDR |
|---|---|---|---|---|---|---|---|---|---|
| `qwen/qwen3.8-27b` (closest Groq has — **not** a valid ~3B-active substitute; dense 27B) | unspecified ("Groq's own 8-bit LPU numerics" per provider-offers report, not confirmed for this specific model) | 131,042 | not found (no price shown on docs/models; not in Groq's own pricing table found in this pass) | not found | not computable | not computable | Yes — `strict: true` in Groq's structured-outputs docs | `reasoning_effort` supports `"none"` (and `"default"`) — **can be fully disabled** | Available self-serve (below) |

Granite, Qwen ~3B-active, `qwen3-30b-a3b`, GLM-5.3-Flash and DeepSeek V4 Flash: **not served by
Groq — no row.**

**Structured output.** `https://console.groq.com/docs/structured-outputs` (read 2026-09-16) lists
`strict: true` json_schema support for `openai/gpt-oss-20b`, `openai/gpt-oss-120b`, and
`qwen/qwen3.8-27b`; best-effort (`strict: false`) additionally covers
`openai/gpt-oss-safeguard-20b`. This is Groq's only structured-output-capable model in the
Qwen/Granite/GLM/DeepSeek families, and it is not a valid substitute per the study's own rule.

**Thinking off.** `https://console.groq.com/docs/reasoning` (read 2026-09-16): `reasoning_effort`
set to `"none"` fully disables reasoning tokens on `qwen/qwen3.8-27b` and `qwen/qwen3.6-27b`
(both support `"none"`/`"default"`); `gpt-oss-20b`/`120b` support `"low"/"medium"/"high"` (no
"none" — reasoning is mandatory, only its budget shrinks) and use a separate `include_reasoning`
flag rather than `reasoning_format`.

**Zero data retention.** `https://console.groq.com/docs/your-data` (read 2026-09-16), quoted
verbatim: "By default, Groq does not retain customer data for inference requests." Retention only
happens for features that need persistence (e.g. batch jobs) or "to troubleshoot system failures or
investigate abuse," in which case logs are kept "up to 30 days unless law requires longer." **"All
customers may enable Zero Data Retention (ZDR) in Data Controls settings"** — this is explicitly
self-serve, not enterprise-gated: "Organization administrators can decide to enable ZDR globally or
on a per-feature basis at any time" from the Data Controls page in console settings. The page does
not state whether human review of the abuse/troubleshooting logs occurs — not found. No free-vs-paid
distinction in data handling is stated.

**Practicalities.** Free tier: no credit card, every model available, but per
`https://console.groq.com/docs/rate-limits` (read 2026-09-16) the specific models above are capped
at 30 RPM / 1K RPD / 8K TPM / 200K TPD on Free. Developer (paid) tier: aggregator consensus (5+
sources) **[A]** reports a credit card required, **zero minimum spend**, no charge until usage
reaches ~$0.50, ~10× the free-tier limits, and a 25% token-cost discount; most production models run
1,000 RPM / 250,000–300,000 TPM on Developer — the primary rate-limits page confirms the *structure*
("higher limits are available... for the Developer plan") but did not render the actual Developer
numbers in this fetch — treat the specific Developer-tier figures as **[A]**, not independently
confirmed on Groq's own page in this pass. US availability: no region restriction found; Groq's docs
present it as globally available. OpenAI-compatible endpoint: `https://api.groq.com/openai/v1`,
usable with plain `fetch`.

---

## OpenRouter

Both picks are served under their exact names. Source: OpenRouter's public models API
(`https://openrouter.ai/api/v1/models/<id>/endpoints`, and the ZDR endpoint list
`https://openrouter.ai/api/v1/endpoints/zdr`), fetched and parsed as raw JSON 2026-09-16 — primary,
not aggregator.

| Model id | Endpoint (provider) | Precision | Context | $/M in | $/M out | $/mo typical | $/mo heavy | JSON schema | Thinking off | ZDR |
|---|---|---|---|---|---|---|---|---|---|
| `ibm-granite/granite-4.2-8b` | DeepInfra | bf16 (labeled) | 131,072 | 0.06 | 0.25 | $0.097 | $0.362 | `response_format` param present; `structured_outputs` **not** in its supported-parameters list | `reasoning`/`reasoning_effort` params listed | **Yes** |
| `ibm-granite/granite-4.2-8b` | CoreWeave | bf16 (labeled) | 131,072 | 0.10 | 0.15 | $0.127 | $0.474 | `structured_outputs` **is** in its supported-parameters list — the stronger guarantee | `reasoning`/`reasoning_effort` params listed | **Yes** |
| `qwen/qwen3.5-35b-a3b` | Darkbloom (cheapest, OpenRouter's displayed default price) | **fp4** | 262,144 | 0.08 | 0.75 | $0.184 | $0.684 | `structured_outputs` listed | `reasoning`/`include_reasoning` listed; **no `reasoning_effort`** in its param list | No |
| `qwen/qwen3.5-35b-a3b` | DeepInfra | **fp8** | 262,144 | 0.14 | 1.00 | $0.281 | $1.046 | `structured_outputs` listed | `reasoning`/`include_reasoning` listed; **no `reasoning_effort`** | **Yes** |
| `qwen/qwen3.5-35b-a3b` | Parasail | **fp8** | 262,144 | 0.15 | 1.00 | $0.291 | $1.086 | `structured_outputs` listed | `reasoning`/`include_reasoning` listed; **no `reasoning_effort`** | **Yes** |
| `qwen/qwen3.5-35b-a3b` | Alibaba (first-party) | unknown | 262,144 | 0.1625 | 1.30 | $0.344 | $1.281 | `structured_outputs` listed | `reasoning`/`include_reasoning` listed; **no `reasoning_effort`** | No |
| `qwen/qwen3.5-35b-a3b` | AtlasCloud | **fp8** | 262,144 | 0.225 | 1.80 | $0.476 | $1.774 | `structured_outputs` listed | same | No |
| `qwen/qwen3.5-35b-a3b` | SiliconFlow | **fp8** | 262,144 | 0.24 | 1.80 | $0.492 | $1.834 | `structured_outputs` listed | same | **Yes** |
| `qwen/qwen3.5-35b-a3b` | Venice | unknown | 256,000 | 0.3125 | 1.25 | $0.498 | $1.860 | `structured_outputs` listed | same | **Yes** |
| `qwen/qwen3-30b-a3b-instruct-2507` (substitute, exact name match) — cheapest ZDR row | SiliconFlow | **fp8** | 262,144 | 0.09 | 0.30 | $0.136 | $0.507 | `structured_outputs` listed | `reasoning`/`reasoning_effort` present per endpoint (not individually re-verified beyond the aggregate search) | **Yes** |
| `z-ai/glm-5.3-flash` (email substitute) — cheapest ZDR row | Inceptron | **fp8** | 1,048,576 | 0.15 | 0.50 | $0.226 | $0.844 | `structured_outputs` listed | `reasoning_effort` param present | **Yes** (22 of its ZDR-listed endpoints total) |
| `deepseek/deepseek-v4-flash-0731` (email substitute, exact match) — cheapest ZDR row | DeepInfra | **fp8** | 1,048,576 | 0.06 | 0.18 | $0.088 | $0.328 | `structured_outputs` listed on most endpoints | `reasoning_effort` param present | **Yes** (23 ZDR endpoints total) |

**JSON schema, precisely.** OpenRouter's own docs
(`https://openrouter.ai/docs/features/structured-outputs`, read 2026-09-16): "Support is determined
per endpoint, not just per model... To see which providers support structured outputs for a
specific model, check the `structured_outputs` parameter in the Providers section of the model's
page," and enforcement is `require_parameters: true` in `provider` preferences plus
`response_format: {type: "json_schema", ...}`. Per the raw endpoint JSON: **Granite's CoreWeave
endpoint** and **every Qwen3.5-35B-A3B / Qwen3-30B-A3B-Instruct-2507 / GLM-5.3-Flash / DeepSeek-V4-Flash
endpoint checked** declares `structured_outputs` in `supported_parameters`; Granite's cheaper
DeepInfra endpoint declares only the weaker `response_format`, not `structured_outputs` — so for
Granite specifically, pin CoreWeave (or use `require_parameters: true` to force the stronger
endpoint) if a hard schema guarantee matters.

**Thinking off.** OpenRouter's `reasoning` object controls this generically:
`reasoning.enabled: false` "stops the model from generating reasoning tokens altogether" (actually
disables, not just hides); `reasoning.exclude: true` lets the model reason internally but omits it
from the response — "the tokens are still billed and count against `max_tokens`" (i.e. it does
**not** save cost or avoid truncation risk against Knowlu's tight `max_tokens` caps — `reasoning.enabled:
false` is the one that matters here). Caveat, same doc: some upstream endpoints mark
`"mandatory": true` and reject `effort: "none"` / `enabled: false` outright — "the model rejects it."
**Qwen3.5-35B-A3B's endpoints (Darkbloom, DeepInfra, Parasail, Alibaba, AtlasCloud, SiliconFlow,
Venice) list `reasoning`/`include_reasoning` as supported parameters but none list `reasoning_effort`**
— OpenRouter's generic `reasoning.enabled: false` is therefore the only documented lever, and because
several of these are vLLM-backed, it likely bottoms out in the same `chat_template_kwargs.enable_thinking`
path that the capability report (§6) already flagged as buggy on vLLM for Qwen3.5 — **not independently
re-verified per-provider in this pass; this is the single highest-risk unresolved item for the
Qwen3.5-35B-A3B pin on this host.** Granite's two endpoints both list `reasoning_effort` explicitly,
which is a more direct, documented switch.

**Zero data retention and provider pinning.** OpenRouter's ZDR guide
(`https://openrouter.ai/docs/guides/features/zdr`, read 2026-09-16): "OpenRouter has privacy
settings that, when enabled, only allow you to route to endpoints that have a Zero Data Retention
policy. You can enforce ZDR globally, per model group, per guardrail, or per request." Per-request:
`provider: {zdr: true}` in the request body — self-serve, no sales conversation required, confirmed
by the same doc plus the publicly fetchable endpoint list. **Provider pinning to exactly one named
upstream is supported and documented**: `provider: {order: ["deepinfra"], allow_fallbacks: false}`
restricts the request to that provider only ("This configuration attempts only those two providers
without falling back to others" — generalizes to a single-entry array). Combining `zdr: true` with
`order: [<one ZDR provider>], allow_fallbacks: false` gives a request that is both restricted to one
named upstream and restricted to ZDR endpoints. "OpenRouter itself has a ZDR policy; your prompts are
not retained unless you specifically opt in to prompt logging." The full ZDR endpoint list is
machine-readable at `https://openrouter.ai/api/v1/endpoints/zdr` (871 endpoints total as of this
read) and is stated to auto-update on provider policy changes.

**Practicalities.** Free-tier/self-serve rate limit: 20 RPM on free-model variants; 50 free-model
requests/day under $10 lifetime credit purchased, rising to 1,000/day once $10 has ever been spent
(a one-time unlock, not a balance requirement) **[A, several aggregators agreeing]**. No credit card
needed to sign up or to use paid (non-free) models pay-as-you-go; a card (or crypto) is needed only
to add credit. No stated minimum spend beyond whatever credit you choose to add. US availability: no
region restriction found. OpenAI-compatible endpoint: `https://openrouter.ai/api/v1`, usable with
plain `fetch`.

---

## Rate-limit / practicalities summary

| | Cloudflare Workers AI | Groq | OpenRouter |
|---|---|---|---|
| Free/self-serve tier | 10,000 Neurons/day, no card **[A, several sources]**; per-model RPM (text gen default 300 RPM; "Frontier models" 20 RPM standard / 50 RPM with prepaid credits) | 30 RPM / 8K TPM / 1K RPD per relevant model, no card, every model available | 20 RPM on free models; 50–1,000 free-model req/day depending on lifetime credit purchased |
| Paid tier | $0.011/1,000 Neurons above the free allocation; card needed only to exceed free tier | Card required, **zero minimum spend**, no charge until ~$0.50 usage, ~10× free RPM, 25% token discount **[A]** | Pay-as-you-go by credit; no stated minimum, no subscription |
| Card required | Only above free tier | Only to unlock Developer tier | Only to add credit |
| US availability | Yes, no restriction found | Yes, no restriction found | Yes, no restriction found |
| OpenAI-compatible base URL | `https://api.cloudflare.com/client/v4/accounts/{account_id}/ai/v1` | `https://api.groq.com/openai/v1` | `https://openrouter.ai/api/v1` |
| Usable with plain `fetch` from Deno | Yes (standard REST/JSON over HTTPS) | Yes | Yes |

---

## OpenRouter routing detail (per requested item 5)

- **Granite 4.2 8B** (`ibm-granite/granite-4.2-8b`): 2 upstreams total — **DeepInfra** (bf16,
  $0.06/$0.25, `response_format` only) and **CoreWeave** (bf16, $0.10/$0.15, `structured_outputs`).
  Both are on the current ZDR list. A 2-provider roster with both members ZDR is about as stable a
  privacy story as this study found — naming "Granite 4.2 8B is served zero-retention via DeepInfra
  or CoreWeave" on a privacy page is defensible today, with the caveat that a 2-provider roster can
  shrink to 1 or 0 without notice; re-check before publishing that sentence.
- **Qwen3.5-35B-A3B** (`qwen/qwen3.5-35b-a3b`): 7 upstreams — Darkbloom (fp4, not ZDR), DeepInfra
  (fp8, ZDR), Parasail (fp8, ZDR), Alibaba (first-party, precision "unknown", not ZDR), AtlasCloud
  (fp8, not ZDR), SiliconFlow (fp8, ZDR), Venice (precision "unknown", ZDR). **4 of 7 are ZDR**
  (DeepInfra, Parasail, SiliconFlow, Venice); the cheapest overall (Darkbloom) and the first-party
  Alibaba endpoint are both non-ZDR. A 7-provider roster with a 4/7 ZDR split is a *less* stable
  thing to name on a privacy page than Granite's — pin `provider.zdr: true` at minimum, and consider
  pinning to one named ZDR provider (e.g. DeepInfra, cheapest ZDR option at $0.14/$1.00,
  $0.281/mo typical) rather than naming "OpenRouter" generically, since the routed default (Darkbloom)
  is not zero-retention.
- **Qwen3-30B-A3B-Instruct-2507** (substitute): DekaLLM, SiliconFlow, Nebius all ZDR (3 of 3 checked
  ZDR-listed rows); cheapest is SiliconFlow at $0.09/$0.30 (typical $0.136/mo).
- **GLM-5.3-Flash** (email substitute): a much larger roster — 22 ZDR-listed endpoints alone
  (Inceptron, Crusoe, CoreWeave, Sail Research, NextBit, Fireworks, Relace, Phala, SiliconFlow,
  Morph, DigitalOcean, Wafer, Together, Reka, Parasail, Modal, BaseTen, Venice, Io Net, Novita,
  DeepInfra, Z.AI — precisions mostly fp8/fp4, a few nvfp4/unknown). This is too large and volatile a
  roster to name individual providers on a privacy page with confidence; naming "OpenRouter, ZDR
  filter enforced" without listing specific upstreams is the honest claim here.
- **DeepSeek V4 Flash 0731** (email substitute): 23 ZDR-listed endpoints, similarly large and
  volatile — same conclusion as GLM-5.3-Flash.

---

## Verdict per host: can it serve both picks at fp8 with zero retention, today?

**Cloudflare Workers AI: No.** Neither pick is on this host at all — Granite 4.2 8B isn't offered
(only the smaller Granite 4.0 H-Micro is), and Qwen3.5-35B-A3B isn't offered (only the substitute
`qwen3-30b-a3b-fp8`, which is FP8 as labeled). Cloudflare's substitute for Qwen is genuinely FP8 and
cheap ($0.098/mo typical), but neither it nor the Granite substitute is on Cloudflare's own JSON
Mode supported-model list, so schema adherence is undocumented for both, and no page found in this
study states a retention duration for direct Workers AI inference (only that Cloudflare doesn't
train on it) — "zero retention" is not demonstrated, only "no training" is. AI Gateway, if used,
logs by default and must be turned off. Cloudflare cannot be described today as serving either pick,
at any precision, with zero retention confirmed.

**Groq: No, more starkly.** Groq serves neither pick nor any of the five named substitutes (Granite
4.x, a ~3B-active Qwen, `qwen3-30b-a3b`, GLM-5.3-Flash, DeepSeek V4 Flash) — its Qwen/Granite-family
catalogue is a single 27B **dense** model (`qwen/qwen3.8-27b`) that fails the substitute's
active-parameter test. Groq's self-serve Zero Data Retention toggle is real, well-documented, and
genuinely self-serve (a strong positive for the *kind* of guarantee Knowlu needs), and its precision,
thinking-off (`reasoning_effort: "none"`), and structured-output (`strict: true`) stories are all
solid — but there is nothing to point that guarantee at for either pick. Groq is out of scope for
this decision as things stand today, not because of privacy or capability but because of catalogue
absence.

**OpenRouter: Yes, with caveats.** Both picks are served under their literal names, at genuinely
declared precisions (bf16 for Granite via DeepInfra/CoreWeave, fp8 for Qwen3.5-35B-A3B via
DeepInfra/Parasail/SiliconFlow/Venice), and the ZDR filter plus `provider.order` +
`allow_fallbacks: false` together let a request be pinned to exactly one named, ZDR-flagged upstream
— which is the strongest, most specific privacy claim any of the three hosts can support today.
Granite's roster is small and entirely ZDR (2/2), a stable claim. Qwen3.5-35B-A3B's roster is larger
and only 4/7 ZDR, with the cheapest default (Darkbloom, fp4) *not* ZDR — so "OpenRouter serves both
picks at fp8 with zero retention" is true **only if the request explicitly pins to a ZDR fp8 endpoint
(e.g. DeepInfra)**; taken at OpenRouter's routing defaults, it is not automatically true. The one
open risk is `reasoning_effort` being undocumented on every Qwen3.5-35B-A3B endpoint (only the
generic, possibly-buggy-on-vLLM `reasoning.enabled: false` is available) — this is a testable risk,
not a blocking one, and it is the item most worth checking first on the founder's own vault.

---

## What I could not verify

- Cloudflare: no per-model context-window figures on the pricing page itself (had to pull them from
  each model's own page instead — done here, but the pricing page alone is insufficient); no stated
  retention *duration* for direct Workers AI API calls (only "no training"); numeric RPM/TPM limits
  for the free tier beyond the few examples the fetch surfaced; whether direct (non-Gateway) Workers
  AI calls are logged by default.
- Groq: current self-serve price for `llama-3.1-8b-instant` / `llama-3.3-70b-versatile` — docs page
  lists them "Production" with no price, 4+ aggregators say Enterprise-only since 2026-08-26,
  unresolved conflict (moot for this study's picks). Developer-tier numeric rate limits are
  aggregator-sourced, not confirmed on Groq's own rendered page in this pass. Whether Groq performs
  human review of its troubleshooting/abuse logs. `qwen/qwen3.8-27b`'s own $/M price (not shown on
  `docs/models`; would need the billing/pricing page, not fetched in this pass) and its declared
  precision beyond "Groq's own 8-bit LPU numerics" claimed generically in the provider-offers report,
  not confirmed on this model's own page.
- OpenRouter: precision for the Alibaba and Venice Qwen3.5-35B-A3B endpoints is "unknown" per
  OpenRouter's own data (not a gap in my research — OpenRouter itself does not know/publish it).
  Whether `reasoning.enabled: false` actually suppresses reasoning tokens specifically on each of
  Qwen3.5-35B-A3B's seven endpoints (vs. being silently ignored per the known vLLM
  `enable_thinking` bug) — not independently testable under this study's no-API-calls rule; flagged
  as the top item for the founder's live vault comparison. Whether OpenRouter's `structured_outputs`
  flag equates to hard JSON-schema enforcement or best-effort per provider — the docs describe the
  mechanism but not per-provider reliability data.
