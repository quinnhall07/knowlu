# Jev (System One) for Knowlu's development workflow

**Question (Quinn, 2026-09-22):** can the new Jev "System One" model speed up how we build Knowlu —
"I use Claude Code primarily, but can we somehow use it to help write code? Or other use cases
throughout this project and my workflow."

**Scope:** the development workflow only. The product question — Jev as the cloud judgment model —
is a separate assessment landing beside this one as
`docs/reports/2026-09-22-jev-system-one-assessment.md`. Nothing here is about the product.

**Consulted:** TypeSafe's docs and blog, Wikipedia, Simon Willison, Firecrawl, OpenRouter, Vercel's
AI SDK page, the Claude Code documentation, Anthropic's pricing page, GitHub's Actions billing page;
and in this repository `CLAUDE.md`, `HANDOFF.md`, `.github/workflows/ci.yml` and the C1b SDD ledger
with its briefs and reviews. Every fact carries a source below; anything unverified is marked.

## Verdict

Jev cannot write code, and it cannot be plugged into Claude Code as a model — not because of a
configuration gap but because it emits no text at all. TypeSafe's own description is "unstructured
state in, typed probabilistic decisions out": it answers yes/no, multiple-choice and score questions
with probabilities, and Firecrawl's write-up states plainly that it "cannot produce free-form prose,
code, or explanations." There is therefore no SWE-bench, Aider-polyglot or Terminal-Bench number to
compare against Sonnet or Opus, and no version of "route Claude Code to Jev" that exists —
independently, Anthropic's gateway documentation says it "doesn't support routing Claude Code to
non-Claude models through any gateway." What Jev is is a very fast, very cheap classifier that can
sit *beside* Claude Code as a decision layer — a `PreToolUse` gate on risky commands, a triage
sorter, a relevance filter — at 70–500 ms and $0.042 per million input tokens with output free. For
this project that is a safety-and-hygiene tool worth one or two cheap experiments, not a speed-up:
the 28 model dispatches that built C1b were writing Rust, TypeScript, SQL and prose, and Jev can take
none of it. Its documented weak spots — counting, date comparison, multi-hop reasoning, large noisy
state — are precisely the shapes this repository's rules take (command recounts, the 15-a-day
approvals cap, the 0-warnings tally, a frozen fixture's expiry date), so it is also a poor fit for
the checks we would most like to automate. Run experiment 1 below (an offline replay against the C1b
review corpus, no settings change, costs cents) and keep the rest on the shelf.

## What it is, for a developer

- **Maker and release.** TypeSafe AI (San Francisco, founded 2024; founder Diogo Almeida, formerly
  of OpenAI). Jev was announced in September 2026 and released in limited early access — Wikipedia
  dates it 2026-09-15, TypeSafe's blog post is dated 2026-09-22 and says "available today in early
  access". It shipped alongside a $40M seed round led by DCVC.
- **The category.** TypeSafe coined "System One model" for it: a model that "gives up string
  generation" and is "optimized for structured outputs". "System One" is not a fast mode of a
  chat model — it is the whole product. There is no System Two sibling that writes text.
- **Output.** Three primitives: **Choice** (pick one of up to 255 options, with a probability per
  option and a confidence score), **Score** (a rating against an ordered rubric) and **Noul** (a
  yes/no probability in 0..1). All questions in a request are evaluated in parallel against one
  shared `state`. Willison's summary: it "returns floating-point numbers rather than text", and
  gives no explanation for any decision.
- **Licence and versions.** Proprietary, API-only; no weights and no training recipe published, and
  Wikipedia records training exclusively on synthetic data. One public model, `jev-1.13.0`, with
  `jev-latest` and `jev-preview` aliases; no size variants documented.
- **Context.** 64k tokens total per request (state plus all questions); 32k for state plus the
  longest single question. Text only — images, audio and video must be preprocessed into text.
  OpenRouter's listing shows "32K context" for its copy of the model.
- **Speed and price.** 70–500 ms end to end; $0.042 per million input tokens, output free. TypeSafe
  claims 40–200x faster and 40–400x cheaper than frontier LLMs, peaking at "193.6x faster, 444.6x
  cheaper" on its own workflows — self-tested, and Wikipedia notes the acknowledged bias. An
  independent test measured 777 judgments in 0.7 s for about $0.0025.
- **Tool use / function calling.** Not documented, and not meaningful: there is no text channel in
  which a tool call could be expressed. The integration model is the reverse — your code calls Jev.
