# quinn-ops — Technical & Business Planning Document

> ## ⚠ PARTLY SUPERSEDED
>
> A companion document was written after this one and **supersedes parts of it**:
> **`docs/superpowers/notes/2026-09-01-market-pricing-and-distribution.md`**. Its §0 is the
> authoritative list. The headline changes:
>
> - **§9.3's pricing is OBSOLETE.** $4.99/mo with no free tier is replaced by a permanent
>   free local-only tier + **$9.99/mo** + **$69.99/academic year**.
> - **§2.4 / §3's Blackboard REST connector is DEPRIORITIZED.** The ICS feed is connector #1
>   — which this repo has always used. REST only if grades or completion state are needed.
> - **§10.3's Year 1 projections must be recalculated** at the new price.
> - **§10.2's 0.1–5% campus adoption is too pessimistic as a ceiling** — Coursicle reaches
>   80–90% at some campuses, over years.
>
> **Everything else in this document stands.** Read that document's §0 before acting on any
> section here, and §12 below before either — the rulings still win over both bodies.

**Provenance:** written by Quinn in a separate session, September 2026; filed here verbatim on
2026-09-01 because nothing about this project may live only in a chat transcript. The rulings Quinn
gave when handing it over are recorded in §12 at the end — **§12 post-dates the body, and where they
conflict §12 wins.**

*Reflects the most recent decisions where our thinking changed mid-conversation.*

**Launch configuration: desktop only, University of Alabama pilot, University of Kentucky as first
expansion test.**

---

## 1. Product framing (unchanged from the original spec)

A personal operations system that answers one question every morning — *what should I work on today,
and in what order?* — from sources that already know your obligations, without manual entry.

Two design commitments from the original spec drove nearly every technical decision below:

- **The engine is deterministic.** Same input, same output. No inference inside it.
- **Local-first.** Not a cloud service that owns user data.

One commitment was added during this session and significantly reduced the system's footprint:

- **The AI never produces anything the user sends externally.** No drafted emails, notes, or
  messages. Its only generative output is internal proposals and info displays.

---

## 2. AI architecture

### 2.1 Colibri — evaluated and rejected

Colibri is a pure-C inference engine built to run frontier MoE models (744B–2.8T parameters) on
consumer hardware by streaming experts from NVMe storage. Its entire value proposition is solving
"model too large for RAM."

**Not a fit.** Our problem is the inverse — constrained devices running small models that already fit
in memory. Adopting it would mean disk-streaming overhead, no speed guarantees, and a narrow set of
supported model families, in exchange for zero benefit.

### 2.2 The core principle: minimize the AI surface

Every capability was sorted by whether it needs a model at all. The large majority do not.

| Capability | Implementation | Rationale |
|---|---|---|
| Structured ingestion (LMS API, calendar, ICS) | Pure script | JSON/ICS parsing, zero ambiguity |
| Unstructured extraction (deadline mentioned in an email) | Tiny model, schema-constrained | "Read text → fill five fields," not conversation |
| Ranking / prioritization | Deterministic weighted scoring | Tie-breaker and scheduler, not a global ordering |
| Capacity computation | Pure script | Free/busy interval math |
| Approval routing | Pure script | A permissions rule, not a judgment call |
| Completion detection (structured sources) | Pure script | Source already reports state |
| Completion detection (inferred from a thread) | Small classifier | Binary judgment, not generation |
| Opportunity relevance filtering | Embeddings + small per-user classifier | Trains in ms, explainable, stays local |
| Duplicate detection | Small classifier | Same pattern |
| Proposals / info display | Small instruct model + LoRA adapter | The only remaining generative task |
| Audit log / back-traceability | Pure script | Data structure, not AI |

### 2.3 Techniques for keeping models small

- **Schema-constrained decoding on every model call, without exception.** Grammar-level enforcement
  of output format removes an entire class of failure and is the single highest-leverage technique
  for making small models reliable.
- **Rule promotion loop.** Log every model input/output pair. When a deterministic rule reproduces
  the model's output across repeated occurrences, promote it into the engine and stop calling the
  model. The model handles a shrinking long tail over time.
