# Model and effort hierarchy for Claude Code sessions

Status: proposed 2026-09-29, not yet measured on Knowlu work. Figures come from web-search summaries of
secondary sources (the original pages could not be fetched); re-check before quoting them.

## The hierarchy

```
                      Opus 5.5
                     Orchestrator
                   Medium (default)
                          |
        +-----------------+-----------------+
        v                 v                 v
   Opus 5.5 Medium   Opus 5.5 High     Sonnet 5.5 Low/Med
    everyday work     specs, plans,     mechanical edits
    commands, views,  cross-crate       with a clear spec
    scheduler, tests  refactors, review
        |                 |                 |
        +-----------------+-----------------+
                          v
                  Escalation layer
                  Opus 5.5 XHigh
        write/journal, yamlemit, sync, entitle,
        frozen fixtures
                          |
        (only if XHigh fails on a real bug)
                          v
              Opus 5.5 Max  ->  Fable 5.1
              one session, then drop back

   Side lane: Haiku 5.5 / Sonnet Low subagents for read-only exploration
```

## Why

- Opus 5.5 costs $4 / $20 per MTok, Sonnet 5.5 $2 / $10, Haiku 5.5 $0.10 / $0.50 (prompts up to 100K
  tokens; $0.50 / $2.50 above), Fable 5.1 $10 / $50.
- Haiku 5.5 (2026-10-18 check): Terminal-Bench 4.0 39.2% against Sonnet 5.5's 70.6%; Anthropic names it
  for "narrow tasks such as compaction, summarization, and subagent work" and Sonnet/Opus for complex
  agentic coding (anthropic.com/claude-haiku-5-5). So Haiku stays on read-only search (`explorer`); TDD,
  implementation and debugging stay on Sonnet or Opus.
- Artificial Analysis Intelligence Index: Opus 5.5 max 58, xhigh 56, high 54, medium 51, low 42; Sonnet 5.5
  max 56; Fable 5.1 53. Opus 5.5 cost per task: about $0.55 low, $1.34 medium, $5.98 max.
- Sonnet 5.5 used the most output tokens per task Artificial Analysis had measured (about 193K at max
  against about 119K for Opus 5.5), so its per-token discount does not reliably survive at high effort.
  Keep it at low or medium.
- Opus 5.5 matched or beat Fable 5.1 on the published coding benchmarks at about a third of the cost and
  half the time in one third-party test. Fable 5.1 is a last resort.
- Effort is the main cost lever. Judge cost per completed task, not per token.

## How it is enforced

- `.claude/settings.json`: default model `claude-opus-5-5` (pinned so a new Opus release does not change
  the default silently), `effortLevel: medium`, `maxEffortLevel: xhigh` (no accidental `max`). Permissions
  deny Edit/Write of the eight frozen references by name (rule 2), a hand-run `cargo tauri build`
  (unsupported per CLAUDE.md), `cargo test --release`, `git stash` and force pushes; they ask before any prod Supabase MCP call or `supabase` CLI command, and allow
  the read-only git and routine cargo commands. `subagentPromptCacheTtl: 1h` keeps a subagent's cache
  warm while it waits for a build slot.
- `.claude/hooks/pretool-bash.js` (PR #37, 2026-10-18): the frozen references are refused by every
  route; other `engine/tests/fixtures/**` paths are open only to contract-engineer and the main
  session; tag pushes, force pushes and GitHub release/secret/variable/ruleset/repo/workflow changes ask; researcher's shell
  is one read-only git or gh command; plain cargo commands are trimmed to failures and warnings.
- `.claude/agents/`: fourteen roles, listed with the routing rubric for undefined roles in `CLAUDE.md`
  ("Model and effort"), which loads every session. Each `description:` says when to use it and when not;
  the five that overlap a built-in or plugin agent (explorer, planner, console-ui, cloud-engineer,
  contract-reviewer) also say which one they replace. Models are pinned (`claude-opus-5-5`,
  `claude-sonnet-5-5`, `claude-haiku-5-5`). The user-level Opus, Sonnet and Haiku `modelSettings`
  entries were removed 2026-10-18 (only Fable's remains), so each agent's `effort:` is what runs.
- Open: this note says Sonnet stays at low or medium, but CLAUDE.md's table and `implementer.md` run
  implementer at Sonnet high. CLAUDE.md wins until Quinn rules on it.

Claude Code has no automatic task-based routing and hooks cannot change the model, so routing to a
subagent is Claude's choice from each `description:`, and the session default is the only hard setting.
Naming the agent in the prompt is the reliable way to force it.

## Suggestions not yet done

- Add a `PreToolUse` hook that blocks writes to the contract list from any agent other than the ones
  allowed, if the descriptions prove too soft in practice.
- Keep `CLAUDE.md` lean: it loads every session, and it is already long. Move stable reference material
  into `docs/` and link it.
- `/clear` between unrelated tasks instead of carrying a long context; use `/compact` mid-task.
- Log which subagent handled each task for a week (a line in the commit message or session notes) to see
  whether delegation is happening, then tune descriptions.

## To verify locally (not checked in the cloud session)

- `maxEffortLevel` in project settings actually caps `/effort max`.
- Whether `availableModels` with `enforceAvailableModels: true` from project settings blocks `fable`; if it
  is managed-settings only, leave it out. `deniedModels` is managed-settings only.
- The subagent `effort:` frontmatter field is honoured on your Claude Code version.
- Regenerating `surface-today-*.json` (console spec 4.6) is allowed to contract-engineer and the main
  session, through the oracle's regenerate flow, in a commit that says why.
- Then run the same two or three real tasks (for example a `sync` change and a `surface` view) on Opus 5.5
  medium and Sonnet 5.5 medium and compare tokens, turns and test-gate passes. Revise the tiers from that.