- **Coding evidence.** None of the usual kind, and none is possible. The one measured coding-adjacent
  result is `pi-warden`, a guardrail for the Pi coding agent: Jev judged tool calls "irreversible" /
  "off-task" and held destructive calls 42 times across 17,000 recorded calls, at a stated 88%
  accuracy. That is judgment *about* code, not the writing of it.
- **Documented failure modes** (TypeSafe's own "jaggedness" page, 2026-09-16): literal reading of the
  question as written; math and counting (it pattern-matches rather than tallies); date comparison
  (dates read as text, not ordered values); multi-hop indirection; large noisy state; and adversarial
  content, where injected instructions can shift the answer.

## Where it runs

- **First-party API.** `docs.typesafe.ai` — one model, `jev-1.13.0`, at $0.042/MTok input, output
  free. Rate limits 250,000 tokens/second and 1,200 requests/minute, stated to adjust dynamically;
  over either, 429. Access was waitlisted at launch (hours to a day or more for approval).
- **Vercel AI Gateway.** Available as `typesafe-ai/jev` through the AI SDK's `evaluate` method,
  added 2026-09-16, with no waitlist; there is a first-party AI SDK provider page for TypeSafe.
- **OpenRouter.** Serves "TypeSafe: Jev 1.13" at $0.042/M input, $0/M output, listed at 32K context.
  Not found on Together, Fireworks, Groq, Bedrock or Cloudflare as a standalone provider.
- **Retention.** TypeSafe states Jev "is not trained on customer requests or responses" and is not
  fine-tuned or LoRA-adapted with customer data, because the same weights serve every account. Zero
  data retention is offered **to enterprise customers under a data processing agreement**; a
  secondary summary notes the corollary that without ZDR, requests may be kept for operations, abuse
  prevention and debugging — unverified, but prudent to assume.
- **Local inference on a Windows laptop.** Not possible with Jev itself: no weights, no recipe. Open
  reimplementations appeared within days; the most developed is **Kev** — 0.8B / 4B / 9B decision
  models on Qwen3.5 bases whose local server implements the same request format down to field names,
  so TypeSafe's own SDK works against it unmodified. Kev-9B trails Jev by about 4.5 points on its own
  development set and trades wins on third-party sets. Documented runtimes are CUDA or Apple Silicon
  (bf16), at roughly 2 s per five-question request on an M5 (an earlier Qwen3 generation: about
  300 ms). Windows laptop viability is **unverified** — no source measures it, and this machine is
  set up for Rust, not a CUDA inference stack.

## Routes into this workflow

### (a) Claude Code itself

**Nothing to route.** Claude Code's model selection — `--model`, `/model`, `ANTHROPIC_MODEL`, the
`model` key in settings, and the `model:` frontmatter of a subagent in `.claude/agents/*.md` —
selects among Claude models, with non-Anthropic values covering only Claude *served by* Bedrock (an
inference profile ARN), Microsoft Foundry (a deployment name) or Google Cloud's Agent Platform (a
version name). On gateways the documentation is explicit: "Any gateway that exposes a supported API
format works. Anthropic doesn't endorse, maintain, or audit third-party gateway products, and
doesn't support routing Claude Code to non-Claude models through any gateway." So a LiteLLM-style
translation layer is out of support even for text models — and for Jev the question does not arise:
no Messages-API response, no streaming text, no `tool_use` block, no prompt caching to preserve, and
nothing the permission system could be handed to approve. Writing code is not a classification task.

**What does exist is the hook surface.** Claude Code hooks can be `type: "http"`, posting the hook's
JSON to a URL and reading the same JSON output back; `PreToolUse` can return `permissionDecision` of
`"allow"` or `"deny"` with a reason, add `additionalContext`, or rewrite `updatedInput`;
`UserPromptSubmit` can deny a prompt or add context. That is exactly the shape a Jev call fits: state
in, one typed decision out, in 70–500 ms. TypeSafe ships a Claude Code plugin and an agent skill
(`/typesafe:typesafe-ai`) giving an agent the API's context, and third parties have built `jev-use`
(a Claude Code / Codex / Pi plugin routing `check` / `pick` / `rate` judgments at **p50 ~230 ms and
~$0.02 per 1,000 judgments**, with `escalate: true` handing anything needing words back to the LLM)
and skill-router hooks on `UserPromptSubmit`. All are third-party and unaudited except TypeSafe's own
skill; adopting any means editing `.claude/`, which is Quinn's decision, not an agent's.