- **Cascading fallback.** Try the heuristic first; invoke the model only on low confidence.
- **Classification over generation.** Any "yes/no/which" judgment becomes a trained classifier on
  embeddings, not an LLM call.
- **Templates over composition.** Proposals fill slots in a template library; the model personalizes
  a few fields.
- **Task-specific LoRA adapter** on a small base model, trained on our narrow extraction schema —
  likely outperforms a much larger general model at this specific job.
- **Avoid reasoning-tuned models.** Chain-of-thought spend buys little on templated proposal
  generation.

### 2.4 Recommended split: local extraction, cloud generation

| Layer | Location | Approx. size |
|---|---|---|
| Embedding model (classification, dedup, relevance) | **Local** | 0.3–1.5 GB (nomic-embed ~0.3 GB; Qwen3-Embedding-0.6B ~1.5 GB) |
| Extraction model (1–4B, schema-constrained) | **Local** | ~1–2.5 GB |
| Proposal generation | **Cloud** (zero-retention provider) | 0 GB local |
| Scraper bootstrapping (browser + vision) | **Cloud job** | 0 GB local |

**Rationale:** the local layer is the part that touches raw personal content (emails, grades,
coursework text) — the highest-stakes data. Proposal generation is templated, lowest-stakes, and the
most expensive to run locally. Keeping the split this way preserves the strongest part of the privacy
story while removing the largest model from every user's device.

**Always-resident local footprint: roughly 1–2 GB.**

---

## 3. Ingestion from sources without APIs

### 3.1 When this happens

Homegrown department portals, professor course pages, club sites, JS-rendered event calendars with no
ICS export, login-gated sources, and any source whose structure silently changes over time.

### 3.2 Three-tier escalation, cheapest first

1. **Plain HTTP fetch + parse.** Many "no API" sites are server-rendered. No browser, no model.
2. **Headless browser render → script-cleaned DOM → small text model.** Strip nav, ads, banners,
   scripts before anything reaches a model, so it reads a short clean blob.
3. **Vision-based navigation.** Reserved for canvas-rendered or obfuscated pages. Rare fallback,
   highest cost.

Most sources resolve at tier 1 or 2.

### 3.3 Teach once, run deterministically forever

The critical pattern. The model + browser run **once, at source setup**, and their output is a small
human-readable extraction script committed as plain text in the repo — not a recurring inference
call.

- Every subsequent sync runs that script directly: no model, no browser, same cost as an API source.
- When a site changes and the script breaks, that is a **visible failure**, per the existing
  "failures are visible" invariant.
- Regeneration is filed as a **proposal to approve**, never a silent replacement — this is "judge
  once, re-propose freely" applied to scrapers.

### 3.4 Hiding it from the user

- User-facing action is "add a source" — a URL, or an embedded login webview where credentials go to
  the real site's form.
- One visible confirm step after setup: *"Here's what I found — is this what you want tracked?"*
  Doubles as ground truth validating the generated script.
- Credentials in the OS keychain, never in plaintext, never in anything a model prompt or log
  touches.
- **Prefer official OAuth wherever it exists** (Gmail, Google/Microsoft Calendar). Reserve raw
  credential handling for the genuine long tail.
- **MFA and anti-automation measures will defeat some sources.** The honest fallback is the
  paste/forward path, surfaced as a visible failure — not an attempt to defeat the protection.

### 3.5 Where bootstrapping runs

**Desktop-only launch means this can run locally at v1.** Desktop OSes permit persistent background
browser automation, so the cloud job can be deferred rather than built for launch. Ship the browser
engine as an **on-demand install** (~150–300 MB per platform, several hundred MB to ~1 GB RAM while
running), downloaded only the first time a user adds a source that needs rendering — users connecting
only API sources never pay it. Launch the process per-sync and close it fully; ingestion is a
background job, so ephemeral launch costs nothing in responsiveness.

**Move it to a cloud job when mobile ships.** Mobile OSes don't permit persistent background browser
automation, so this becomes a hard requirement at that point, not an optimization. Building the local
version first with a clean interface boundary makes that migration additive rather than a rewrite.

