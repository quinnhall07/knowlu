// Builds the request the paid run sends, and estimates its cost, without sending anything.
//
// **Transport, verified by the controller's smoke call on 2026-09-23.** `typesafe/jev-1.13` is a
// *decisions* model, not a chat model: `POST https://openrouter.ai/api/v1/chat/completions` answers
// 400 "cannot be used with the chat/completions endpoint. Use the /api/alpha/decisions endpoint
// instead." The working call is `POST https://openrouter.ai/api/alpha/decisions` with
//
//   { model, state: <any JSON>, questions: { <key>: {type: "choice", instructions, criteria:
//     {<option>: <description>}} | {type: "noul", instructions: <a statement>} }, provider }
//
// and the reply is `{model, answers: {<key>: {type: "choice", choice, probabilities, confidence} |
// {type: "noul", noul}}, usage: {input_tokens, output_tokens, cost}, id, provider}`; errors come as
// `{error: {code, message}}`. The zero-retention provider block `{order: ["typesafe"],
// allow_fallbacks: false, zdr: true}` was accepted on that call, so it is exactly what `ZDR_ROUTE`
// sends — `require_parameters` (which `judge_openrouter.ts` pins for chat models) was not part of
// the verified call and is left out rather than guessed at.
//
// So Jev's native `{state, questions}` shape travels as-is; there is no system/user split and no
// JSON-in-a-prompt. Two identical calls gave slightly different probabilities (0.72 vs 0.67): the
// model is not deterministic, which the report weighs.
//
// **Criteria are written from the review rubric's own definitions** (the SDD code-reviewer
// template's Critical / Important / Minor headings, and the four dispositions procedure §1 names),
// never from any finding's text, so nothing in a question carries a label. `findLeaks` still guards
// the state: `buildDecisionsBody` throws if a severity word survives in it.
//
// The cost table is denominated in input tokens with output free (assessment §5). `estimateTokens`
// is a chars/4 heuristic — good enough to bound a fraction-of-a-cent experiment; the real run reads
// the cost from each reply's `usage.cost`.

import type { CorpusFinding, Disposition } from "./types.ts";
import { findLeaks } from "./severity.ts";

export const JEV_MODEL_ID = "typesafe/jev-1.13";
export const OPENROUTER_DECISIONS_URL = "https://openrouter.ai/api/alpha/decisions";
export const INPUT_COST_PER_MILLION_USD = 0.042; // output free — assessment §5

/** The zero-retention pin, exactly the provider block the verified smoke call carried. */
export const ZDR_ROUTE = {
  order: ["typesafe"],
  allow_fallbacks: false,
  zdr: true,
} as const;

export interface JevState {
  finding: string;
  file: string | null;
  context: string;
}

export interface ChoiceQuestion {
  type: "choice";
  instructions: string;
  criteria: Record<string, string>;
}

export interface NoulQuestion {
  type: "noul";
  instructions: string;
}

/** Severity options, from the review rubric (Critical / Important / Minor) and nothing else. */
export const SEVERITY_CRITERIA: Record<"critical" | "important" | "minor", string> = {
  critical:
    "Must be fixed: a bug, a security issue, a data-loss risk, or functionality that is broken as written.",
  important:
    "Should be fixed: an architecture problem, a missing feature, poor error handling, or a gap in the tests.",
  minor: "Nice to have: code style, an optimisation opportunity, or polish to documentation or comments.",
};

/** Disposition options, from the four rulings procedure §1 names. */
export const DISPOSITION_CRITERIA: Record<"fix" | "rule_against" | "hand_off" | "defer", string> = {
  fix: "Accept the finding and fix it in this round, before the work merges.",
  rule_against:
    "Rule against the finding: the current code or wording stays as it is, because the finding is " +
    "mistaken or its fix is not worth making.",
  hand_off: "Hand the finding off: it is real but lies outside this work's scope, so someone else takes it.",
  defer: "Defer the finding: it is real, but it is recorded as a follow-up for later rather than fixed now.",
};

/** Disposition option key → the corpus's disposition label. */
export const DISPOSITION_OPTION_TO_LABEL: Record<keyof typeof DISPOSITION_CRITERIA, Disposition> = {
  fix: "fixed",
  rule_against: "ruled_against",
  hand_off: "handed_off",
  defer: "deferred",
};

