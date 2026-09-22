# Jev beside Claude Code — the correction, the legal line, and what actually integrates

**Status: research note, 2026-09-22. Nothing installed, no `.claude/` file touched, no setting
changed.** This corrects and completes `docs/reports/2026-09-22-jev-system-one-for-development.md`,
which Quinn challenged the same day: "your original verdict was that Jev couldn't be used in
conjunction with Claude Code, but I know this is not true. People have chained together OpenCode, a
Claude Code subscription, and an API key for OpenRouter." Not legal advice.

## The correction, stated plainly

**Quinn's factual premise is right. The earlier report's verdict was too absolute, and its reasoning
was too narrow.** That report cited one sentence — Anthropic does not support routing Claude Code to
non-Claude models through any gateway — and generalised from it to "at most a classifier beside
Claude Code". It did name the hook surface, but it treated that as a footnote rather than as the
answer, and it never considered harness-chaining at all. People do chain harnesses, and there is a
whole ecosystem of shipped Jev integrations the report missed.

**But the conclusion moves for a different reason than the premise suggests, and one part of it
hardens rather than softens.** Three separate things were being run together:

| mechanism | status |
|---|---|
| Jev **as the model** Claude Code writes code with | does not exist, and cannot |
| A **third-party harness** driving a Claude subscription | **prohibited, and enforced — accounts were banned** |
| Jev as an **MCP server, a hook, or a proxy in front of the unmodified binary** | **permitted, and widely shipped** |

The third row is the answer, and it is a bigger answer than the earlier report gave. The second row
is the one Quinn's example names, and it is the one to stay away from.

## 1. Jev as a model: settled, and not a policy question

Jev is absent from OpenRouter's model listing. Its record says `"output_modalities":["decisions"]`,
`"has_text_output": false` and `"supported_parameters": []`. It is served through two dedicated
endpoints, `POST /api/v1/systemone` and `POST /api/alpha/decisions`, not through
`/chat/completions`. There is no chat parameter it accepts and no text channel in which a code edit
or a tool call could be expressed. **No harness can use it as a coding model, and this has nothing
to do with anyone's terms.** That part of the earlier report stands.

## 2. The chain Quinn named: real, and prohibited

OpenCode plus an OpenRouter key is ordinary and fine. **The Claude subscription is the part that is
not.** The record is unambiguous and it is public:

- **OpenCode removed it under legal pressure.** Issue #6930, "Using opencode with Anthropic OAuth
  violates ToS & Results in Ban", was closed on 2026-02-19 by a maintainer with: "**anthropic legal
  demanded we respond to this issue and close it** […] their ToS prohibits using your claude max
  subscription outside of claude code." A later community pull request to restore it was closed with
  "**We cannot legally support this.**" The removal deleted a file that had been named
  `anthropic_spoof.txt`.
- **Crush removed it too.** Pull request #1783, merged 2026-01-07, minus 1,078 lines: "This patch
  removes Claude Code support **following a request from Anthropic**, to align with their terms of
  service."
- **Accounts were banned, and Anthropic said so on the record.** A member of the Claude Code team,
  2026-01-09: "Yesterday we tightened our safeguards against spoofing the Claude Code harness
  **after accounts were banned** for triggering abuse filters from third-party harnesses using
  Claude subscriptions. […] Third-party harnesses using Claude subscriptions create problems for
  users and **are prohibited by our Terms of Service**." The head of Claude Code, 2026-04-03:
  "Starting tomorrow at 12pm PT, Claude subscriptions will no longer cover usage on third-party
  tools like OpenClaw."

**So the chain exists in the sense Quinn means — people built it — and it was dismantled by the
projects themselves after Anthropic's lawyers asked.** It is not a supported path and it is not one
to rebuild.

**It is also technically fragile, independently of the terms.** Subscription OAuth tokens are gated
on an undocumented fingerprint of the official client. The system prompt must begin with the exact
string the binary sends, or the API answers HTTP 400 with the message `"Error"`; by August that
failure had mutated into a headerless 429 indistinguishable from real quota exhaustion. A packet
capture in a public issue shows `claude-cli/2.1.2 (external, cli)` succeeding where an SDK
user-agent gets a 400 on an otherwise identical bearer token, and renaming a tool from `read_tool`
to `read` reproduced the failure. **Anything built on this breaks silently and looks like a quota
problem when it does.**

## 3. The line Anthropic actually draws, and it moved

The controlling document is Claude Code's legal and compliance page, under "Authentication and
credential use". The prohibition:

