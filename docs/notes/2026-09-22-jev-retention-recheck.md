# Jev's retention terms — a re-check, 2026-09-22

**Status: a dated record of what the legal and provider pages say today. Nothing decided.** Written
on Quinn's instruction to re-check the one question
`docs/reports/2026-09-22-jev-system-one-assessment.md` left open: whether TypeSafe offers a
**self-serve** retention control. Not legal advice. Every page below was read on **2026-09-22**.

## The answer, in one line

**TypeSafe itself still has no self-serve retention control — but the answer to the question that
actually governs us has changed: OpenRouter now marks `typesafe/jev-1.13` zero-data-retention, and
that is the production path R8 as amended already uses.**

## 1. TypeSafe's own properties — unchanged

Zero data retention is still enterprise-only and obtained by email. `docs.typesafe.ai/legal`: *"We
also offer zero data retention (ZDR) for enterprise customers. Contact privacy@typesafe.ai to learn
more."* No toggle, no dashboard setting in any document, no API field.

The API reference is the cleanest evidence. `docs.typesafe.ai/api` documents exactly three
request-body fields — `state`, `model`, `questions` — and two headers, `Authorization` and
`Content-Type`. A keyword scan of the raw page returns **zero** hits for retention, ZDR, store,
logging or privacy. There is no per-request control to find.

The three policy documents, with their own stated dates:

| document | last updated | the operative sentence |
|---|---|---|
| Privacy Policy | **2025-11-19** | *"We retain personal data about you for as long as reasonably necessary to provide you with the Services, or otherwise in support of our business or commercial purposes."* And, separately: *"We will not train or fine tune any artificial intelligence or machine learning models on your prompts or other Input."* |
| Data Processing Agreement | **2026-04-24** | *"Customer Personal Data will be retained for as long as necessary taking into account the purpose of the Processing…"* — and **zero** occurrences of "zero data retention" or "ZDR" anywhere in it. |
| Master Customer Agreement | **2026-09-19** (three days before this check) | §10.3: *"TypeSafe will be under no obligation to store or retain Customer Data and may delete Customer Data at any time in its sole discretion."* §4.1 grants TypeSafe, *"in perpetuity"*, a licence over Customer Data to derive Telemetry and to monitor fraud and abuse, excluding model training without consent. §4.3: *"TypeSafe may Process Telemetry without restriction."* "Retention", "ZDR" and "enterprise" appear **zero** times. |

So the newest first-party document, three days old, adds no retention control and its default
posture for a self-serve account is the opposite of zero retention: a perpetual processing licence
for telemetry and abuse monitoring, with no period attached.

`docs.typesafe.ai` has **no changelog, security, data-controls, privacy or enterprise page** —
each returns 404 — so there is no dated evidence of a revision in either direction on `/legal` or
`/models`, neither of which carries a "last updated" line at all.

## 2. What changed — the gateways

Three things shipped in the week after launch, all third-party:

- **OpenRouter marks it zero-retention.** `GET https://openrouter.ai/api/v1/endpoints/zdr` returns
  the endpoint `TypeSafe | typesafe/jev-1.13-20260917`, and both `openrouter.ai/typesafe` and
  `openrouter.ai/provider/typesafe` embed `"dataPolicy":{"training":false,"trainingOpenRouter":false,"retainsPrompts":false,…}`.
  OpenRouter's enforcement is genuinely self-serve and per request: *"You can enforce Zero Data
  Retention (ZDR) on a per-request basis using the `zdr` parameter, ensuring your request only routes
  to endpoints that do not retain prompts."*
- **Vercel's AI Gateway lists TypeSafe AI as ZDR** with a quoted commitment: *"Except as necessary
  to comply with its legal obligations, TypeSafe shall not retain (a) prompts that are Customer Data
  for any longer than is necessary to generate Output for Customer and (b) Output for any longer
  than necessary…"*. Its changelog of **2026-09-16** adds Jev with ZDR *"enabled per request"* via
  `providerOptions: { gateway: { zeroDataRetention: true } }`.
