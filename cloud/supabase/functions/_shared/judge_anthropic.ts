// The inference provider, and the ONLY file in this repository that imports an inference SDK
// (cloud design §10: "Nothing model-shaped ships in the app"; §11 R8: Anthropic is the launch
// provider). Everything else in the service talks to `JudgeModel`, so swapping the provider is
// this file plus one row in `models`.
//
// Three properties this file exists to hold:
//   1. **Determinism where it can exist** (§5.2): a JSON schema on every call, no `thinking` — the
//      three judgments are classifications, not reasoning tasks — and whatever sampling the pinned
//      row asks for. Sampling is the ROW's business, not this file's: `temperature`/`top_p`/`top_k`
//      are removed and return a 400 on Sonnet 5, Opus 5, Opus 4.8/4.7 and Fable 5/5.1, and remain
//      valid on Haiku 4.5 and the 4.6 generation (checked with the claude-api skill, 2026-09-09).
//      §11 R8 lets the eval move the pin; a hard-coded `temperature: 0` would make the first such
//      move 400 every call and read as a provider outage.
//   2. **Every unusable answer says which kind of unusable it was.** A truncated reply, a refusal
//      and a genuinely malformed reply are three different faults with three different fixes, and
//      the pipeline logs all three as `model failed` — so only these words tell them apart.
//   3. **The body never reaches a log.** Nothing here logs, and no error it throws quotes the
//      prompt or the reply text.
//
// Two things deliberately NOT used, so the next reader does not "fix" them:
//   - **Prompt caching.** The system prompts here are ~280 tokens; the minimum cacheable prefix is
//     512-4096 tokens depending on the model, so a `cache_control` breakpoint would silently never
//     cache. Revisit only if a prompt grows past the floor.
//   - **The Batch API** (50% cheaper, and the twice-daily slot is latency-tolerant for events and
//     email). It is the right lever if per-account spend ever becomes the binding constraint, and
//     it is a change to this file plus a queue — not to the pipeline. Recorded in Task 14's budget
//     work as the first thing to reach for.
import Anthropic from "npm:@anthropic-ai/sdk@0.125.0";

/// One call's wall-clock bound, and the DEFAULT only — a caller that makes many calls inside one
/// invocation passes its own (`gmail-read` passes 60 s, so its 40 s read budget plus one call in
/// flight still lands inside the edge function's own wall clock).
///
/// 120 s is the device's own per-call bound (`cloudmodel::CALL_TIMEOUT`) exactly, not a hair under
/// it: a shorter bound here would only mean the service gave up on a call the device was still
/// willing to wait for, and then had nothing to say about it. Equal means the device is the one
/// that gives up, with a timeout it can name. (C2 final review S-6: the sentence that used to be
/// here had lost a clause in an edit and read as its own opposite.)
export const CALL_TIMEOUT_MS = 120_000;

/// What the pinned row says about sampling. `{}` means "this model has no sampling parameters" —
/// which is the correct request for every model above Haiku.
export type Sampling = { temperature: number } | Record<string, never>;

export interface ModelRequest {
  model: string;
  system: string;
  user: string;
  /** The JSON schema the reply is constrained to. */
  schema: Record<string, unknown>;
  maxTokens: number;
  sampling: Sampling;
}

export interface ModelReply {
  json: Record<string, unknown>;
  inputTokens: number;
  outputTokens: number;
}

export interface JudgeModel {
  complete(req: ModelRequest): Promise<ModelReply>;
}

/// `stop_reason: "refusal"` — HTTP 200, no usable content, a safety classifier declined. Its own
/// class so the pipeline can one day route it somewhere other than `model failed`.
export class ModelRefused extends Error {}

export class AnthropicModel implements JudgeModel {
  readonly #client: Anthropic;

  constructor(opts: { apiKey: string; baseURL?: string; timeoutMs?: number }) {
    this.#client = new Anthropic({
      apiKey: opts.apiKey,
      ...(opts.baseURL === undefined ? {} : { baseURL: opts.baseURL }),
      // Milliseconds in the TypeScript SDK (seconds in Python) — a real trap, and the reason this
      // constant is named `_MS`.
      timeout: opts.timeoutMs ?? CALL_TIMEOUT_MS,
      // One retry, not two: the caller is a slot step with its own budget, and a judgment that
      // fails is a normal outcome the device reports, never a failed run.
      maxRetries: 1,
    });
  }

  async complete(req: ModelRequest): Promise<ModelReply> {
    const response = await this.#client.messages.create({
      model: req.model,
      max_tokens: req.maxTokens,
      system: req.system,
      messages: [{ role: "user", content: req.user }],
      output_config: { format: { type: "json_schema", schema: req.schema } },
      ...req.sampling,
    });
    // Checked BEFORE `content` is read, always: on a refusal `content` is empty and on a
    // `max_tokens` stop it is half an object, and both would otherwise arrive as "bad JSON".
    if (response.stop_reason === "refusal") {
      throw new ModelRefused(`the model declined (${response.stop_details?.category ?? "no category"})`);
    }
    if (response.stop_reason === "max_tokens") {
      throw new Error(
        `the reply hit max_tokens (${req.maxTokens}) for model ${req.model}: raise the row's max_tokens`,
      );
    }
    const text = response.content
      .filter((block): block is Anthropic.TextBlock => block.type === "text")
      .map((block) => block.text)
      .join("");
    let json: unknown;
    try {
      json = JSON.parse(text);
    } catch {
      // Deliberately does NOT quote the text: an unparsed reply is the one case where the model
      // may have echoed the item, and this message reaches a 500 log line.
      throw new Error(`the model's reply did not parse as JSON (${text.length} chars)`);
    }
    if (json === null || typeof json !== "object" || Array.isArray(json)) {
      throw new Error("the model's reply did not parse as JSON: not an object");
    }
    return {
      json: json as Record<string, unknown>,
      inputTokens: response.usage.input_tokens,
      outputTokens: response.usage.output_tokens,
    };
  }
}

/// The test double. Answers the scripted replies in order and records every request it was given,
/// so a pipeline test can assert what the prompt carried without a provider, a key or a socket.
export class ScriptedModel implements JudgeModel {
  readonly seen: ModelRequest[] = [];
  readonly #replies: Array<Record<string, unknown> | Error>;

  constructor(replies: Array<Record<string, unknown> | Error>) {
    this.#replies = [...replies];
  }

  complete(req: ModelRequest): Promise<ModelReply> {
    this.seen.push(req);
    const next = this.#replies.shift();
    if (next === undefined) {
      return Promise.reject(new Error("ScriptedModel: no reply scripted for this call"));
    }
    if (next instanceof Error) return Promise.reject(next);
    // Two token counts that are not zero, so a test that asserts usage is recorded actually can.
    return Promise.resolve({ json: next, inputTokens: 300, outputTokens: 60 });
  }
}
