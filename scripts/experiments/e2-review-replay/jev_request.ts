// Builds the request the paid run would send, and estimates its cost, without sending anything.
//
// Two things this file is honest about rather than papering over:
//
// 1. **Transport is genuinely unverified.** Jev's native interface (`docs/reports/
//    2026-09-22-jev-system-one-assessment.md` line 65, 193-195) is `POST
//    https://api.typesafe.ai/v1/systemone` with body `{state, model, questions}`, answering
//    `{answers, usage}` — no system/user split, no JSON schema, nothing like the product's own
//    `judge_openrouter.ts` (`ModelRequest = {model, system, user, schema, maxTokens, sampling,
//    route}`). Going through OpenRouter (the decision this brief made, not TypeSafe direct) means
//    OpenRouter's normal contract — one OpenAI-compatible `/chat/completions` body, whatever the
//    underlying provider's native shape — applies, the same way `judge_openrouter.ts` calls every
//    other model. `buildOpenRouterChatBody` follows that contract: the native `{state, questions}`
//    intent travels as structured JSON inside the user message, not as top-level fields, because
//    nothing in the assessment confirms OpenRouter exposes Jev's native fields directly. This is a
//    real open question, not a detail — see the report's "before the paid run" note.
// 2. **The cost table is denominated in input tokens with output free** (assessment §5, procured
//    from docs.typesafe.ai/models). `estimateTokens` is a chars/4 heuristic, the common rough
//    approximation for English prose — good enough to bound a two-cent experiment, not a billing
//    reconciliation.

import type { CorpusFinding } from "./types.ts";

export const JEV_MODEL_ID = "typesafe/jev-1.13";
export const OPENROUTER_CHAT_URL = "https://openrouter.ai/api/v1/chat/completions";
export const INPUT_COST_PER_MILLION_USD = 0.042; // output free — assessment §5

/** The zero-retention pin, matching judge_openrouter.ts's `assertPinnedRoute` shape exactly. */
export const ZDR_ROUTE = {
  order: ["typesafe"],
  allow_fallbacks: false,
  zdr: true,
  require_parameters: true,
} as const;

export interface JevState {
  finding: string;
  file: string | null;
  context: string;
}

export type JevQuestion =
  | { type: "choice"; options: string[]; prompt: string }
  | { type: "noul"; prompt: string };

export const QUESTIONS: Record<"grade" | "blocks" | "disposition", JevQuestion> = {
  grade: {
    type: "choice",
    options: ["critical", "important", "minor"],
    prompt: "How severe is this code-review finding?",
  },
  blocks: {
    type: "noul",
    prompt: "This finding must be fixed before the branch merges.",
  },
  disposition: {
    type: "choice",
    options: ["fix now", "rule against", "hand off", "defer"],
    prompt: "What should happen to this finding?",
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

/** The intent the procedure's own pseudocode (§Step 2) specifies — one request, one price. */
export function buildJevNativeBody(finding: CorpusFinding): Record<string, unknown> {
  return { state: buildState(finding), model: JEV_MODEL_ID, questions: QUESTIONS };
}

function renderQuestionsAsPrompt(): string {
  const lines: string[] = [];
  for (const [key, q] of Object.entries(QUESTIONS)) {
    if (q.type === "choice") {
      lines.push(`${key} (choice, one of ${JSON.stringify(q.options)}): ${q.prompt}`);
    } else {
      lines.push(`${key} (noul, a calibrated probability 0-1): ${q.prompt}`);
    }
  }
  return lines.join("\n");
}

/**
 * The transport-level body for OpenRouter's `/chat/completions`, per this file's header note: the
 * native `state`/`questions` intent travels as JSON inside the user message, and the reply is
 * asked to come back as one JSON object keyed by question name. Unverified against a live call —
 * the report says so.
 */
export function buildOpenRouterChatBody(finding: CorpusFinding): Record<string, unknown> {
  const state = buildState(finding);
  const userContent = [
    "You are triaging one code-review finding. Answer strictly as JSON with exactly these keys: " +
    "grade, blocks, disposition. Do not include any other text.",
    "",
    "STATE:",
    JSON.stringify(state, null, 2),
    "",
    "QUESTIONS:",
    renderQuestionsAsPrompt(),
  ].join("\n");
  return {
    model: JEV_MODEL_ID,
    messages: [{ role: "user", content: userContent }],
    provider: ZDR_ROUTE,
  };
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

/** Estimates the cost of one pass over `findings`, minimal-state shape (finding + file + context + questions). */
export function estimateCorpusCost(findings: CorpusFinding[]): CostEstimate {
  let totalChars = 0;
  for (const f of findings) {
    const state = buildState(f);
    const stateText = JSON.stringify(state);
    const questionsText = renderQuestionsAsPrompt();
    totalChars += stateText.length + questionsText.length;
  }
  const tokens = estimateTokens("x".repeat(totalChars));
  return {
    items: findings.length,
    totalInputChars: totalChars,
    estimatedInputTokens: tokens,
    estimatedCostUsd: (tokens / 1_000_000) * INPUT_COST_PER_MILLION_USD,
  };
}
