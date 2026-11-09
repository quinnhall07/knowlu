# Model and effort review, 2026-11-09

Status: adopted 2026-11-09 on Quinn's word, with the goal stated as performance first: better code,
smarter decisions, a more helpful development partner, not fewer tokens. Supersedes
`2026-09-29-model-and-effort-hierarchy.md`.

Benchmark figures were fetched on 2026-11-09 from Anthropic's model pages and third-party leaderboards;
re-check before quoting. Usage figures come from this repository's own Claude Code transcripts,
2026-10-03 to 2026-11-09, priced at list API rates. They are estimates.

## The map

| Role | Before | Now |
|---|---|---|
| main session | Opus 5.5, medium | Fable 5.2, high |
| `planner`, `debugger` | Opus 5.5, high | Fable 5.2, high |
| `implementer` | Sonnet 5.5, high | Opus 5.5, high |
| `test-writer`, `console-ui`, `researcher` | Sonnet 5.5, medium | Opus 5.5, high |
| `explorer` | Haiku 5.5, low | Sonnet 5.5, medium |
| `mechanical` | Sonnet 5.5, low | Sonnet 5.5, medium |
| `reviewer`, `cloud-engineer`, `integrator` | Opus 5.5, high | unchanged |
| `contract-engineer`, `contract-reviewer` | Opus 5.5, xhigh | unchanged for now |
| `docs-keeper` | Sonnet 5.5, medium | unchanged |
| an agent with no model of its own | the session's model | Opus 5.5 |

## What the transcripts showed

- 1,097 subagent runs. About 90% of all use was Opus 5.5, under 10% Sonnet, almost none Haiku
  (`explorer` ran four times; the main thread did its own searching).
- About two-thirds of use was the main thread, `planner` and `reviewer`: orchestrating, planning and
  reviewing, not writing code. That is where a stronger model pays.
- The main thread ran mostly at `medium` (3,912 requests, against 2,149 at `high` and 561 at `xhigh`),
  because the project's `effortLevel` outranks a level saved with `/effort` (see "How it is enforced").
- Plan reviews failed about 70% of the time where the verdict was clear (40 FAIL, 17 PASS). Later rounds
  failed at the same rate (27 to 12) with no Critical findings and three to five Important ones, mostly
  "this step depends on something the code does not do". Diff reviews: 74 PASS, 28 FAIL. Contract
  reviews: 22 PASS, 7 FAIL. About half the review reports carried no clear verdict word.
- `planner` made about 3,000 Edit calls under the write-in-150-line-pieces rule, and one plan passed
  1,400 lines.
- `implementer` ran 110 times on Sonnet at `high` and 18 more times after escalation to Opus.
- The faults that reached Quinn in the first production install
  (`2026-10-22-first-install-feedback.md`) were product-behavior and production-configuration faults.
  A diff review against invariants does not catch those; a live run does.

## Why each change

- **Fable for the session, plans and root causes.** Fable 5.2 leads Opus 5.5 on the long, open-ended
  benchmarks: FrontierSWE v2 69.9 against 62.3, Remote Labor Index 41.3 against 21.3, WANDR 82.1 against
  72.3. The plan-review failures above are misses in reasoning about what the code already does, which
  is that kind of work. Anthropic's guidance for Fable is to start at `high`: gains above it are uneven,
  and at `xhigh` or `max` it starts subagents to review its own work. So `xhigh` is the step after two
  failed attempts, never the default.
- **Opus for specified code, tests, UI and research.** The evidence is mixed and this is a judgment.
  For: FrontierCode 1.1 has Opus at 54.4 to 54.6 across efforts and Sonnet at 52.1 (`xhigh`) falling to
  46.2 (`max`); CursorBench 4.0 has 57.8 against 55.5; and one implementer run in seven here ended on
  Opus anyway. Against: Anthropic's Terminal-Bench 4.0 figures put Sonnet (70.6) above Opus (66.4).
  Fable is not used here: it ties Opus on the Vals Terminal-Bench run (65.15 each) and trails it on
  Vals SWE-bench Multimodal (62.29 against 70.21), so specified work gains nothing from it.
- **Sonnet at `medium` for search and mechanical edits.** Haiku 5.5 scores 39.2 on Terminal-Bench 4.0
  and at `low` skips searches and stops early; Sonnet at `low` may skip verification.
- **The contract list stays on Opus at `xhigh` for now.** Byte-exact work is where Opus at `xhigh` is
  measured and where a change of model is the riskiest. Revisit after the two weeks below.

## How it is enforced

Checked against the Claude Code documentation for v2.1.321 on 2026-11-09.

- `.claude/settings.json`: `model: claude-fable-5-2`, `effortLevel: high`, `maxEffortLevel: xhigh`,
  `switchModelsOnFlag: false`, and `CLAUDE_CODE_SUBAGENT_MODEL=claude-opus-5-5` in `env`.
- A subagent's model resolves in this order: the Agent call's `model`, the agent file's `model:`, then
  `CLAUDE_CODE_SUBAGENT_MODEL`, then the session's model. The variable is a default since v2.1.251;
  before that it overrode every agent file. Never set `CLAUDE_CODE_SUBAGENT_MODEL_FORCE`, which brings
  the old behavior back. The built-in Explore agent runs on Opus when the session is on Fable.
- Effort resolves per model from the highest-precedence settings file that names one, and a project
  file outranks the user file. So this repository's `effortLevel` decides how a session starts, and a
  level saved with `/effort` does not carry into the next session here; `/effort` still works within
  one. Each agent runs at its own `effort:` (measured in the transcripts).
- `maxEffortLevel` is honored from project settings, and across files the lowest cap wins. This closes
  the first "to verify" item in the September note.
- `switchModelsOnFlag: false`: when a safety classifier flags a request the session pauses and asks,
  instead of carrying on with an older fallback model for the rest of the session.
- `reviewer` has Write so it can file its own report. `.claude/hooks/pretool-bash.js` refuses every
  reviewer Edit or Write except a new file named `YYYY-MM-DD-<name>.md` directly under `docs/reports/`
  of the project or of a worktree under its `.claude/worktrees/`; with those tools it cannot overwrite
  an earlier report or plant a `CLAUDE.md`. From a worktree kept anywhere else, it reports in its
  final message.
  The hook does not guard reviewer's Bash, which only its prompt limits to `git diff`, `log` and `show`.
  That gap is older than this change.
- The eight `Write(...)` deny rules for the frozen references are gone because they never matched:
  file permission checks read `Edit(path)` rules for every file-writing tool. The `Edit(...)` rules
  stay, and the hook still refuses the frozen references by every route.

## Limits

- The Max plans include Fable up to half the weekly limit. On this map the heaviest week in the record
  would have put between about two-fifths and two-thirds of its use on Fable, so a push week may run
  out. The order then: `planner` and `debugger` go to Opus at `xhigh` first (`CLAUDE.md` says how), the
  main session last.
- Past the allowance, an interactive session asks before it bills usage credits. A non-interactive one
  (`claude -p`, a background session) bills without asking when credits are enabled on the account.

## How to tell whether it worked

Count these for two weeks, then revisit the map:

1. Review rounds per plan (three or more was common).
2. The share of diff reviews that fail (28 of 102).
3. Faults found in live use after a release.

`reviewer` now writes `PASS` or `FAIL` on line 1 of every report so the first two can be counted.

## Not part of this change

The pipeline (how much process a change gets, a cap on plan-review rounds, a live run at the end of a
lane) and the hard caps in the agent prompts are Quinn's rules. They are proposed separately.