> "**Anthropic does not permit third-party developers to offer Claude.ai login into their own
> applications, or to route requests through Free, Pro, or Max plan credentials on behalf of their
> users.** Moreover, developers may not collect, store, or intermediate Claude.ai credentials or
> session tokens."

And the carve-out, which is the sentence that decides this whole question:

> "Nor does it prevent an end user from signing in to **the unmodified Claude Code binary** with
> their own Claude subscription, including where a platform hosts Claude Code."

> "**The Claude Code binary must not be modified.** Claude Code must be installed and run as
> published by Anthropic, and customers may not remove, disable, or restrict any authentication
> method built into it."

**This explicitly blesses the mechanism every shipped Jev-beside-Claude-Code integration uses**: run
the unmodified binary, signed in normally, with a hook, an MCP server, or a loopback proxy beside
it. The three-way split in the table above is not my inference. It is the line Anthropic writes.

**Two cautions about sources on this topic, both of which caught my earlier research.**

1. **The policy was softened in the last seven months and most secondary writing is stale.** An
   archived snapshot from 2026-02-21 carried a blanket sentence — using OAuth tokens from Free, Pro
   or Max accounts "in any other product, tool, or service — including the Agent SDK — is not
   permitted" — that is **gone** from the live page, along with the arrival of the unmodified-binary
   carve-out. News coverage quoted the February text accurately at the time; blogs still recycle it.
   Do not quote it for today's question.
2. **The prohibition is commonly misattributed.** It lives in the Consumer Terms' automated-access
   clause, which carries its own escape hatch, "or where we otherwise explicitly permit it" — which
   is exactly what the Claude Code legal page then does. **The Usage Policy says nothing about
   this**; it covers ban evasion, automated account creation, jailbreaking and distillation. Anyone
   citing the usage policy against third-party harnesses is wrong.

