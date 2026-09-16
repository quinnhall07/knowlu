// The OpenRouter adapter: one OpenAI-compatible POST per judgment, pinned to ONE named upstream
// with zero retention, reasoning disabled, the reply constrained to the kind's JSON schema.
//
// Why a router and not the upstream directly: OpenRouter enforces the zero-retention filter and
// the single-upstream pin per request (`provider.zdr`, `provider.order`, `allow_fallbacks: false`),
// so a policy change at the upstream fails the request instead of silently routing elsewhere. The
// upstream is named in the row's `route` and on the privacy page; OpenRouter's own policy is not to
// retain prompts unless logging is opted into (never on this account).
import { type JudgeModel, type ModelReply, type ModelRequest, ModelRefused } from "./judge_anthropic.ts";

export const OPENROUTER_URL = "https://openrouter.ai/api/v1/chat/completions";
export const CALL_TIMEOUT_MS = 120_000;

/** The zero-retention pin a route MUST carry before any request is built from it: one named
 *  upstream, no fallback, OpenRouter's zero-data-retention filter and strict parameters. A row
 *  whose `route` is `{}` (the column default) or missing entirely would otherwise send an
 *  unpinned request — any upstream, fallbacks on, no zero-retention filter — with nothing thrown
 *  or logged (whole-branch review I1). Throws a PLAIN `Error`, never `ModelRefused`: a refused
 *  route is not the model declining, and `judge_pipeline.ts` maps a plain `Error` whose message
 *  lacks `max_tokens` to `model failed` — a named low-confidence outcome, exit 0, fail closed. */
export function assertPinnedRoute(route: Record<string, unknown> | undefined): void {
  const order = route?.order;
  const pinned = route !== undefined &&
    Array.isArray(order) && order.length === 1 && typeof order[0] === "string" && order[0].length > 0 &&
    route.allow_fallbacks === false &&
    route.zdr === true &&
    route.require_parameters === true;
  if (!pinned) {
    throw new Error(
      "the pinned row's route does not carry the zero-retention pin (order of one, allow_fallbacks false, zdr true, require_parameters true)",
    );
  }
}

/** The request body, pure: the row's sampling spreads at the top level (temperature, reasoning),
 *  the row's route becomes OpenRouter's `provider` object, the schema is strict. */
export function openRouterBody(req: ModelRequest): Record<string, unknown> {
  return {
    // The row's sampling spreads FIRST: a sampling object can never override a pinned key
    // (`model`, `max_tokens`, `messages`, `response_format`, `provider`) — only add to them.
    ...req.sampling,
    model: req.model,
    messages: [{ role: "system", content: req.system }, { role: "user", content: req.user }],
    max_tokens: req.maxTokens,
    response_format: { type: "json_schema", json_schema: { name: "verdict", strict: true, schema: req.schema } },
    provider: req.route,
  };
}

export class OpenRouterModel implements JudgeModel {
  #apiKey: string;
  #url: string;
  #timeoutMs: number;
  #fetch: typeof fetch;
  constructor(opts: { apiKey: string; baseURL?: string; timeoutMs?: number; fetchImpl?: typeof fetch }) {
    this.#apiKey = opts.apiKey;
    this.#url = opts.baseURL ? `${opts.baseURL.replace(/\/$/, "")}/chat/completions` : OPENROUTER_URL;
    this.#timeoutMs = opts.timeoutMs ?? CALL_TIMEOUT_MS;
    this.#fetch = opts.fetchImpl ?? fetch;
  }
  async complete(req: ModelRequest): Promise<ModelReply> {
    assertPinnedRoute(req.route);
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), this.#timeoutMs);
    let response: Response;
    try {
      response = await this.#fetch(this.#url, {
        method: "POST",
        headers: {
          "Authorization": `Bearer ${this.#apiKey}`,
          "Content-Type": "application/json",
          "HTTP-Referer": "https://knowlu.com",
          "X-Title": "Knowlu",
        },
        body: JSON.stringify(openRouterBody(req)),
        signal: controller.signal,
      });
    } catch (e) {
      if (e instanceof DOMException && e.name === "AbortError") {
        throw new Error(`the model call timed out after ${this.#timeoutMs} ms`);
      }
      throw e;
    } finally {
      clearTimeout(timer);
    }
    if (!response.ok) {
      await response.body?.cancel();
      throw new Error(`the model service answered HTTP ${response.status}`);
    }
    const envelopeText = await response.text();
    let data: {
      choices?: { finish_reason?: string; message?: { content?: string | null; refusal?: string | null } }[];
      usage?: { prompt_tokens?: number; completion_tokens?: number };
      error?: unknown;
    };
    try {
      data = JSON.parse(envelopeText);
    } catch {
      // Deliberately does NOT quote the body: a 2xx answer that is not JSON at all (an HTML error
      // page from an intermediary, say) must not leak its text into a 500 log line.
      throw new Error(`the model service answered with a body that is not JSON (${envelopeText.length} chars)`);
    }
    if (data.error !== undefined && (data.choices === undefined || data.choices.length === 0)) {
      // Never includes `data.error`'s text: OpenRouter can answer HTTP 200 with an error envelope
      // (an upstream fault it chose not to surface as a non-2xx), and that text is exactly the
      // kind of provider string that can carry a fragment of the prompt back out (whole-branch
      // review M2) — the same reason `judge_pipeline.ts` never logs a failure's message.
      throw new Error("the model service answered a 2xx with an error envelope");
    }
    const choice = data.choices?.[0];
    if (choice?.message?.refusal) throw new ModelRefused("the model declined");
    if (choice?.finish_reason === "length") {
      throw new Error(`the reply hit max_tokens (${req.maxTokens}) for model ${req.model}: raise the row's max_tokens`);
    }
    const text = choice?.message?.content ?? "";
    let json: unknown;
    try {
      json = JSON.parse(text);
    } catch {
      throw new Error(`the model's reply did not parse as JSON (${text.length} chars)`);
    }
    if (json === null || typeof json !== "object" || Array.isArray(json)) {
      throw new Error("the model's reply did not parse as JSON: not an object");
    }
    return {
      json: json as Record<string, unknown>,
      inputTokens: data.usage?.prompt_tokens ?? 0,
      outputTokens: data.usage?.completion_tokens ?? 0,
    };
  }
}