**Optional future lever:** extraction scripts (never data) for commonly-encountered site patterns
could be shared across users, cutting how often bootstrapping fires. Any imported script must be
treated as untrusted — sandboxed and explicitly approved before running.

---

## 4. Cross-device sync

The existing journalled-write invariant — record first, then single-line edit — is already a
sync-ready format. Sync replays an ordered append-only log; it does not need database replication.

- **Encrypt client-side before anything leaves the device.** Keys derived from the user's passphrase
  or device key, never touching our infrastructure. We become a blind relay that genuinely *cannot*
  read the data, not one that promises not to.
- **Device pairing over account systems.** QR scan or local-network handshake passes keys
  device-to-device. We never hold readable data, so we cannot be compelled to produce it.
- **Consider not running a sync server at all for most users.** Since the source of truth is plain
  text files, syncing through storage the user already trusts (iCloud Drive, Dropbox, self-hosted
  WebDAV/git) is more aligned with the philosophy, and is zero infrastructure and zero liability for
  us. An encrypted relay remains available for users who don't want to think about it.
- **Conflicts surface as proposals, not silent merges.** Edits are rare (humans are the primary write
  path), so this is mostly ordered appends with logical clocks.
- **Keep sync entirely separate from model distribution.** Sync carries tiny text; models are
  multi-GB binaries from a CDN.

---

## 5. Size budget

### 5.1 Current repo (measured)

| Component | Size |
|---|---|
| `docs/` | 2.4 MB |
| `intake/` | 2.0 MB |
| `.obsidian/` | ~2.1 MB |
| `.git` history | 3.4 MB |
| `tests/` | 716 KB |
| `tasks/` | 592 KB |
| `engine/`, `state/` | 316 KB each |
| **Total** | **~13 MB** (9.6 MB working + 3.4 MB git) |

411 tracked files — 264 markdown, 73 Python, remainder JSON/YAML/PDF/JS/CSS.

**Two findings:**

1. The three largest files are vendored Obsidian plugin bundles (~2.1 MB combined) — third-party
   compiled code that shouldn't be version-controlled. Add `.obsidian/plugins/` to `.gitignore`.
2. **`intake/documents/` contains real syllabus PDFs** with course codes and section numbers. The
   repo is now public. Verify whether these contain identifying information; if so, strip them with
   `git filter-repo` or BFG — a plain delete only removes them going forward, not from history.

Note that repo size ≠ shipped size. Almost none of this ships: docs, tests, fixtures, and git history
are development scaffolding.

### 5.2 Shipped app size (Rust + Tauri)

| Component | Size |
|---|---|
| Tauri shell (Rust; uses OS native webview, no bundled Chromium) | ~5–10 MB |
| UI assets | ~2–5 MB |
| Deterministic engine (compiled) | ~1–5 MB |
| Sync + API client | <1 MB |
| SQLite / vector store library | ~1–2 MB |
| **Base app, cloud-assisted AI** | **~10–30 MB** |
| *(add llama.cpp runtime binary if shipping local inference)* | *+20–50 MB* |

**Do not use Electron** — it bundles Chromium and Node for ~150–200 MB before your app does
anything.

**The Python decision matters.** Shipping the current Python engine standalone requires a bundled
interpreter, adding 20–50 MB. The engine's logic (parsing, scoring, matching) ports cleanly to Rust
and gets both speed and size for free.

### 5.3 Further size reduction

- **Cargo release profile:** `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`,
  `strip = true`. Routinely cuts a naive release build 30–50%.
- **Audit with `cargo-bloat`** before assuming anything — one or two crates often dominate. Prefer
  `ureq` over `reqwest` for simple HTTP; `rustls` over OpenSSL.
- **UI:** purge unused CSS, tree-shake JS, prefer Svelte (compiles away its runtime) over React,
  system fonts over bundled font files, inline SVG over icon-font sets.
- **Per-platform builds** should compile only that platform's inference backend if local inference
  ships at all.
- **Ship build artifacts only** — no docs, tests, or vault content.

### 5.4 Thresholds

- **Desktop has no hard cliff.** Multi-GB desktop installs are normal; no app-store size limit binds
  at launch.