### (b) As an SDD subagent role

**No role moves.** Every dispatch in the C1b ledger produced text: Rust, TypeScript, SQL, HTML,
markdown, or a review with numbered findings and a verdict. Jev can produce none of these, so the
cost/turn-count trade-off that makes a cheaper *text* model tempting (and that this project has seen
cost 2–3x the turns) never comes up. Jev is not a cheaper tier of the same ladder; it is a different
instrument.

The nearest real adjacency is **around** a dispatch, not inside it: a Choice could pre-sort a
review's findings into blocking / should-fix / nit, or a Noul could flag an implementer's claim as
unverified. Both are triage, both leave the ruling with the controller, and experiment 1 tests
exactly that offline against findings already ruled.

### (c) Other tools beside Claude Code

Aider, Cursor, Copilot, Continue, Cline, OpenCode and Codex all need a text model to write code, so
none of them can *use Jev instead of* a code model. What exists is the same decision-layer pattern
ported to each: an MCP server (`jevx-mcp`) exposing typed decisions to any MCP client including
Cursor and Cline; a Cursor CLI tool that scores captured tool output for keep/drop and re-injects the
keepers after a compaction; a skill-picker with hooks for Claude Code, Codex, Gemini CLI, Droid,
OpenCode and Amp; `pi-warden` for the Pi agent. A Cursor feature request for a Jev decision layer is
open, not shipped. So the division of labour is unchanged: Claude Code writes and reviews; Jev, if
adopted, answers bounded questions the harness asks about what Claude Code is doing. A second coding
tool would cost more than it saves: the SDD loop is built around one controller.

### (d) CI and review

The deterministic checks stay deterministic and need no model: `scripts/ci/eol-check.ps1`, the
0-warning grep over `test.log` (splitting accepted `.rsrc` lines, cargo tallies and "other"), the
SHA-pinned-actions test, and the `paths-ignore` filter added on 2026-09-21 (`1d06912`). That last one
matters here: the CI-minute problem Quinn hit this week was solved by a five-line YAML filter, and a
model — any model — would have been a worse answer. Windows runners bill $0.010 a minute to $0.002
for Linux 1-core, with
2,000 included minutes on Free and 3,000 on Pro, so the lever is fewer and shorter Windows jobs, not
smarter ones. Where Jev could help is **after** a red run: a Choice over a `test.log` tail (compile
error / assertion failure / flake / toolchain) and a Noul for "is this the known timing-sensitive
`app/tests/commands.rs` flake". A PR-review bot is weaker: Jev can rank a hunk risky but cannot say
why, and this project's reviews are arguments, not labels. Release notes are text, and out of scope.

### (e) Non-code work

