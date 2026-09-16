# The legal and policy floor for a cheaper inference provider

Research agent 3 of 3, 2026-09-16. Everything below was read on 2026-09-16 unless a different date is
given beside it. Provenance is marked: **(primary)** = I fetched the provider's or regulator's own page
and the quote is verbatim from it; **(search)** = the fact comes from a search engine's summary of a
primary page I could not fetch directly, and is therefore reported but not verified. "Not found" means
I looked and did not find it, never that it does not exist.

This is a landscape for a founder and, at the four questions in the last section, for a lawyer. It is
not legal advice.

---

## 0. The three things that change the shape of the decision

Before the verdicts, three findings that were not in the brief and that move the boundaries:

1. **Google's Limited Use requirements bind Knowlu's Google Calendar data exactly as hard as its Gmail
   data.** The Workspace policy's own words: *"These requirements apply to data derived from both
   **Sensitive** and **Restricted** scopes."* Knowlu holds `calendar.readonly` — a sensitive scope —
   and `ingest-calendar` fetches `calendar/v3/calendars/primary/events` with it
   (`cloud/supabase/functions/ingest-calendar/index.ts:58`). So the founder's ruling 1, which puts
   "event" in the looser bucket, is only safe for events that came from the **LMS/personal ICS feed**
   (`kind = 'calendar_ics'`), not for events that came from the Google Calendar API. The judgment
   service pins a model **per kind**, not per origin, so today a single `event` pin covers both.

2. **Nothing redacts the email prompt.** `scrub()` exists and is thorough, but it is imported only by
   `cloud/supabase/functions/issues/handler.ts`. The email prompt is built in
   `judge_prompts.ts:buildPrompt` as `Subject / From / Date / Known course slugs / Message:` where
   `Message` is the first `text/plain` part clipped to 1,200 characters, verbatim. A password-reset
   link, a magic link, a calendar capability URL, or an API key quoted inside an email reaches the
   provider intact. That is the single strongest argument for zero data retention on the email kind.

3. **Zero data retention is available self-serve, on several providers, for free.** The brief's
   worst-case hypothesis — "email's strictest self-serve tier is nowhere better than 30 days" — is
   false. Groq, Together, Fireworks, Baseten, Nebius, Cloudflare Workers AI and AWS Bedrock all offer
   true no-retention inference without an enterprise sales conversation. The majors do not: Anthropic,
   OpenAI and Vertex all gate ZDR behind approval or sales. **The current pinned provider (Anthropic
   self-serve) is therefore almost certainly *not* under the "zero-retention terms" the signed design
   §5.2 asserts**, which is a live accuracy problem in the spec independent of any provider change.

---

## 1. The verdict per data class

### 1.1 LMS task data — title, body (≤1,200 chars), course slug, grade weights, stated preferences

Not Google data: it arrives from the school's `.ics` feed and from zyBooks/VHL, fetched on device. No
statute imposes a floor at pilot scale (see §4). The floor is therefore Knowlu's own published page and
the FTC Act's prohibition on saying a false thing about it. **Minimum terms: no training on inputs,
retention only for abuse/reliability and no longer than 30 days, no discretionary human review, and a
DPA or service-provider terms that bind the provider to those words.** Every provider in the table
except Mistral (which trains by default on some products) and the Gemini API free tier (which trains
and human-reviews) meets this at self-serve. Ruling 1's 30-day allowance is fully usable here. One
caveat that is Knowlu's own, not a provider's: the privacy page's bullet *"No note bodies. The text of
your tasks and notes stays in the folder on your machine"* is already in tension with sending
`body[:1200]` to a model, and a provider change is the moment that tension becomes visible. §5.6 drafts
the fix.

### 1.2 Calendar events — split, and the split is not currently implemented

Events from the **ICS feed** (`sources.kind = 'calendar_ics'`, the LMS or the student's own secret
calendar address) sit exactly where task data sits: same floor, ruling 1's 30-day tier is fine. Events
from the **Google Calendar API** are Google user data obtained through a sensitive scope and carry the
full Limited Use set: use only for the prominent user-facing feature, transfer only to provide that
feature and only with the user's consent, no human reading outside the four exceptions, and no training
beyond that user's own personalized model. **Minimum terms for the Google half are identical to
Gmail's** (§1.3). Because the `models` table pins by kind, the practical choices are: pin `event` to the
strict provider and accept the cost on ICS events too; or add origin-aware routing so
`origin = 'events'` rows that came from the Calendar API take the strict pin. A second, separate defect:
the training-export filter is `select * from judgments where origin <> 'gmail_api'`
(`cloud/supabase/migrations/20260911000200_google.sql:100`), so Calendar-API-derived rows are exported
today. That is a Limited Use gap that exists whatever provider is chosen.

### 1.3 Gmail message text — the strict class

