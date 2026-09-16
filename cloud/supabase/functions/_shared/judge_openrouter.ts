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

/** The request body, pure: the row's sampling spreads at the top level (temperature, reasoning),
 *  the row's route becomes OpenRouter's `provider` object, the schema is strict. */
export function openRouterBody(req: ModelRequest): Record<string, unknown> {
  return {
    model: req.model,
    messages: [{ role: "system", content: req.system }, { role: "user", content: req.user }],
    max_tokens: req.maxTokens,
    response_format: { type: "json_schema", json_schema: { name: "verdict", strict: true, schema: req.schema } },
    provider: req.route,
    ...req.sampling,
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
    const data = await response.json() as {
      choices?: { finish_reason?: string; message?: { content?: string | null; refusal?: string | null } }[];
      usage?: { prompt_tokens?: number; completion_tokens?: number };
    };
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