- **Mobile cliffs apply later:** Google Play warns over 100 MB on cellular; Apple blocks over 200 MB
  without explicit override. Worth designing to now so the mobile port isn't a size crisis.
- **Below any cliff it's a conversion curve** — every extra MB costs some fraction of users at the
  download prompt.

**Target: under 50 MB base app. Under ~20 MB is the frictionless zone.** Both are realistic on the
Rust + Tauri path.

Note the tension with §3.5: the on-demand browser engine (~150–300 MB) exceeds these targets. That's
why it must stay a separate post-install download rather than part of the installer.

---

## 6. Platform strategy

**Decision: desktop only at launch.**

### 6.1 What this simplifies

- Browser automation runs locally — the cloud bootstrapping job is deferred (see §3.5).
- No app-store size cliffs, no store review cycles, no store revenue cut at launch.
- Full local model capability available; no hardware-tier fallbacks needed for phone-class RAM.
- One UI to build and one set of platform bugs, which matters for a solo/small team.

### 6.2 Build for the port anyway

- **One portable core, thin native UI.** Deterministic engine, journal, sync protocol in one shared
  Rust codebase (compiles to iOS/Android targets natively). Later platforms become mostly UI work.
- **Keep the scraper behind an interface boundary** so it can move server-side without a rewrite.
- **Mobile becomes viable** once generation is cloud-side and scraping is a server job — those are
  the only two blockers. A phone would ship: Rust engine, native UI, sync client, optionally a
  ~0.3 GB embedding model.
- **Not every device needs every capability.** Mobile reading the journal while desktop does
  ingestion is a legitimate architecture, not a degraded one.

### 6.3 Risk worth tracking

Students are phone-first for most daily tooling, and the product's core promise is a *morning
check-in*. Desktop-only is the right call for build simplicity, but watch during the pilot whether
people actually open a laptop for this or whether the ritual dies for lack of a phone surface. If
pilot retention is weak, absence of mobile is the first hypothesis to test — not the ranking quality.

---

## 7. Telemetry and product insight

### 7.1 The tension, stated plainly

"Collect as much as possible, comprehensive and plentiful" is close to the opposite of the
local-first, privacy-forward position that is the product's actual differentiator. This is the thing
that undoes the trust position if it ever surfaces — through a privacy audit, a user reading the
policy closely, or a breach.

**"Depersonalized" is harder than it sounds.** Stripping a name still leaves a unique course
schedule, source set, and usage pattern — that combination fingerprints a person as effectively as a
name, especially in a small cohort. More comprehensive data means *higher* re-identification risk,
not lower. Current privacy law (GDPR, and the direction CCPA and similar laws are moving) codifies
**data minimization** as a legal principle, not a best practice.

### 7.2 What was actually wanted is achievable without raw content

The stated goals — most-built preferences, most-common issues, emergent behaviors across many users —
are all answerable from structured usage telemetry computed **on-device**, transmitting only derived
metrics. This is standard product analytics practice, and structured events are more useful for
behavioral pattern-mining than raw content would be anyway.

**Collect:**

- Feature usage frequency and sequence; session length; drop-off points
- **Proposal acceptance/rejection rates** — directly measures whether the ranking and judgment logic
  works, across every user, without seeing proposal contents
- Source *type* connected as a category ("LMS," "personal email," "calendar") — never account details
  or content
- Error/failure rates per source type — tells you which scrapers break and where the rule-promotion
  loop needs work
- Ratio of deterministic resolution vs. model fallback — measures the "how small can we get" progress
  across the user base
- Notification open/dismiss/ignore rates — measures against the stated habituation anti-goal
- Aggregate slack trend — already one of the four "what working means" criteria
- Retention curves, feature adoption over time, hardware tier distribution

**Guardrails:** compute client-side, transmit only aggregates; enforce a minimum cohort size before
any slice is usable; present as an opt-in "help improve the app" toggle. A privacy-conscious audience
tends to trust disclosed opt-in telemetry more than silence.

---

## 8. Legal considerations

*Not legal advice. These are the specific places our design choices create questions worth bringing
to counsel — the first two most urgently.*