A separate, narrower path is also legitimate and is what the surviving integrations use: **shell out
to the official binary.** A Kilo Code pull request of 2026-07-31 says it plainly — an earlier
attempt to reuse the credentials directly "was confirmed dead — Anthropic rejects those credentials
outside the CLI" — and replaces it with `claude --print --input-format stream-json --output-format
stream-json`, "using documented flags only — **no traffic shaping, header/TLS fingerprinting, or
attempt to disguise the request shape**". The project declined a request for a stealth mode. Cline,
Goose and Zed sit in the same category. Whether that pull request shipped into Kilo's live product
is **unverified**; its user-facing docs still show no subscription path.

## 4. What is actually shipped

Every one of these was verified to exist. Stars and dates read 2026-09-22.

| project | what it does | surface | stars |
|---|---|---|---|
| fast-jev-compaction | lossless context compaction, never summarises | Claude Code hook on `session.compact` | **6,300** |
| typesafe-ai/skills | TypeSafe's own agent skill and plugin | `claude plugin marketplace add typesafe-ai/skills` | **1,800** |
| pi-jev | gate, output judge and an ask tool for the Pi agent | Pi extension | 139 |
| pi-warden | guardrails with a local regex prefilter | Pi extension | 134 |
| skillranker | ranks agent skills, **written in Rust** | managed Claude Code hook | 113 |
| pi-typesafe | shared client with daily spend caps | Pi extension | 41 |
| jev-use | routes no-text agent steps to Jev | Claude Code / Codex `PreToolUse` gate plus a skill | 19 |
| jev-mcp | typed decisions for any MCP client | MCP server | 20 |
| jev-commit | checks a commit message against its diff | git `commit-msg` hook | 10 |
| skillpick | picks the right skill per prompt | `UserPromptSubmit` hook | 2 |
| jevx-mcp | typed decisions over MCP | MCP server | 0 |
| cursor-clijev-compaction | scores captured tool output for keep or drop | Cursor CLI hooks | 1 |

Four design patterns recur across them and all four are worth stealing regardless of Jev:

- **A cheap local prefilter before any model call.** The Pi guardrail catches force-push, `rm -rf`
  and `DROP` with a regex and never calls the model for read-only operations; the Rust skill ranker
  uses BM25 to get below the 255-option ceiling before it asks anything.
- **Batch every question into one request.** TypeSafe's own cookbook claims parallel questions are
  "12.2x cheaper and 10.0x faster with no change in answers". One integration asks four questions in
  a single round trip at about 300 ms rather than four round trips.
- **Fail open on every error path.** The commit-message checker's own words: "Every error path fails
  open… exits 0 and lets the commit through."
- **Calibrate the threshold rather than defaulting to 0.5.** One project sets its destructive-action
  threshold at 0.90 rather than 0.70 specifically "because ordinary requested edits score ~0.85".
  That is the same lesson `docs/notes/2026-09-22-confidence-calibration-and-the-floor.md` reaches
  from the literature.

## 5. The cost caveat, which is the one that would have bitten us

A proxy in front of Claude Code sits inside the prompt cache, and Anthropic's own gateway
documentation names the failure mode: "**No error: the conversation bills as uncached input on every
turn**, visible as high `input_tokens` with little or no cache activity in `usage`." It adds that
the attribution block Claude Code prepends is stripped **positionally**, so "prepending another
system block, reordering the array, or converting it to a single string defeats the strip".

The most prominent Jev proxy acknowledges this and works around it by adding a hint to the request
rather than changing `tool_choice`. But a hint is still a prompt mutation. Its own README is honest
about the result: "Expect better tool picks on large tool lists, **not lower cost or latency**."

**So a Jev layer in front of Claude Code should be judged on decision quality, never on cost.** If
anyone quotes a saving, check `usage` for cache activity across turns before believing it.

## 6. What I would and would not wire into this project

**Not yet, and none of it without Quinn's word, because all of it edits `.claude/`.**

Worth considering, in order:

1. **An MCP server, if anything.** It is the least invasive surface, it does not touch
   authentication, it does not sit in the prompt cache, and it is opt-in per call. Two exist.
2. **Context compaction** is the single most-adopted use by a wide margin, and the one whose value
   does not depend on the classifier being right — a wrong keep costs tokens, not correctness.
3. **A `PreToolUse` gate on destructive commands only**, with a regex prefilter first, failing open,
   log-only to start. This was experiment 2 in the earlier report and the procedure in
   `docs/notes/2026-09-22-jev-review-replay-experiment.md` §4 is the stop rule it would need.

Against, and these have not changed:

- **Anything holding a secret or a session.** A `PreToolUse` gate sees every command and tool input.
  The standing rule is that implementers never hold a token, and this would be a new egress path for
  exactly that material.
- **Deterministic checks.** The command recount, the approvals cap, the warning tally and every
  frozen-fixture comparison are arithmetic, and arithmetic is the model's own documented weakness.
- **Security review and any ruling.** A model that returns a number and no reason cannot hold a seat
  where the output is an argument.

**And nothing here is a reason to revisit the product decision.** Jev's place, if it has one, is
beside the tools that build Knowlu, not inside Knowlu. The judgment-service verdict rests on
measurement now rather than on privacy, and
`docs/notes/2026-09-22-confidence-calibration-and-the-floor.md` is where that argument lives.

## 7. Sources

All read 2026-09-22.

**The legal position.** `code.claude.com/docs/en/legal-and-compliance`, the "Authentication and
credential use" section, for both the prohibition and the unmodified-binary carve-out; the Internet
Archive snapshot of the same page dated 2026-02-21 for the superseded wording;
`anthropic.com/legal/consumer-terms` (effective 2025-10-08) for the automated-access clause and its
"where we otherwise explicitly permit it" exception; Anthropic's usage policy, read in full, which
does not address this; `support.claude.com/en/articles/15036540` (updated 2026-06-16) for the paused
Agent SDK billing change; Claude Code's LLM gateway documentation for the cache-billing failure mode
and the positional attribution strip.

**Enforcement, on the record.** Posts by a Claude Code team member dated 2026-01-09 and by the head
of Claude Code dated 2026-04-03, read through the syndication API rather than a summariser. OpenCode
issue #6930 (closed 2026-02-19) and pull requests #18186 and #20080; Crush pull request #1783
(merged 2026-01-07); Kilo Code pull request #12736 (2026-07-31); Claude Code issues #40515 and
#87420 and OpenCode issue #7410 for the fingerprinting behaviour.

**The integrations.** Each repository was fetched directly; stars and last-commit dates are from the
repository pages. OpenRouter's model record for `typesafe/jev-1.13` and its `systemone` and
`decisions` endpoint documentation. TypeSafe's own parallel-questions cookbook for the batching
claim, which is the vendor's own number and is marked as such.

**Unverified, and recorded as such.** Whether the Kilo pull request shipped into the live product.
The exact date the legal page was softened. Whether the third-party extra-usage path mentioned in
April still exists. Individual ban anecdotes in the OpenCode thread carry no Anthropic email or
screenshot, and the same thread contains counter-examples of bans among people using only the
official client, several correlated with plan changes rather than the harness. **No Reddit or
YouTube evidence is in this note**: one research pass could not reach Reddit and exhausted its
search budget, and nothing was invented to fill the gap.
