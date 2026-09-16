// The OpenRouter adapter, exercised entirely against an injected fetch — no listener, no socket,
// no network permission needed for this file (CLAUDE.md, plan 3a's loopback rule covers the
// Anthropic adapter's real-listener tests; this one needs none of that).
import { assertEquals, assertRejects } from "@std/assert";
import { ModelRefused } from "./judge_anthropic.ts";
import { OPENROUTER_URL, openRouterBody, OpenRouterModel } from "./judge_openrouter.ts";

const SCHEMA = { type: "object" } as const;

function request(overrides: Record<string, unknown> = {}) {
  return {
    model: "qwen/qwen3.5-35b-a3b",
    system: "S",
    user: "U",
    schema: SCHEMA as unknown as Record<string, unknown>,
    maxTokens: 640,
    sampling: { temperature: 0, reasoning: { enabled: false } },
    route: { order: ["DeepInfra"], allow_fallbacks: false, zdr: true, require_parameters: true },
    ...overrides,
  };
}

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

Deno.test("openRouterBody builds an OpenAI-compatible request with the schema, the pin and reasoning off", () => {
  const body = openRouterBody(request());
  assertEquals(body, {
    model: "qwen/qwen3.5-35b-a3b",
    messages: [{ role: "system", content: "S" }, { role: "user", content: "U" }],
    max_tokens: 640,
    response_format: { type: "json_schema", json_schema: { name: "verdict", strict: true, schema: { type: "object" } } },
    provider: { order: ["DeepInfra"], allow_fallbacks: false, zdr: true, require_parameters: true },
    temperature: 0,
    reasoning: { enabled: false },
  });
});

Deno.test("a reply's content parses and its usage is mapped", async () => {
  let seenUrl = "";
  let seenInit: RequestInit | undefined;
  const fake: typeof fetch = (input, init) => {
    seenUrl = String(input);
    seenInit = init;
    return Promise.resolve(jsonResponse({
      choices: [{ finish_reason: "stop", message: { content: '{"tier":"task"}' } }],
      usage: { prompt_tokens: 810, completion_tokens: 80 },
    }));
  };
  const model = new OpenRouterModel({ apiKey: "test-key-not-a-secret", fetchImpl: fake });
  const reply = await model.complete(request());
  assertEquals(reply, { json: { tier: "task" }, inputTokens: 810, outputTokens: 80 });
  assertEquals(seenUrl, OPENROUTER_URL);
  assertEquals(seenInit?.method, "POST");
  assertEquals(JSON.parse(String(seenInit?.body)), openRouterBody(request()));
  const headers = seenInit?.headers as Record<string, string>;
  assertEquals(headers["Authorization"], "Bearer test-key-not-a-secret");
  assertEquals(headers["HTTP-Referer"], "https://knowlu.com");
  assertEquals(headers["X-Title"], "Knowlu");
  assertEquals(headers["Content-Type"], "application/json");
});

Deno.test("a length stop is the max_tokens error, not bad JSON", async () => {
  const fake: typeof fetch = () =>
    Promise.resolve(jsonResponse({
      choices: [{ finish_reason: "length", message: { content: '{"tier":"ta' } }],
      usage: { prompt_tokens: 1, completion_tokens: 1 },
    }));
  const model = new OpenRouterModel({ apiKey: "test-key-not-a-secret", fetchImpl: fake });
  await assertRejects(
    () => model.complete(request()),
    Error,
    "hit max_tokens (640)",
  );
  await assertRejects(
    () => model.complete(request()),
    Error,
    "qwen/qwen3.5-35b-a3b",
  );
});

Deno.test("a refusal is ModelRefused", async () => {
  const fake: typeof fetch = () =>
    Promise.resolve(jsonResponse({
      choices: [{ finish_reason: "stop", message: { content: null, refusal: "no" } }],
      usage: { prompt_tokens: 1, completion_tokens: 1 },
    }));
  const model = new OpenRouterModel({ apiKey: "test-key-not-a-secret", fetchImpl: fake });
  await assertRejects(() => model.complete(request()), ModelRefused);
});

Deno.test("a non-2xx answer names the status and never the body", async () => {
  const fake: typeof fetch = () =>
    Promise.resolve(jsonResponse({ error: { message: "SECRET-LOOKING-TEXT" } }, 429));
  const model = new OpenRouterModel({ apiKey: "test-key-not-a-secret", fetchImpl: fake });
  await assertRejects(
    () => model.complete(request()),
    Error,
    "HTTP 429",
  );
  let message = "";
  try {
    await model.complete(request());
  } catch (e) {
    message = e instanceof Error ? e.message : String(e);
  }
  assertEquals(message.includes("SECRET-LOOKING-TEXT"), false);
});

Deno.test("a reply that is not a JSON object is named without quoting it", async () => {
  const arrayFake: typeof fetch = () =>
    Promise.resolve(jsonResponse({
      choices: [{ finish_reason: "stop", message: { content: "[1,2]" } }],
      usage: { prompt_tokens: 1, completion_tokens: 1 },
    }));
  const modelArray = new OpenRouterModel({ apiKey: "test-key-not-a-secret", fetchImpl: arrayFake });
  await assertRejects(
    () => modelArray.complete(request()),
    Error,
    "did not parse as JSON: not an object",
  );

  const nopeFake: typeof fetch = () =>
    Promise.resolve(jsonResponse({
      choices: [{ finish_reason: "stop", message: { content: "nope" } }],
      usage: { prompt_tokens: 1, completion_tokens: 1 },
    }));
  const modelNope = new OpenRouterModel({ apiKey: "test-key-not-a-secret", fetchImpl: nopeFake });
  let message = "";
  try {
    await modelNope.complete(request());
  } catch (e) {
    message = e instanceof Error ? e.message : String(e);
  }
  assertEquals(message.includes("did not parse as JSON (4 chars)"), true);
  assertEquals(message.includes("nope"), false);
});

Deno.test("the request times out at timeoutMs", async () => {
  const fake: typeof fetch = (_input, init) =>
    new Promise((_resolve, reject) => {
      const signal = init?.signal;
      signal?.addEventListener("abort", () => {
        reject(new DOMException("Aborted", "AbortError"));
      });
    });
  const model = new OpenRouterModel({ apiKey: "test-key-not-a-secret", fetchImpl: fake, timeoutMs: 10 });
  await assertRejects(() => model.complete(request()), Error, "timed out after 10 ms");
});

Deno.test("a 2xx body that is not JSON at all is named by length, never quoted", async () => {
  const fake: typeof fetch = () =>
    Promise.resolve(
      new Response("<html>SECRET-LOOKING</html>", { status: 200, headers: { "Content-Type": "text/html" } }),
    );
  const model = new OpenRouterModel({ apiKey: "test-key-not-a-secret", fetchImpl: fake });
  let message = "";
  try {
    await model.complete(request());
  } catch (e) {
    message = e instanceof Error ? e.message : String(e);
  }
  assertEquals(message.includes("a body that is not JSON (27 chars)"), true, message);
  assertEquals(message.includes("SECRET-LOOKING"), false, message);
});

Deno.test("a sampling that carries model or max_tokens never overrides the pinned request", () => {
  const body = openRouterBody(request({ sampling: { model: "evil", max_tokens: 1, temperature: 0 } }));
  assertEquals(body.model, "qwen/qwen3.5-35b-a3b");
  assertEquals(body.max_tokens, 640);
  assertEquals(body.temperature, 0);
});