Plan reviews, spec fidelity checks, research like this one and privacy-policy drafting are all prose
in and prose out; Jev produces none of it. The one useful non-code shape is a **checklist scorer**:
a plan section as state, one Noul per rubric item ("does this task name its files?", "does it carry
a test-first step?"), pre-scanning a 2,000-line plan before a reviewer sees it. Note the C1b
pre-flight scan already does the equivalent deterministically — it grepped all 25 consumed symbols
and found them present, which a probabilistic answer could not have established.

### (f) What must not move

- The controller's seat, every ruling, and every judgment about what to build — Jev has no words to
  argue with.
- Security reviews. Two of the C1b reviews carried a security lens on the most capable model; a model
  that returns a number and no reason cannot hold that seat.
- Anything holding a secret or a session: the session JWT, the service-role key, the signing key,
  Credential Manager entries. The standing rule is that implementers never hold a token, and a
  `PreToolUse` gate sees every command and tool input — a new egress path for exactly that material.
- The frozen references and byte contracts, compared byte for byte. Probability has no role there.

## Cost and time

**Method, stated plainly.** This repository records no token counts and no dollar figures — the SDD
ledger records dispatches, commits and test results, not usage. The only grounded quantity is the
*shape* of the spend, counted from the C1b ledger's `progress.md` (read 2026-09-22):

- **28 model-tagged dispatches** for one stream of 7 tasks, 2026-09-17 to 2026-09-22 — 19 tagged
  sonnet (17 plain, 2 "fresh sonnet") and 9 tagged opus (7 plain, 2 "opus, security lens"), with the
  controller itself on Claude Fable 5.1. By role: 1 plan review, 2 plan fix rounds, 7 implementers,
  8 task and whole-branch reviewers, 5 scoped re-reviews, 2 fix dispatches, 3 rounds after the
  final review.
- The countable inputs: 1,964 lines of task briefs (113 to 580 each), 618 lines of task reviews plus
  a 160-line final review, a 2,073-line plan and a 393-line spec, over a suite of 1,263 tests.
- Prices (Anthropic, 2026-09-22): Opus 5 $5/$25 per MTok, Opus 5.5 $4/$20, Sonnet 5 $2/$10, Haiku
  4.5 $1/$5, Fable 5.1 $10/$50; cache hits at 0.1x base input on most models. **Any monthly dollar
  total I could give would be invented** — the real number is in the Console usage view or the
  subscription, not in this repository. Treat "28 dispatches a stream, two-thirds sonnet" as the
  shape and read the bill for the size.

**What Jev would change.** Nothing on that bill, because none of those 28 dispatches is a decision
task. Jev's spend would be additive and tiny: at $0.042/MTok input with output free, a 5k-token state
costs $0.00021 a call, so 1,000 gate calls on states that size cost about $0.21; the measured figure
from a real Claude Code plugin is **~$0.02 per 1,000 judgments** at **p50 ~230 ms**; the same gate on
Haiku 4.5 would pay $1/MTok input plus $5/MTok output. So the trade is a few cents and a fifth of a
second per gated tool call against a classifier's error rate — and latency is the real budget, which
is why the shipped designs gate narrowly (destructive patterns first, one Jev call only when a regex
layer already fired) rather than on everything.

## Risks

- **Fit.** The jaggedness page names counting, date comparison, multi-hop indirection and large noisy
  state as failure modes. This repository's checks are mostly exactly those: the Tauri command
  recount, the 15-a-day approvals cap, the warning tally, and — found on main this week — a frozen
  fixture whose approval expired on 2026-09-20 against the real clock, which is a date comparison,
  Jev's worst documented category. Deterministic checks must stay deterministic.
- **No explanation.** Jev returns a number and no reason; a verdict that cannot be argued with cannot
  be reviewed or appealed. The best public accuracy figure for a coding gate is pi-warden's 88% over
  17,000 calls — one judgment in eight wrong, which is why it steers rather than blocks.
- **Data egress.** A hook sees commands, file paths, diffs and tool inputs, all leaving this machine
  to a third party whose zero-data-retention option is enterprise-only under a DPA. Our product rule
  is that portal credentials never leave the device; the dev machine deserves the same caution.
- **Adversarial content.** Injected instructions can shift Jev's answers. A gate that can be talked
  out of holding a destructive command is worse than no gate, because it reads as a safety net.
- **Quality on Rust / Tauri / Deno specifically.** No evidence exists, and none can: Jev writes no
  code in any language. The licence question for generated code likewise does not arise.
- **The repository's own rules.** Rule 1 forbids single-user assumptions, so an API key or a personal
  endpoint in a settings file would be a bug — a key must come from the environment. Anything under
  `.claude/` is Quinn's to change, and this report changes nothing.
- **Lock-in.** Low at the wire level (Kev reimplements the request format field for field, so an
  alternative backend is a URL change); higher at the tooling level, where every integration is a
  week-old third-party plugin. The model is in limited early access, one version, with rate limits
  documented as changing without notice and benchmarks published by its maker.

## Experiments, ranked

**1. Replay the C1b review corpus offline (do this one).** We already own a labelled dataset: eight
review files plus the ledger's rulings, roughly forty findings the controller graded blocking /
should-fix / nit (F1–F12 on the final review; C1–C6, I1–I8, M1–M10 on the plan review). *Set up:* an
API key from the environment, one throwaway script in the scratchpad, state = the finding's text plus
the file it names, a Choice over the three grades plus a Noul for "would this block a merge".
*Measure:* agreement with the controller's ruling, cost, p50 latency. *Stop rule:* stop under 85%
agreement, or if anything ruled blocking comes back a nit — a triage that demotes a blocker is worse
than none. *Rules touched:* none; it changes no settings and writes no vault. The one thing to ask
Quinn first is that our own review prose leaves the machine.

**2. A shadow-mode PreToolUse gate for exactly one stream.** *Set up:* an `http` hook matched to
`Bash` only, log-only (it may return `additionalContext`, never `deny`), redacting environment values
and refusing to send anything matching a credential name, in a worktree's settings for one stream.
*Measure:* how many calls it would have held against the incidents the ledger records; the false-hold
rate; the added p50 per call. *Stop rule:* remove it if, in 500 calls, it flags nothing a human would
also have flagged, or if more than 5% of holds are wrong. *Rule it asks Quinn to change:* it edits a
Claude Code settings file and sends tool inputs off the machine without a ZDR agreement — Quinn's
calls, not an agent's.

**3. Red-CI triage.** *Set up:* on a failed run, the tail of `test.log` as state and one Choice over
{compile error, assertion failure, known flake, toolchain or environment}. *Measure:* against the red
runs the ledgers describe. *Stop rule:* drop it if it cannot separate the known timing-sensitive
`app/tests/commands.rs` flake from a real regression. The warning tally stays a grep — that is
arithmetic, and arithmetic is the model's documented weakness.

**4. Wait.** Entirely defensible: the model is days old, in limited early access, one version, with
self-published benchmarks, and nothing in the current stream is blocked on it. Re-check at general
availability or when an independent coding-gate evaluation is published.

## Sources

All read 2026-09-22 unless noted.

- TypeSafe AI, "Introducing System One Models and Jev" (dated 2026-09-22):
  https://typesafe.ai/blog/introducing-system-one-models-and-jev ; docs — Models (pricing, context,
  rate limits, input types) https://docs.typesafe.ai/models ; Agent skill (the Claude Code plugin,
  `/typesafe:typesafe-ai`) https://docs.typesafe.ai/agent-skill