Limited Use applies in full, and the privacy page has already promised more than Google demands.
**Minimum terms: (a) the provider must not train any model on inputs or outputs, with no opt-in
default anywhere in the chain; (b) no discretionary human review of prompts — review only for a
security purpose, which Limited Use's own third bullet permits; (c) retention as short as the provider
offers, and given §0.3 that means true zero data retention, because it is obtainable self-serve and
choosing worse would be a deliberate downgrade of a published promise; (d) the provider must be
nameable on the privacy page and stable enough to stay named.** Providers meeting all four at
self-serve, from the table: **Groq, Together, Fireworks, Baseten, Nebius (with the ZDR mode on),
Cloudflare Workers AI, and AWS Bedrock for models not on its abuse-detection exception list.**
Providers that do not meet it self-serve: Anthropic (ZDR via sales; standard tier is 30 days, and up to
2 years plus 7 years of classifier scores on a detected violation), OpenAI (ZDR by prior approval),
Gemini API paid tier (logs "for a limited period", no self-serve exemption), Vertex (30-day abuse
logging, exemption by form), Mistral (opt-in-by-default training on some products), OpenRouter
(structurally — see §1.5). Hyperbolic and SambaNova are **not verifiable from public text** and should
not be used for this class on the strength of a marketing sentence.

Direct answer to the brief's question 1: **a provider that keeps prompts 30 days for abuse monitoring,
with no training and no human review, *is* compatible with Limited Use for Gmail data** — the policy
contains no retention-duration clause at all, and both the transfer and the retention fall inside its
own security exception. It is not, however, compatible with the sentence Knowlu has already published,
and §5 says what would have to change. See §3.3 for the one place where 30 days does create real
friction: restricted-scope verification's deletion-on-request expectation.

### 1.4 Consented corrections used as eval cases — much lower risk than it looks

By construction the correction record carries no message text. `fieldsOf` drops `title`, `why` and
`importance_reason` before anything is logged (`judge_pipeline.ts`), the design's §5.2 says the log
holds *"ids, field values, confidences and the four promotion features — never the body"*, and the
privacy page adds that a title correction *"is not recorded at all"*. So replaying stored corrections
cannot leak an email body, because no email body was stored. **Minimum terms: whatever the task class
requires; no additional floor.** The sharp line is the other direction: **if the email eval is ever
made real by persisting the request text, that store becomes a database of Gmail message bodies**, and
Google's API Terms of Service forbid exactly that — *"Scrape, build databases, or otherwise create
permanent copies of such content, or keep cached copies longer than permitted by the cache header."*
The email eval corpus must come from the founder's own archived vault seed (§5.4 of the design already
says so) or from examples collected under a separate, explicit consent — never from a live user's
inbox by accumulation.

### 1.5 One structural note: OpenRouter

OpenRouter says *"OpenRouter does not use your Inputs or Outputs for model training"* but also *"Some
Model Providers may use your Inputs and Outputs for model training or improvement."* Its ZDR control is
a routing filter, which is a real and useful control — but the set of companies that touch a given
request is decided per request. Knowlu's privacy page names its processors by company and its own
"Changes to this policy" clause requires an email and an in-app yes before *"a new company on the
list"*. A router whose membership changes without a policy version cannot be reconciled with that
sentence, and Limited Use's *"transferred to one place"* framing on the Gmail bullet would have to go.
**Recommendation: OpenRouter is usable as a price-discovery and eval tool, not as the production path
for any class of Knowlu data, unless the page is rewritten to name a router rather than a processor.**

---

## 2. The provider table (self-serve tier only)

All rows read 2026-09-16. Columns: (a) training on inputs, (b) retention of prompts/outputs,
(c) human review, (d) ZDR availability, (e) DPA, (f) subprocessors/regions, (g) AUP and third-party
personal data.