- **Scraping / browser automation.** Using a user's own credentials for their own personal data is on
  firmer ground than unauthorized access — US case law (*Van Buren*, *hiQ v. LinkedIn*) has trended
  toward ToS violation alone not constituting a computer-crime violation where credentials are valid.
  But this is actively contested, and ToS breach can still mean account termination or a
  cease-and-desist. **This is the single area most needing a real legal read**, given how central it
  is to the design.
- **Minors.** If a meaningful share of users are under 18 (high schoolers, dual-enrollment), state
  ed-tech privacy laws (e.g. California's SOPIPA) impose real obligations. Establish the actual age
  distribution before scaling.
- **Grades and coursework (FERPA).** FERPA primarily restricts *schools* and their agents, not a
  student voluntarily using a personal tool on their own data. This changes if we ever partner with
  institutions directly. Worth confirming rather than assuming.
- **Credential handling.** OAuth wherever it exists; keychain storage; never in model prompts or
  logs.
- **Subscription practices.** The FTC click-to-cancel rule and state auto-renewal laws require clear
  upfront pricing and cancellation as easy as signup. Here the compliance requirement and the trust
  positioning point the same direction — build it in cleanly.
- **Privacy policy and breach notification.** Required for any personal data processing, including
  data that only passes through transiently for cloud inference.

---

## 9. Pricing

### 9.1 Research findings that changed the recommendation

From RevenueCat's 2026 State of Subscription Apps (115,000 apps, $16B+ revenue, 1B+ transactions),
ChartMogul's 2026 conversion report, and current market data:

- **Cheaper does not convert better.** Median download-to-paid: low-priced apps 1.4%, mid-priced
  2.0%, high-priced 2.8% — roughly 2x from bottom to top tier. Pricing as low as possible can read as
  low-value.
- **Hard paywalls convert ~5x better than freemium** (10.7% vs 2.1% by day 35), with near-identical
  year-one retention — *but* freemium remains correct when free users drive word of mouth, which is
  our situation.
- **Longer trials convert better:** 17–32 day trials at 42.5% median vs 25.5% for trials under 4
  days. Education-category apps already trend long (80%+ run 5–9+ days) because value takes time to
  demonstrate.
- **Day 0 is decisive:** 55% of trial cancellations and 82% of trial starts happen the same day as
  install.
- **AI apps generate ~41% more revenue per customer but churn ~30% faster.** Retention, not
  acquisition, is likely the harder economic problem.
- **Trial model conversion rates** (ChartMogul, 200 products): opt-in trials 8.9%, opt-out/CC-required
  31.4%, freemium 5.6%. Opt-out converts better per signup but cuts signup volume 50–70%.

### 9.2 Why paid acquisition doesn't work at this price

Mobile CPI runs $1–5; at typical 2–5% conversion, real cost per paying subscriber is 20–50x CPI —
commonly $20–80. At $4.99/mo, a paid-acquired subscriber needs 4–16 months just to break even.

**This resolves the apparent tension between "maximize profit" and "reach the largest audience."**
Both point to the same answer: organic, word-of-mouth, campus-based growth isn't just cheaper, it's
close to the only channel that pencils out at this price.

### 9.3 Recommendation

| Decision | Recommendation |
|---|---|
| Price | **$4.99/month**, single flat price, no visible tiers |
| Annual | **~$39.99/year** (≈$3.33/mo effective), offered as an upgrade, not the headline |
| Model | **Opt-in free trial**, no credit card required |
| Trial length | **14–21 days** |
| Student tier | None needed — the whole product is the student tier |

Comparison anchors: Spotify Premium Student is $6.99/mo; ChatGPT Plus is $20/mo with no student
discount. $4.99 is clearly cheaper than both while staying out of the weakest-converting price tier.

Keep the local-vs-cloud model routing invisible to the user — pricing simplicity and technical
architecture reinforce each other.

**Design imperative from the Day 0 data:** the "the app already knows what I should work on today"
moment must land in the first session, not the first week.

---

## 10. Launch path and revenue projections

*Assumptions: $4.99/mo, Stripe-style fees (~2.9% + $0.30), AI cost $0.10–$0.50/user/month.*

### 10.1 A caveat on campus adoption data

Published campus-app adoption figures (98–100%) come almost entirely from official
university-branded apps distributed through orientation and administrative mandate — a fundamentally
different mechanism from an indie app spreading between friends. The closer comparable is Codecademy,
which grew from zero to a million users within about a year through organic word-of-mouth in college
communities: real, but gradual. Grassroots spread is slower and less certain than institution-backed
rollout.

### 10.2 Phase 1 — University of Alabama pilot

**UA enrolled 42,360 students for Fall 2025** (35,622 undergraduate, 6,279 graduate, 459
professional) — a large state school, near the top of the addressable-size range.

| Scenario | Share of student body | Users | Gross/mo |
|---|---|---|---|
| Weak | 0.1% | ~42 | ~$210 |
| Plausible | 0.5% | ~210 | ~$1,050 |
| Strong | 2% | ~850 | ~$4,240 |
| Exceptional | 5% | ~2,100 | ~$10,500 |

*These are reasoned planning ranges, not cited benchmarks — published data on unsponsored
single-campus adoption effectively doesn't exist. The 98–100% figures in campus-app case studies come
from administratively mandated university apps and do not apply.*

The pilot is not a revenue phase. It exists to validate whether the core loop (use → value → tell a
friend) holds.

**Advantages of UA specifically:** attending the campus means direct access to the LMS to build and
test the primary connector, real classmates for feedback, and native credibility for the "built for
students, by students" positioning. The one scraper that matters most at launch — UA's own systems —
is the one you can test daily.

**The metric that matters most:** the share of new signups arriving via referral rather than your
direct outreach. That is the actual viral coefficient. If it exceeds churn, growth compounds; if not,
rapid expansion won't happen regardless of product quality — better to learn that before committing
to expansion.

### 10.2b Phase 1b — University of Kentucky as the first expansion test

**UK enrolled 38,719 students for Fall 2025** (~28,000 undergraduate) — comparably sized to UA,
reached through existing personal contacts.

This is the single most informative expansion you can run, because it isolates the variable that
matters. UA growth is confounded by your physical presence; UK is not. It tests three things at once:

1. **Does the product travel?** A different LMS instance and different course structures will break
   assumptions the UA-tuned scrapers made.
2. **Does word-of-mouth work without you in the room?** Growth at UK is driven by a handful of
   contacts, not by you.
3. **Does the extraction-script-sharing idea (§3.5) pay off?** If UK runs comparable systems, scripts
   derived at UA may transfer directly.

Treat UK as an experiment with a hypothesis, not just a second market. If UK adoption per-contact is
far below UA's, the constraint is word-of-mouth mechanics. If adoption is fine but sources break, the
constraint is scraper portability — a much more tractable problem.

### 10.3 Phase 2 — Year 1 (UA + UK + 2–4 further campuses)

UA and UK together represent **~81,000 students**, which is a substantial base if penetration holds —
but penetration, not campus count, is the binding constraint.

| Scenario | Users | Gross/mo | Net after fees + AI |
|---|---|---|---|
| Conservative | 1,500 | $7,485 | ~$6,075–6,675 |
| Moderate | 5,000 | $24,950 | ~$20,250–22,250 |
| Optimistic | 15,000 | $74,850 | ~$60,750–66,750 |

The range is wide because it depends entirely on whether UA's referral rate survives the move to
campuses where you aren't physically present. The conservative case is lower than the previous draft
because desktop-only narrows the addressable share of a phone-first student population.

### 10.4 Phase 3 — steady state

Against ~19.6M US undergraduates. Campus-by-campus organic spread caps lower than a
nationally-marketed product, and the ~30% faster AI-app churn means steady state requires continuous
new adoption to offset losses.

| Penetration | Users | Gross/mo | Net after fees + AI |
|---|---|---|---|
| 0.05% | ~9,800 | $48,900 | ~$39,700–43,600 |
| 0.5% | ~98,000 | $489,000 | ~$396,900–436,100 |
| 2% | ~392,000 | $1,956,000 | ~$1,587,600–1,744,400 |

**All figures are contribution margin, not profit** — they exclude engineering time, support, and
marketing. Reaching these penetration levels realistically takes years.

Supporting context: 57% of students already report regular subscription spending, so the behavior
exists. A significant share also report real insecurity about affording college costs, which is why
the no-brainer price point matters.

**Revisit these projections with real pilot data as soon as a few weeks of it exists.** Every number
in this section is downstream of one figure not yet in hand: the actual organic referral rate.

---

## 11. Open decisions

| Decision | Status |
|---|---|
| Rust rewrite of the engine | Recommended, not committed |
| Local extraction + cloud generation split | Recommended; alternative is fully local (larger footprint, stronger privacy story) or fully cloud (smallest app, weakest privacy story) |
| Cloud inference provider (zero-retention terms required) | Not selected |
| Sync approach: user's own cloud storage vs. our encrypted relay | Leaning toward user's own storage as default |
| Launch platform | **Decided: desktop only** |
| Pilot campus | **Decided: University of Alabama; UK as first expansion test** |
| Mobile port timing | Not decided — revisit against pilot retention (see §6.3) |
| $4.99 price point | Recommended, not committed |
| Telemetry scope | Structured events recommended; final scope not set |
| Legal review of scraping and minors questions | Not started — highest priority external dependency |

---

## 12. Rulings taken on ingestion — 2026-09-01

**These post-date the body above. Where they conflict, these win.**

| # | Ruling | Effect on the body |
|---|---|---|
| 1 | **The Rust rewrite is COMMITTED**, not recommended (Quinn, 2026-09-01: *"I want to write this app in Rust"* → whole system). | §11 row 1 is settled. Spec: `docs/superpowers/specs/2026-09-01-rust-rewrite-design.md`. |
| 2 | **Design toward the app, build for personal use — but friends will test and demo soon, so onboarding and an account system are near-term, not deferred.** | Amends §6. Multi-user arrives before the pilot, not at it. |
| 3 | **The Claude Code cloud routine is a dead end.** It is a personal-deployment artefact with no place in a product; a local scheduler plus a cloud inference API replaces it. | Removes the "can Rust run in the Claude sandbox?" blocker from the Rust spec §3 entirely. |
| 4 | **Next build is the Rust port to parity**, waves 0–7, using the golden-`today.md` oracle. S2/S3 resume after cutover. | Sequencing. |
| 5 | **§1's "AI never produces anything the user sends externally" WINS over VISION.md's People domain.** Email drafting is cut; People becomes reminders and meeting proposals only. | Settles a direct contradiction. `docs/drafts/2026-08-20-course-emails.md` stays a one-off artefact; no producer replaces it. |
| 6 | **"Account system" means LOCAL PROFILES** — per-user vault, settings and credential store, plus onboarding — **not a hosted auth backend.** Hosted accounts remain an open decision, needed only for cross-device sync and billing. | Interprets ruling 2 consistently with §4's "device pairing over account systems". |

### Correction to §5.1 finding 2

**The repo is private, not public.** Verified 2026-09-01: an unauthenticated GitHub API probe of
`quinnhall07/quinn-ops` returns 404. Nothing is currently exposed.

The finding still stands as a **precondition**, and it understates the exposure. Going public would
leak, in git *history* as well as the working tree:

- `config/ingest.yaml:1` — the Blackboard feed URL and its bearer-style token. `CLAUDE.md` accepted
  this **explicitly on the grounds that the repo is private**.
- `config/ingest.yaml:104` — the secret Google Calendar ICS URL, a capability URL granting read
  access to Quinn's calendar to anyone holding it.
- `intake/documents/` — 10 tracked syllabus PDFs.
- The vault itself: `tasks/`, `courses/`, grades — the whole academic record.

`config/events.yaml`'s URLs are public campus endpoints and are not secrets.

**Order of operations if this repo is ever made public: rotate both credentials first, then rewrite
history with `git filter-repo`, then split the vault out of the code repo.** Doing it in any other
order publishes a live token. The `.obsidian/plugins/` finding is correct and cheap — do it whenever.