- **Netlify's AI Gateway** added Jev on **2026-09-17**, with no retention language at all.

**And one source says the opposite.** Opper, verified **2026-09-18**: *"Zero data retention posture
is not established for this route."*

## 3. Reconciling the conflict

Vercel states the mechanism plainly, and it explains all four sources at once: *"AI Gateway has
agreements in place to offer ZDR with specific providers. **A provider's default policy may not
match with the status that AI Gateway has in place due to these agreements.**"*

So zero retention on Jev exists **as a per-counterparty contract, not as a product feature**. Each
gateway reports whether it has one. TypeSafe's published self-serve posture (the MCA of three days
ago) is unchanged and unrelated. Opper reporting no established posture is consistent with Opper not
having such an agreement.

Two cautions this places on the OpenRouter flag, and both matter:

1. **OpenRouter disclaims its own accuracy**: *"This is not a definitive source of third party data
   policies, but represents our best knowledge."*
2. **The commitment, if it exists, runs between OpenRouter and TypeSafe** — not between Knowlu and
   TypeSafe. Our contractual chain would be Knowlu → OpenRouter → TypeSafe, and we would be relying
   on a term we have not seen. That is the same reliance the current pins already place on CoreWeave
   and DeepInfra, so it is not a new *kind* of risk; it is the same one, on a company whose own
   published default is weaker.

## 4. What this changes for the product

The assessment of 2026-09-22 concluded that the email and task kinds fail ruling 1's privacy floor
because *"its zero-retention tier is enterprise-only and sales-gated"*. **That reasoning is now
incomplete.** It is true of TypeSafe direct. It is not true of the path we would actually use.

R8 as amended (2026-09-16) makes **OpenRouter the production path**, pinned per request to one named
upstream carrying `order` of one, `allow_fallbacks: false`, `zdr: true` and `require_parameters:
true`, with `assertPinnedRoute` (`cloud/supabase/functions/_shared/judge_openrouter.ts:21`) refusing
to send anything that does not. A Jev row would take exactly that shape, and the `zdr: true` term it
asserts is the one OpenRouter now reports for this endpoint. So:

- **The transport-level retention objection to the task and ICS-origin event kinds falls away** —
  `retainsPrompts: false` is stronger than ruling 1's loose class, which only asked for ≤30 days for
  abuse monitoring.