| Provider | (a) Training | (b) Retention | (c) Human review | (d) ZDR self-serve? | (e) DPA | (f) Subproc./regions | (g) AUP on others' email |
|---|---|---|---|---|---|---|---|
| **Anthropic** (Claude API) | No — *"Anthropic may not train models on Customer Content from Services."* [Commercial ToS, eff. 2025‑06‑17](https://www.anthropic.com/legal/commercial-terms) (primary) | Inputs/outputs deleted within 30 days by default; up to **2 years** on a detected usage violation and **7 years** of trust‑and‑safety classifier scores (search, via [privacy.claude.com](https://privacy.claude.com/en/articles/7996866-how-long-do-you-store-my-organization-s-data)) | On flagged violations (search) | **No** — *"To request ZDR for your organization, contact the Anthropic sales team."* [platform.claude.com ZDR docs](https://platform.claude.com/docs/en/manage-claude/api-and-data-retention) (primary) | Published commercial DPA (not verified as click‑through) | trust.anthropic.com; US | Not found |
| **OpenAI** | No by default — *"As of March 1, 2023, data sent to the OpenAI API is not used to train or improve OpenAI models (unless you explicitly opt in…)"* [developers.openai.com/api/docs/guides/your-data](https://developers.openai.com/api/docs/guides/your-data) (primary) | Abuse logs *"retained for up to 30 days, unless longer retention is required by law"* (primary) | Yes under stated safety conditions, with advance written notice, even under ZDR/MAM (primary) | **No** — *"Currently, these controls are subject to prior approval by OpenAI."* (primary) | Click‑through DPA in the platform console (search) | Published; US/global | Not found |
| **Google Gemini API — free tier** | **Yes** — *"Google uses the content you submit to the Services and any generated responses to provide, improve, and develop Google products"* [ai.google.dev/gemini-api/terms, last updated 2026‑04‑28](https://ai.google.dev/gemini-api/terms) (primary) | Not specified (primary) | **Yes** — *"Human reviewers may read, annotate, and process your API input and output"* (primary) | No | n/a | Global | **Disqualified for every Knowlu class** |
| **Google Gemini API — paid tier** | No — *"Google doesn't use your prompts (including associated system instructions, cached content, and files…) or responses to improve our products"* (primary) | *"Google logs prompts and responses for a limited period of time, solely for detecting and preventing violations of the Prohibited Use Policy"* — duration **not stated** (primary) | Not stated for paid (primary) | No | Google Cloud terms | Global | Not found |
| **Google Vertex AI** | No — prompts/responses *"won't be used to train or fine-tune any AI/ML models"* (search, [abuse-monitoring docs](https://docs.cloud.google.com/vertex-ai/generative-ai/docs/learn/abuse-monitoring)) | *"All prompts and responses will be logged and securely stored for up to 30 days for the sole purpose of monitoring for abuse"* under the GCP ToS (search) | Not stated; Claude‑on‑Vertex requires enabling data sharing with Anthropic for abuse monitoring (search) | **No** — exemption *"by filling out a form"*; customers on a Google Cloud Master Agreement are exempt by default (search) | Google Cloud DPA, incorporated | Region‑selectable | Not found |
| **Cloudflare Workers AI** | **No** — *"Cloudflare does not use your Customer Content to (1) train any AI models made available on Workers AI or (2) improve any Cloudflare or third-party services"* [data‑usage docs, updated 2026‑04‑21](https://developers.cloudflare.com/workers-ai/platform/data-usage/) (primary); trust hub adds *"We do not store this data nor do we train models using it"* [trust-hub/responsible-ai](https://www.cloudflare.com/trust-hub/responsible-ai/) (primary) | **Not found as a duration.** The docs say content is stored only if you use Cloudflare storage services alongside Workers AI (primary) | Not found | Effectively default; **no toggle documented** | **Yes** — Customer DPA v6.4 (eff. 2026‑04‑03) *"forms part of your Main Agreement, whether that is an Enterprise Subscription Agreement, a **Self‑Serve Subscription Agreement**, or another written or electronic agreement"* [cloudflare.com/dpa](https://www.cloudflare.com/cloudflare-customer-dpa/) (search) | Published subprocessor list; global edge | Not found |
| **Groq** | **No** — *"Groq is not permitted to use Inputs or Outputs for training or fine-tuning any AI Model Services or other models, unless explicitly granted permission or instructed by Customer."* [Services Agreement](https://console.groq.com/docs/legal/services-agreement) (primary) | *"By default, Groq does not retain customer data for inference requests."* Temporary logs only *"when troubleshooting errors that degrade platform reliability, or investigating suspected abuse… These logs are retained for up to 30 days."* [console.groq.com/docs/your-data](https://console.groq.com/docs/your-data) (primary) | Not found | **Yes** — *"All customers may enable Zero Data Retention (ZDR) in Data Controls settings. Organization admins can decide to enable ZDR globally or on a per-feature basis at any time."* (primary) | **Yes, automatic** — *"By entering into the Agreement, Customer and Groq are deemed to have signed"*; [DPA eff. 2025‑10‑15](https://console.groq.com/docs/legal/customer-data-processing-addendum) (primary) | trust.groq.com/subprocessors; *"All customer data is retained in Google Cloud Platform (GCP) buckets located in the United States."* (primary) | Not found |
| **Fireworks** | No without opt‑in (search, [data‑handling docs](https://docs.fireworks.ai/guides/security_compliance/data_handling)) | *"Fireworks does not log or store prompt or generation data for any open models, without explicit user opt-in"*; prompt/generation data *"exist only in volatile memory for the duration of the request"*. Responses API with `store=True` retains 30 days (primary) | Not found | **Yes — ZDR is the default** for open models; opt‑in is what turns logging *on* (primary) | Not found | Not found | Not found |
| **Together** | No — *"We do not use any data collected from you to train our models without your explicit opt-in and consent."* [Privacy policy, eff. 2025‑12‑17](https://www.together.ai/privacy) (primary) | Under ZDR *"not stored, retained, or used for model training, product improvements, or any secondary purposes except as needed to provide the Services to you"* (primary) | Not found | **Yes** — enabled in Settings → Profile by answering "No" to storing prompts / model training (primary) | **No** — DPA/BAA via sales@together.ai (search) | Not enumerated in the policy | Not found |
| **DeepInfra** | No — *"We will not store, sell, or train using this data unless we have your explicit consent."* [Privacy policy, eff. 2026‑08‑15](https://deepinfra.com/privacy) (primary) | Same sentence covers storage; metadata logs keep IP, timestamp and interaction (primary) | Not found | Effectively default; **no toggle found** | Not found | US (primary) | Not found |
| **Mistral** | **Trains by default on some products** — *"Mistral AI will not use Customer Data or Outputs to train its artificial intelligence models except (a) when you (i) opted-in to training on a Mistral AI Product set to opt-out by default or (ii) **have not opted-out of training on a Mistral AI Product set to opt-in by default**…"* [Commercial ToS, eff. 2026‑08‑05](https://legal.mistral.ai/terms/commercial-terms-of-service) (primary) | Not specified (primary) | Not found | ZDR referenced only for Labs/Preview models (primary) | Published DPA | EU | **Disqualified for Gmail** until the per‑product default is verified in writing |
| **AWS Bedrock** | No; and *"Retained inputs and outputs are stored and processed by AWS and are not shared with third-party model providers"*; *"Model providers don't have any access to Amazon Bedrock logs or to customer prompts and completions."* [Bedrock abuse‑detection + data‑protection docs](https://docs.aws.amazon.com/bedrock/latest/userguide/abuse-detection.html) (primary) | *"Amazon Bedrock uses a zero data retention (ZDR) data security model. This means that by default, Amazon Bedrock does not store model inputs or outputs."* Named exception models retain 30 days (primary) | *"zero operator access (ZOA)… no operators of the service can access model input or output."* Exception: for Claude Fable 5/5.1, *"Classifier-flagged traffic will be subject to potential human review performed by AWS."* (primary) | **Yes, by default**, for any model not on the exception list. Full ZDR for exception models is *"through their AWS account team"* (primary) | AWS GDPR DPA / Service Terms, incorporated | Region‑selectable; US regions available (primary) | AWS AUP; not specifically verified |
| **OpenRouter** | OpenRouter itself no — *"OpenRouter does not use your Inputs or Outputs for model training."* But *"Some Model Providers may use your Inputs and Outputs for model training or improvement."* [openrouter.ai/privacy](https://openrouter.ai/privacy) (primary) | Ephemeral for routing, *"except as required for abuse detection, security, billing, or legal compliance"* (primary) | Not found | **Yes, as a routing filter** — per‑request or account‑wide restriction to providers with a given data policy (primary) | Not found | Upstream providers act as subprocessors; **the set varies per request** | Not found |
| **Baseten** | No (implied by ZDR posture) | *"Zero Data Retention means that Baseten shall not (a) log, record, or store Customer Content; or (b) save Customer Content to persistent storage… after the real-time processing required for the provision of the Services."* Model APIs and Dedicated Inference adhere to ZDR (search, [docs.baseten.co/observability/security](https://docs.baseten.co/observability/security)) | Not found | **Yes — ZDR posture is the product default** for Model APIs (search) | Not found (Trust Center at trust.baseten.co) | Not found | Not found |
| **Nebius** (AI Studio / Token Factory) | No in either mode — *"your content is not used to train any models in either mode"* (search, [docs.tokenfactory.nebius.com](https://docs.tokenfactory.nebius.com/legal/legal-quick-guide)) | **Default is *not* zero**: inputs and outputs are kept to accelerate inference via speculative decoding. With ZDR on, *"your inputs (prompts) and outputs (responses) are not stored on our systems after each request is processed"* (search) | Not found | **Appears yes** (a mode you enable) — **verify before relying on it** | Not verified | SOC 2 Type II / HIPAA / ISO 27001 facilities; EU‑headquartered | Not found |
| **Hyperbolic** | *"private customer data is not used to train public AI systems without explicit permission"* (search) | **Not found** | Not found | **Not found** | Not found | Not found | Not found |
| **SambaNova** (SambaCloud) | Marketing claim only: *"SambaCloud never sees or collects any of your data or user prompts"* (search) | Privacy policy says data is erased or anonymized *"On the expiry of the applicable data retention period"* — **duration not stated** (search) | Not found | **Not found as a toggle** | EULA published | Not found | Not found |

**On column (g) generally.** No provider's public acceptable-use terms that I read address, one way or
the other, whether a customer may submit a third party's email on an end user's behalf. What they all
do is put the representation on the customer: you warrant you have the rights to the data you submit.
For Knowlu that warranty is satisfied by the student's own OAuth grant to their own mailbox — the
student is the account holder, and the correspondents' text arrives as a normal incident of that. The
residual question, which no public AUP settles, is the institutional mailbox: see §6, question 5.

---

## 3. The policy analysis, with the operative sentences

### 3.1 What Limited Use actually says

From [Google Workspace user data and developer policy](https://developers.google.com/workspace/workspace-api-user-data-developer-policy),
last updated **2026‑09‑03** (the spec cites a 2026‑07‑22 version; the page has moved since and the
citation should be refreshed):

> "Upon accessing Google Workspace scopes for an appropriate use, your use of the data obtained must
> comply with the below requirements. **These requirements apply to data derived from both Sensitive
> and Restricted scopes.**
>
> 1. Limit your use of data to providing or improving your appropriate use case or features that are
>    visible and prominent in the requesting application's user interface.
> 2. Transfers of data are not allowed, except:
>    - To provide or improve your appropriate use case or user-facing features that are visible and
>      prominent in the requesting application's user interface and only with the user's consent;
>    - **For security purposes (for example, investigating abuse);**
>    - To comply with applicable laws and/or regulations; or,
>    - As part of a merger, acquisition or sale of assets of the developer after obtaining explicit
>      prior consent from the user.
> 3. Do not allow humans to read user data, unless:
>    - You have obtained and documented the user's explicit consent or affirmative agreement to view
>      specific messages, files, or other data…;
>    - The data (including derivations) is aggregated and anonymized and used for internal operations…;
>    - **It's necessary for security purposes (for example, investigating a bug or abuse);** or,
>    - To comply with applicable laws and/or regulations."

And the AI/ML clause, from the prohibited-uses list on the same page:

> "Transferring, selling, or using user data to create, train, or improve a machine learning or
> artificial intelligence model **beyond that specific user's personalized model** for the appropriate
> use case or user-facing feature."

Google's own gloss on "personalized model" (Cloud Console help, [answer 13463817](https://support.google.com/cloud/answer/13463817)):

> "For purposes of the Workspace API Policy, personalized models include any models run exclusively
> on-device or a model specifically tailored to only that end user or organization. **Personalized
> models do not co-mingle Google User Data with other end users or organizations.**"

That last sentence is why the design's per-account rule promotion is legal and why the global-rule
promotion path (*"same pattern across ≥ 20 accounts on the same campus source"*, design §5.4 measure 1)
must never be fed by a `gmail_api`-origin row — which the export filter already enforces for Gmail and
does **not** enforce for Calendar-API rows.

### 3.2 The retention question, answered

**There is no retention duration in the Limited Use policy.** I searched the Workspace policy, the
general [API Services User Data Policy](https://developers.google.com/terms/api-services-user-data-policy)
(last updated 2024‑02‑15) and the Cloud Console FAQ; none of them state a maximum, a minimum, or a
"shortest possible" standard for how long a developer or its processors may hold Workspace data. The
closest thing to a retention constraint is in the Google APIs Terms of Service:

> "Scrape, build databases, or otherwise create permanent copies of such content, or keep cached copies
> longer than permitted by the cache header."

So: a provider that retains a prompt for 30 days solely to investigate abuse of its own service, does
not train on it and does not read it outside a security investigation, is **doing a thing Limited Use
expressly permits** — the transfer falls under bullet 2's security exception, any review falls under
bullet 3's, and no retention clause is violated because there is none. A 30-day abuse log is not a
"permanent copy" and is not a cache of API content in the ToS's sense. **Answer to question 1: yes,
compatible.** The word "shortest" in the brief is Knowlu's own standard, not Google's.

### 3.3 Where 30 days does bite: verification and deletion

Restricted-scope verification and the annual CASA assessment exist to verify, in Google's words, that
apps *"can handle data securely and delete user data upon a user's request"*
([restricted scope verification](https://developers.google.com/identity/protocols/oauth2/production-readiness/restricted-scope-verification), search).
A provider-held 30-day abuse log is data Knowlu cannot purge on demand: when a student presses
**Settings → Delete my data**, the founder can truthfully delete every row and object on Knowlu's side
and cannot reach a Groq or Bedrock abuse log. Whether the deletion expectation reaches a subprocessor's
security log is the single unresolved compliance question in this report (§6, question 3). ZDR removes
it entirely, which is a second, independent reason to put Gmail on a ZDR provider: **it is the only
configuration where the Delete-my-data button is unqualifiedly true.**

### 3.4 Calendar

Google Calendar scopes are **sensitive, not restricted** — no Calendar scope appears on Google's
restricted-scope list ([Cloud Console answer 13464325](https://support.google.com/cloud/answer/13464325), primary),
which confirms the spec's §5.3 claim that asking for calendar write later does not enlarge the CASA
assessment. But sensitive scopes carry the identical Limited Use set, as §3.1's first quoted sentence
says in terms. The spec's §9 policy table treats the Workspace policy as *"binding on all Gmail data,
always"*; it should read "on all Workspace data, always."

---

## 4. US law, for a one-person company with students in Alabama and Kentucky

**Comprehensive state privacy laws: none applies at pilot scale, and the spec is right.**

- **Kentucky (KCDPA)** took effect **2026‑01‑01**. Applicability: control or process the personal data
  of **100,000+** Kentucky consumers in a year, or 25,000+ Kentucky consumers while deriving **>50%** of
  gross revenue from selling personal data. There is no revenue-alone threshold
  ([Bricker](https://www.bricker.com/hitech-law/the-kentucky-consumer-data-protection-act-goes-into-effect-on-january-1-2026-are-you-ready), search).
- **Alabama (APDPA, HB 351)** takes effect **2027‑05‑01**. Applicability: personal data of **more than
  25,000** consumers (excluding data processed solely to complete a payment transaction), or **>25%** of
  gross revenue from the sale of personal data
  ([White & Case](https://www.whitecase.com/insight-alert/alabama-enacts-comprehensive-data-privacy-law), search).
  The 25,000 figure is among the lowest numeric floors in the country, so this is the one to watch — but
  it is a 2027 problem and the pilot is two orders of magnitude below it.

At those thresholds, **no state comprehensive law compels a DPA with an inference provider today.**
What does still apply regardless of size is **CalOPPA** (any commercial site collecting PII from
Californians must post a conspicuous privacy policy naming the categories collected and the categories
of third parties it is shared with — the page already does this), and **FTC Act § 5**, under which a
materially false statement on the privacy page is a deceptive practice. §5 is the real enforcer here:
it makes the privacy page's sentences legally operative even where no privacy statute reaches.

**Alabama breach law, § 8-38.** The statute defines *"sensitive personally identifying information"* as
an Alabama resident's first name or initial and last name in combination with one or more listed
elements, including —

> "A user name or email address, in combination with a password or security question and answer that
> would permit access to an online account affiliated with the covered entity that is reasonably likely
> to contain or is used to obtain sensitive personally identifying information."

([Ala. Code § 8-38-1](https://acua.alabama.gov/PDF/law/AlabamaDataBreachNotificationAct.pdf), search;
notice deadline is **45 days** under § 8-38-5.) The statute's literal element is username/email plus a
password or security Q&A. The spec's characterization — "OAuth tokens are SPII" — is a defensible
reading of the purpose (a bearer token grants the same access a password does) but is **not the
statute's literal words**, and a lawyer should confirm it (§6, question 4). Two other statutory hooks
matter more than the definition: § 8-38-3 makes *contractual requirements imposed on third-party
agents* an explicit factor in whether security was "reasonable," which is the best legal argument for
insisting on a DPA even where none is compelled; and § 8-38-2 makes the trigger the *unauthorized
acquisition* of SPII, so a provider lawfully holding a prompt is not a breach — a breach of that
provider's store would be.

**Do prompts ever carry a token? Read from the code, honestly: yes, by accident, never by design.**

- By design: no. `buildPrompt` inserts only title/body/due/course/weights/preferences (task),
  title/time/source/organizer/location/categories/audiences/description/interests (event), and
  subject/from/date/known-course-slugs/message-text (email). `config/ingest.yaml` never leaves the
  device. The Google refresh token lives in Supabase Vault and never enters a judgment path. The ICS
  capability URL is encrypted in `sources` (`url_ciphertext`, `url_iv`) and is read by the fetcher, not
  the prompt builder.
- By content: **yes.** The email prompt carries the first 1,200 characters of the message's `text/plain`
  part verbatim, and nothing redacts it (§0.2). Password-reset links, magic links, LMS capability URLs
  and pasted keys all live in ordinary student email. A task `body` or an event `description` can carry
  the same.
- **Consequence.** Under a 30-day provider log this puts credential-shaped strings at a third party for
  30 days; under ZDR it does not persist at all. **Recommendation: put the email kind on ZDR, and
  additionally run the existing `CREDENTIAL` and `TOKEN` patterns from `scrub.ts` over the email prompt
  before it leaves — but not the `URL_RE` pattern**, because a Canvas or Handshake link is one of the
  strongest classification signals the model has and blanking it would cost accuracy for no privacy
  gain that ZDR has not already delivered.

**The sub-processor relationship, practically.** Absent an applicable comprehensive law, what the
founder actually needs is: (1) terms that bind the provider to the no-training/limited-retention words —
a published DPA that incorporates automatically (Groq, Cloudflare, AWS) is the cheapest way to have
this without a lawyer's time; (2) the provider named in the privacy notice, which Google's Limited Use
effectively requires for the Gmail transfer to be "with the user's consent" and which CalOPPA requires
as a category; (3) the notice-and-ask step the privacy page itself imposes on adding a company (§5.5).

---

## 5. The privacy-page sentence changes

Five sentences on `site/privacy.html` are affected, plus one already-existing tension. Drafts below;
`<Provider>` is a placeholder for the chosen company's name.

### 5.1 The processor list — the sentence that breaks first

**Today:** *"**Anthropic** — the API that makes the judgments, and **the only place your content is ever
sent to be read by a model**: one item at a time, to answer one question about that item, and only while
a judgment is being made. Anthropic's commercial API terms say they do not train their models on what is
sent through it. We hold the text for the length of the call and then let it go; what we write down is
the answer."*

**Any provider change makes this false**, because it names Anthropic. If the founder takes the
**single-provider, ZDR** path:

> **\<Provider\>** — the API that makes the judgments, and **the only place your content is ever sent to
> be read by a model**: one item at a time, to answer one question about that item, and only while a
> judgment is being made. \<Provider\>'s terms say they do not train any model on what is sent through
> it, and Knowlu's account has zero data retention switched on, so nothing you send is written down on
> their side once the answer comes back. We hold the text for the length of the call and then let it go;
> what we write down is the answer.

If the founder takes the **split** path (a 30-day provider for tasks and ICS events, a ZDR provider for
email and Google Calendar), the one bullet becomes two and the count in the sentence above it —
*"Four companies, each doing one job"* — becomes five:

> **\<Provider A\>** — the API that judges your assignments and the events on your school's calendar
> feed. It does not train any model on what is sent through it and no person there reads it. It may keep
> a copy of a request for up to 30 days to investigate abuse of its own service, and then deletes it.
>
> **\<Provider B\>** — the API that judges your email and the events on your Google calendar, and the
> stricter of the two: zero data retention, so nothing sent to it is written down on their side at all.
> Nothing that came from Google is ever sent anywhere else.
>
> These two are **the only places your content is ever sent to be read by a model**: one item at a time,
> to answer one question about that item, and only while a judgment is being made.

### 5.2 The Gmail "one place" bullet

**Today:** *"It is transferred to **one place**: Anthropic's API, to make that judgment for you. Nowhere
else, and never for advertising, credit, lending or resale."*

**Under a ZDR provider:**

> It is transferred to **one place**: \<Provider\>'s API, to make that judgment for you, under
> zero-data-retention terms — nothing is stored there. Nowhere else, and never for advertising, credit,
> lending or resale.

**Under a 30-day provider** (if the founder overrides the recommendation):

> It is transferred to **one place**: \<Provider\>'s API, to make that judgment for you. \<Provider\>
> does not train any model on it and no person there reads it; it keeps a copy for up to 30 days so it
> can investigate abuse of its own service, and then deletes it. Nowhere else, and never for
> advertising, credit, lending or resale.

### 5.3 The two "discarded" sentences

Both are about what *Knowlu* holds, so a 30-day provider does not make either literally false — but the
plain reading a student or a Google reviewer takes from them is that the text does not persist anywhere,
and leaving them unqualified beside a 30-day provider is the kind of technically-true sentence this page
has otherwise refused to write.

**Today (Gmail section):** *"Message text is **discarded when the judgment is done**."*

**Under a 30-day provider:**

> Message text is **discarded by us when the judgment is done**, and by the provider that read it within
> 30 days.

**Today (the "What we collect" list):** *"Message metadata and text, **held in memory for the length of
one judgment and then discarded**."*

**Under a 30-day provider:**

> Message metadata and text, **held in memory for the length of one judgment and then discarded** — and
> held by the model provider that reads it for no longer than 30 days, to investigate abuse of its own
> service.

Under a ZDR provider **both sentences stay exactly as written**, which is the cleanest argument for ZDR.

### 5.4 The human-review bullet — no sentence edit can rescue it

**Today:** *"**No human at Knowlu reads your mail without your explicit consent.**"* This stays true
whatever the provider does, because it speaks only for Knowlu. But Limited Use bullet 3 speaks for the
whole chain, and it permits human reading only for consent, aggregation-and-anonymization, security, or
law. **A provider that reserves discretionary human review of prompts cannot be used for Gmail or
Google Calendar data at all**, and the fix is provider selection, not wording. (This is what removes
the Gemini API free tier from consideration outright, and what makes AWS Bedrock's Claude Fable
exception models unusable for the Gmail kind while leaving the rest of Bedrock fine.)

### 5.5 The clause that makes a provider swap a product event, not a config change

**Today, under "Changes to this policy":** *"If we change something that matters — a new category
collected, **a new company on the list**, a new use for what we already hold — we will email the address
on your account, and Knowlu will show you the change and ask, before the new use begins. **Carrying on
using Knowlu is not how we take consent for a materially different use of your data**; a yes is."*

Knowlu has bound itself: **swapping the inference provider requires a new `privacy_version` on the page,
an email to every account, and an in-app accept before the first request goes to the new provider.** At
pilot scale that is a small job, and doing it now is much cheaper than doing it at 500 accounts. It is
also a reason to make the provider decision once and pin it, rather than to route dynamically.

### 5.6 A pre-existing tension the change will expose

**Today, under "What we do not collect":** *"**No note bodies.** The text of your tasks and notes stays
in the folder on your machine. If that ever changes it will be a separate switch, off by default…"*

The task judgment already sends `item.body[:1200]`, the student's stated preferences and their grade
weights to a model. The sentence sits under a heading about collection and is arguably about storage,
but "stays in the folder on your machine" is stronger than "we keep no copy," and a careful reader
would call it wrong. Recommended replacement, regardless of which provider is chosen:

> **No note bodies stored.** The text of your tasks and notes lives in the folder on your machine and we
> keep no copy of it. Up to the first 1,200 characters of one note travel to the model that judges it,
> for the length of that one call, and are written down by neither side. If we ever start keeping them
> it will be a separate switch, off by default, with its own screen explaining what and why — and no
> such switch exists today.

### 5.7 Nothing on `site/terms.html` needs to change

The terms page promises *"Your vault is a folder on your own machine"* and *"Nothing in these terms gives
us any ownership of what is in it"* — both survive any provider choice. The terms name no processor.

---

## 6. Open questions for a lawyer

Things I could not settle from public text, in the order they would bite.

1. **Does Limited Use's "only with the user's consent" on transfers require naming the inference
   provider on the OAuth consent screen itself, or is naming it in the linked privacy policy enough?**
   Universal practice is the policy, and Google's verification reviewers read the policy; but the
   policy's own words attach the consent to the transfer, not to the disclosure. Cheap to over-comply:
   name the provider in the wizard's Gmail step as well as on the page.

2. **Is a subprocessor's 30-day abuse log a "cached copy" or a "permanent copy" under the Google APIs
   ToS?** I read it as neither — it is a security log within bullet 2's express exception — but the ToS
   sentence is the only retention-shaped language Google has, and a reviewer could read it the other
   way.

3. **Does restricted-scope verification's deletion-on-request expectation reach a subprocessor's
   security log the developer cannot purge?** If yes, only a ZDR provider lets the "Delete my data"
   button be unqualifiedly true for Gmail, and this question decides the email pin on its own. This is
   the most consequential unresolved item in the report.

4. **Is an OAuth access or refresh token, or a calendar capability URL, "sensitive personally
   identifying information" under Ala. Code § 8-38-1 when it is not paired with a first name?** The
   statute's enumerated element is username/email *plus a password or security question and answer*.
   The design's §9 asserts tokens are SPII; if that is right the incident plan is correctly scoped, and
   if it is wrong the scope is narrower — either way it is worth one sentence of advice, because it
   sets what triggers the 45-day clock.

5. **Can a student validly consent to third-party processing of a mailbox their university owns?** The
   wizard recommends a personal account and Google's admin controls can block the app, but that is a
   technical control, not an answer to who the data controller of `@crimson.ua.edu` mail is. If the
   university is the controller, the student's OAuth grant may not be the consent Limited Use bullet 2
   requires. This is the question that scales badly if the pilot widens on campus mail.

6. **Does replaying a consented correction through a model to evaluate that model count as "improving a
   machine learning model beyond that specific user's personalized model"?** Today the question is
   academic for Gmail, because no message body is stored and `origin = 'gmail_api'` rows are excluded
   from the export. It stops being academic the moment anyone proposes storing request text to make the
   email eval real, and §1.4 says why that line should simply never be crossed.

7. **Is a written DPA with the inference provider legally required of a controller to whom no state
   comprehensive law applies, or is the obligation entirely FTC § 5 (don't lie on the page) plus
   § 8-38-3's reasonable-security factor?** This decides whether a provider with no published DPA —
   Fireworks, DeepInfra, Baseten as of this reading — can be used at all, or only disfavoured.

8. **Carried forward from P5 #6, unchanged:** the no-arbitration paragraph and the limitation-of-
   liability paragraph on `site/terms.html` still want a lawyer's eyes before the first non-founder paid
   sign-up. Nothing in this study changes that recommendation.

---

## 7. What I would do, in one paragraph

Pin **email and Google-Calendar-origin events to a self-serve ZDR provider** — on public terms alone
**Groq** is the strongest candidate (explicit no-training clause in the Services Agreement, a
self-serve ZDR toggle in Data Controls, a DPA that incorporates automatically, US-only storage, a
published subprocessor list), with **AWS Bedrock** the strongest alternative for its ZDR-plus-zero-
operator-access default and **Together** or **Fireworks** close behind. Pin **task and ICS-event
judgments** wherever price and quality say, so long as the provider does not train and does not
discretionarily human-review; the 30-day allowance in ruling 1 is genuinely available there. Do not use
Mistral for anything Google-derived until its per-product training default is confirmed in writing, do
not use the Gemini API free tier for anything at all, and treat OpenRouter as an eval tool rather than
a production path. Then do the three things that are Knowlu's own to fix and are true regardless of
provider: extend the training-export filter to Calendar-API-origin rows, redact credential and token
patterns from the email prompt, and run the privacy page's own notice-and-ask when the new company goes
on the list.