- Wikipedia, "Jev (AI model)" (release 2026-09-15, proprietary, synthetic training data, primitives,
  latency and cost claims) https://en.wikipedia.org/wiki/Jev_(AI_model) ; Simon Willison, 2026-09-21
  (what it does and does not do, no explanations, pricing) https://simonwillison.net/2026/Sep/21/jev/
- Firecrawl, "What Is Jev?" (cannot produce prose or code; pi-warden 42 holds over 17,000 calls at
  88%; the jaggedness page of 2026-09-16; Vercel AI Gateway 2026-09-16; Every's 777 judgments in
  0.7 s for about $0.0025): https://www.firecrawl.dev/blog/what-is-jev
- Integrations: pi-warden (regex layer, hold at irreversible 0.7 or above, steer not interrupt)
  https://github.com/badgerexplore/pi-warden ; jev-use (Claude Code / Codex / Pi; p50 ~230 ms,
  ~$0.02 per 1,000 judgments) https://github.com/shitianfang/jev-use ; skillpick (six agents)
  https://github.com/cnYui/skillpick ; jevx-mcp https://github.com/hanshs474/jevx-mcp ; Cursor CLI
  compaction scorer https://github.com/kleosr/cursor-clijev-compaction ; Cursor request (open)
  https://forum.cursor.com/t/add-jev-as-a-fast-decision-layer-in-cursor-agent/172585
- OpenRouter TypeSafe provider ($0.042/M in, $0/M out, 32K context)
  https://openrouter.ai/provider/typesafe ; Vercel AI SDK https://ai-sdk.dev/providers/ai-sdk-providers/typesafe-ai
- Kev (0.8B/4B/9B on Qwen3.5, same wire format, CUDA or Apple Silicon, about 2 s per five-question
  request on an M5; dated 2026-09-21)
  https://www.explainx.ai/blog/kev-open-source-jev-clone-qwen35-family-2026 ; retention corollary
  without a ZDR agreement (secondary, unverified) https://jevaiguide.com/faq/does-jev-train-on-your-data/
- Claude Code: gateways ("doesn't support routing Claude Code to non-Claude models through any
  gateway") https://code.claude.com/docs/en/llm-gateway ; model selection (`--model`, `/model`,
  `ANTHROPIC_MODEL`, settings `model`, subagent `model:` frontmatter)
  https://code.claude.com/docs/en/model-config ; hooks (`type: "http"`, `PreToolUse`,
  `permissionDecision`) https://code.claude.com/docs/en/hooks
- Anthropic model pricing https://platform.claude.com/docs/en/about-claude/pricing ; GitHub Actions
  billing (Windows 2-core $0.010/min, Linux 1-core $0.002/min, 2,000 / 3,000 included minutes)
  https://docs.github.com/en/billing/managing-billing-for-your-products/about-billing-for-github-actions
- In this repository: `CLAUDE.md`; `HANDOFF.md` sections 1 to 3; `.github/workflows/ci.yml`; the C1b
  SDD ledger with its seven briefs and eight review files, in the `c1b-sign-in` worktree under
  `.superpowers/sdd/2026-09-17-c1b-sign-in-plan/`.