export const DECISION_QUESTIONS: {
  grade: ChoiceQuestion;
  blocks: NoulQuestion;
  disposition: ChoiceQuestion;
} = {
  grade: {
    type: "choice",
    instructions: "How severe is this code-review finding?",
    criteria: SEVERITY_CRITERIA,
  },
  blocks: {
    type: "noul",
    instructions: "This finding must be fixed before the branch merges.",
  },
  disposition: {
    type: "choice",
    instructions: "What should happen to this code-review finding?",
    criteria: DISPOSITION_CRITERIA,
  },
};

const FILE_PATTERN = /`([a-zA-Z0-9_./-]+\.[a-zA-Z]{1,10})(?::\d[\d-]*)?`/;

/** The first backtick-quoted file path the finding names, if any — procedure §Step 2's `file`. */
export function primaryFileFor(text: string): string | null {
  const m = text.match(FILE_PATTERN);
  return m ? m[1] : null;
}

const TASK_CONTEXT: Record<string, string> = {
  "task-1-review.md": "Task 1 — OAuth sign-up, POST /account/consent, the attestation off the trigger.",
  "task-2-review.md": "Task 2 — billing-checkout: promotion code + attestation gate.",
  "task-3-review.md": "Task 3 — the loopback listener, the PKCE pair, and google_sign_in.",
  "task-4-review.md": "Task 4 — the email path loses the password.",
  "task-5-review.md": "Task 5 — the account panel and the upgrade overlay.",
  "task-6-review.md": "Task 6 — Back always goes back, Next either moves or says what is missing.",
  "task-7-review.md": "Task 7 — site/privacy.html, account.rs, static_assets.rs, CLAUDE.md H3.",
  "final-review.md": "The C1b whole-branch final review, after all seven tasks landed.",
  "2026-09-17-c1b-sign-in-plan-review.md": "The C1b sign-in plan review, before any task started.",
};

/** The task brief's one-line description (procedure §Step 2's `context`), from the finding's source. */
export function contextFor(source: string): string {
  const base = source.split(/[\\/]/).pop() ?? source;
  return TASK_CONTEXT[base] ?? `Source: ${base}`;
}

export function buildState(finding: CorpusFinding): JevState {
  return {
    finding: finding.text,
    file: primaryFileFor(finding.text),
    context: contextFor(finding.source),
  };
}

/**
 * The decisions-endpoint body (this file's header note): Jev's native `{state, questions}` shape
 * plus the model id and the zero-retention provider pin. Throws if the state still carries a
 * severity word — the leak check is the first stop rule and is enforced here, at the last moment
 * before anything could leave the machine.
 */
export function buildDecisionsBody(finding: CorpusFinding): {
  model: string;
  state: JevState;
  questions: typeof DECISION_QUESTIONS;
  provider: typeof ZDR_ROUTE;
} {
  const state = buildState(finding);
  // Scan the state's own strings, never its JSON: `JSON.stringify` writes a newline before "it" as
  // `\nit`, which `\bnit\b` matches — a false leak that stopped the first paid pass.
  const leaks = [state.finding, state.file ?? "", state.context].flatMap(findLeaks);
  if (leaks.length > 0) {
    throw new Error(`buildDecisionsBody: severity leak in ${finding.id}'s state: ${JSON.stringify(leaks)}`);
  }
  return { model: JEV_MODEL_ID, state, questions: DECISION_QUESTIONS, provider: ZDR_ROUTE };
}

/** chars/4, the common rough approximation for English prose — see this file's header note. */
export function estimateTokens(text: string): number {
  return Math.ceil(text.length / 4);
}

export interface CostEstimate {
  items: number;
  totalInputChars: number;
  estimatedInputTokens: number;
  estimatedCostUsd: number;
}

/** Estimates the cost of one pass over `findings`: the state plus every question's instructions and criteria. */
export function estimateCorpusCost(findings: CorpusFinding[]): CostEstimate {
  let totalChars = 0;
  const questionsText = JSON.stringify(DECISION_QUESTIONS);
  for (const f of findings) {
    totalChars += JSON.stringify(buildState(f)).length + questionsText.length;
  }
  const tokens = estimateTokens("x".repeat(totalChars));
  return {
    items: findings.length,
    totalInputChars: totalChars,
    estimatedInputTokens: tokens,
    estimatedCostUsd: (tokens / 1_000_000) * INPUT_COST_PER_MILLION_USD,
  };
}
