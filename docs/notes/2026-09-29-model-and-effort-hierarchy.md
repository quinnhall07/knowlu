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

   Side lane: Haiku 4.5 / Sonnet Low subagents for read-only exploration
```

## Why

- Opus 5.5 costs $4 / $20 per MTok, Sonnet 5.5 $2 / $10, Haiku 4.5 $1 / $5, Fable 5.1 $10 / $50.
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
  deny edits to `engine/tests/fixtures/**` (rule 2, deterministically) and a hand-run `cargo tauri build`
  (unsupported per CLAUDE.md), and allow the read-only git and routine cargo commands.
- `.claude/agents/`: nine roles, listed with the routing rubric for undefined roles in `CLAUDE.md`
  ("Model and effort"), which loads every session. Each `description:` says when to use it and when not.

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
- The permission rules take effect: an edit under `engine/tests/fixtures/**` is refused. The deny also
  blocks the one legitimate case, regenerating `surface-today-*.json` (console spec 4.6); do that from a
  shell (`cargo test` with the oracle's regenerate flow), not through an edit tool, in a commit that
  says why.
- Then run the same two or three real tasks (for example a `sync` change and a `surface` view) on Opus 5.5
  medium and Sonnet 5.5 medium and compare tokens, turns and test-gate passes. Revise the tiers from that.
