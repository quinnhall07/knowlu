// The two arms of E1 and how a request is built for either. The request goes through the
// product's own OpenRouter body builder (`openRouterBody`), after the product's own pin check
// (`assertPinnedRoute`), so E1 sends what the service would send — the same model, route,
// sampling, max_tokens and strict schema — with only the arm's system prompt and schema swapped in.
import { assertPinnedRoute, openRouterBody } from "../../../cloud/supabase/functions/_shared/judge_openrouter.ts";
import type { ModelRequest } from "../../../cloud/supabase/functions/_shared/judge_anthropic.ts";
import type { ModelRow } from "../../../cloud/supabase/functions/_shared/judge_models.ts";
import {
  buildPrompt,
  promptHash,
  schemaFor,
  systemTemplate,
} from "../../../cloud/supabase/functions/_shared/judge_prompts.ts";
import { type Cause, validate } from "../../../cloud/supabase/functions/_shared/judge_validate.ts";
import { EVENT3_SCHEMA, EVENT3_SYSTEM, event3Hash, validateEvent3 } from "./event3_frozen.ts";

export type ArmName = "event-3" | "event-4";

export interface Interpreted {
  /** What the device would record: `null` when the product would have refused the answer. */
  verdict: Record<string, unknown> | null;
  cause?: Cause;
  /** event-4 only: which named drop rule fired, if any. */
  rule?: "audience" | "standing";
}

export interface Arm {
  name: ArmName;
  system: string;
  schema: Record<string, unknown>;
  promptHash: () => Promise<string>;
  interpret: (answer: Record<string, unknown>) => Interpreted;
  /** A rough output size in tokens, for the estimate only. */
  expectedOutputTokens: number;
}

export const ARMS: Record<ArmName, Arm> = {
  "event-3": {
    name: "event-3",
    system: EVENT3_SYSTEM,
    schema: EVENT3_SCHEMA,
    promptHash: event3Hash,
    interpret: (answer) => {
      const v = validateEvent3(answer);
      return v.ok && v.verdict ? { verdict: v.verdict } : { verdict: null, cause: v.cause };
    },
    expectedOutputTokens: 60,
  },
  "event-4": {
    name: "event-4",
    system: systemTemplate("event"),
    schema: schemaFor("event"),
    promptHash: () => promptHash("event"),
    interpret: (answer) => {
      const v = validate("event", answer, {});
      if (!v.ok || !v.verdict) return { verdict: null, cause: v.cause };
      const rule = answer.audience_excludes_student === true
        ? "audience"
        : answer.standing_or_drop_in === true
        ? "standing"
        : undefined;
      return rule === undefined ? { verdict: v.verdict } : { verdict: v.verdict, rule };
    },
    expectedOutputTokens: 75,
  },
};

export function modelRequest(
  arm: Arm,
  item: Record<string, unknown>,
  seed: Record<string, unknown>,
  row: ModelRow,
): ModelRequest {
  const { user } = buildPrompt("event", item, seed);
  return {
    model: row.model_id,
    system: arm.system,
    user,
    schema: arm.schema,
    maxTokens: row.max_tokens,
    sampling: row.sampling,
    route: row.route,
  };
}

/** The exact JSON body OpenRouter would receive, after the product's pin check. */
export function requestBody(
  arm: Arm,
  item: Record<string, unknown>,
  seed: Record<string, unknown>,
  row: ModelRow,
): Record<string, unknown> {
  assertPinnedRoute(row.route);
  return openRouterBody(modelRequest(arm, item, seed, row));
}

export interface CostEstimate {
  calls: number;
  inputTokens: number;
  outputTokens: number;
  usd: number;
}

/** A deliberately rough estimate: four characters a token over the whole request body (which
 *  counts the schema and the routing object as input — an over-estimate, on purpose), and each
 *  arm's typical reply size as output. Priced at the pinned row's own rates. */
export function estimateCost(
  cases: Array<{ item: Record<string, unknown>; seed: Record<string, unknown> }>,
  row: ModelRow,
): CostEstimate {
  let calls = 0, inputTokens = 0, outputTokens = 0;
  for (const arm of Object.values(ARMS)) {
    for (const c of cases) {
      const body = JSON.stringify(openRouterBody(modelRequest(arm, c.item, c.seed, row)));
      inputTokens += Math.ceil(body.length / 4);
      outputTokens += arm.expectedOutputTokens;
      calls++;
    }
  }
  return { calls, inputTokens, outputTokens, usd: (inputTokens * row.usd_per_m_in + outputTokens * row.usd_per_m_out) / 1e6 };
}