- **The strict class is arguable but not settled.** Ruling 1 asks Gmail-derived text (and, on the
  legal report's reading, Google-Calendar-derived events) for *"retention as short as offered, which
  self-serve means zero"*. OpenRouter's per-request `zdr` parameter is self-serve and is zero. What
  is unverified is whether the underlying TypeSafe commitment exists in the form OpenRouter
  reports — and the two cautions in section 3 are why this should not be treated as closed.
- **Every other objection in the assessment stands untouched.** Jev emits no text, so the email
  kind's `title` and `due` and the task kind's `importance_reason` remain impossible; the weights
  are proprietary, so R8's open-weights wording still needs amending; the achievable saving is still
  1.8 cents a student a month. **Retention was never the only blocker, and removing it does not
  change the verdict** — it removes one of several reasons, and the remaining ones are sufficient.

The practical effect is narrower than it first looks: it makes the **event-kind experiment**
(`docs/notes/2026-09-22-jev-event-kind-experiment-design.md`) legally reachable for ICS-origin
events sooner than that note assumed, and it leaves the origin problem — `models` pins by kind, not
by origin — as the binding constraint for the Calendar half.

## 5. Not verified, and why

- **`trust.typesafe.ai` and its subprocessor list.** Both return HTTP 200 but serve only a
  client-rendered Vanta shell with no server-rendered text; `api.vanta.com/v1/trust-center/typesafe.ai`
  answers **401**. The certifications, the subprocessor list and any retention statement there are
  unread. Treat this as an open question, not as absence of evidence.
- **Whether `console.typesafe.ai` exposes a retention setting once signed in.** The console requires
  authentication and no account was created. No TypeSafe document mentions a console or settings
  retention control, so the **documented** answer is no; the signed-in interface is unverified.
- **Whether Cloudflare runs `typesafe/jev` on its own hardware or proxies to TypeSafe.** Cloudflare's
  model page states neither; a "Third-party" badge and an off-site licence link are the only
  signals. Separately, the sentence the assessment attributed to Cloudflare — *"we do not store this
  data nor do we train models using it"* — **does not appear** on the current Workers AI data page
  (`developers.cloudflare.com/workers-ai/platform/data-usage/`, last updated 2026-04-21). What that
  page actually says is a **no-training** commitment, *"Cloudflare does not use your Customer Content
  to (1) train any AI models made available on Workers AI…"*, which is not a no-storage commitment
  and does not address what a third-party model provider does. The assessment's Cloudflare line
  should be read as corrected by this paragraph.
- **When TypeSafe's enterprise-ZDR sentence was written.** No changelog, no revision line.
- **When the TypeSafe row was added to Vercel's ZDR table.** That page's own frontmatter says
  `last_updated: 2026-09-10`, which predates the 2026-09-16 changelog entry, so the stamp is stale
  relative to the table's contents.

## 6. What to re-check, and when

Re-read this list at the next provider decision, or if Jev is reconsidered for any kind:

1. `typesafe.ai/legal/mca` and `/data-processing` for a stated retention period or a human-review
   clause. The MCA moved on 2026-09-19; it may move again.
2. `docs.typesafe.ai/legal` for a self-serve control, and `docs.typesafe.ai/api` for a `store` or
   `zdr` request field — the cheapest single check, since one new field would settle it.
3. OpenRouter's ZDR endpoint list, for whether `typesafe/jev-1.13` is still on it. This is the term
   `assertPinnedRoute` would be enforcing, and a flag that appeared can disappear.
4. `trust.typesafe.ai`, if it ever serves readable text.

## Sources

All read 2026-09-22 unless the row states the document's own date.

- `https://docs.typesafe.ai/legal` (enterprise ZDR, no self-serve control);
  `https://docs.typesafe.ai/models` (no training; points at Legal);
  `https://docs.typesafe.ai/api` (three body fields, two headers, no retention control).
- `https://typesafe.ai/legal/privacy-policy` (last updated 2025-11-19);
  `https://typesafe.ai/legal/data-processing` (last updated 2026-04-24);
  `https://typesafe.ai/legal/mca` (last updated 2026-09-19, §4.1, §4.3, §10.3).
- `https://openrouter.ai/api/v1/endpoints/zdr` (the endpoint list);
  `https://openrouter.ai/typesafe` and `https://openrouter.ai/provider/typesafe` (`retainsPrompts: false`);
  OpenRouter's privacy-and-logging documentation (the per-request `zdr` parameter).
- `https://vercel.com/docs/ai-gateway/security-and-compliance/zdr` (the TypeSafe row and the quoted
  commitment; frontmatter `last_updated: 2026-09-10`);
  `https://vercel.com/changelog/typesafe-ai-jev-now-available-on-ai-gateway` (2026-09-16).
- Netlify's AI Gateway changelog (2026-09-17).
- `https://opper.ai/provider/typesafe` ("verified by Opper on 2026-09-18").
- `https://developers.cloudflare.com/ai/models/typesafe/jev/` (Third-party badge, no retention
  statement); `https://developers.cloudflare.com/workers-ai/platform/data-usage/` (last updated
  2026-04-21, the no-training sentence).
- `https://trust.typesafe.ai/` and `/subprocessors` (Vanta shell, unreadable);
  `https://api.vanta.com/v1/trust-center/typesafe.ai` (401).
