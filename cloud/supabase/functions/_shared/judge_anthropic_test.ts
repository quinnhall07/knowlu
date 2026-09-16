// The model client, exercised against a listener bound to 127.0.0.1:0 inside this same test.
// That is not egress: no DNS, no route off the machine, no listener on a routable interface.
// The listener is shut down before the test returns (CLAUDE.md, plan 3a's loopback rule).
import { assert, assertEquals, assertRejects } from "@std/assert";
import { AnthropicModel, ModelRefused, ScriptedModel } from "./judge_anthropic.ts";

const SCHEMA = {
  type: "object",
  properties: {
    course: { type: ["string", "null"] },
    effort_hours: { type: "number" },
    importance: { type: "integer" },
    importance_reason: { type: "string" },
    confidence: { type: "number" },
  },
  required: ["course", "effort_hours", "importance", "importance_reason", "confidence"],
  additionalProperties: false,
} as const;

const ANSWER =
  '{"course":"cs-100","effort_hours":2.5,"importance":4,"importance_reason":"20% of the grade","confidence":0.82}';

function messagesReply(text: string, stopReason = "end_turn"): Response {
  return Response.json({
    id: "msg_test",
    type: "message",
    role: "assistant",
    model: "claude-haiku-4-5",
    content: [{ type: "text", text }],
    stop_reason: stopReason,
    stop_sequence: null,
    usage: { input_tokens: 11, output_tokens: 22 },
  });
}

function request(overrides: Record<string, unknown> = {}) {
  return {
    model: "claude-haiku-4-5",
    system: "You estimate effort and importance.",
    user: "Title: CS 100 HW 01",
    schema: SCHEMA as unknown as Record<string, unknown>,
    maxTokens: 256,
    sampling: { temperature: 0 },
    route: {},
    ...overrides,
  };
}

Deno.test("the request is the pinned id, the row's sampling, and the schema — and the reply is parsed", async () => {
  let body: Record<string, unknown> = {};
  const server = Deno.serve({ hostname: "127.0.0.1", port: 0, onListen: () => {} }, async (req) => {
    body = await req.json();
    return messagesReply(ANSWER);
  });
  try {
    const model = new AnthropicModel({
      apiKey: "test-key-not-a-secret",
      baseURL: `http://127.0.0.1:${server.addr.port}`,
    });
    const reply = await model.complete(request());
    assertEquals(reply.json.importance, 4);
    assertEquals(reply.json.effort_hours, 2.5);
    assertEquals(reply.inputTokens, 11);
    assertEquals(reply.outputTokens, 22);
    assertEquals(body.model, "claude-haiku-4-5");
    assertEquals(body.max_tokens, 256);
    // Constrained decoding on every call, without exception (cloud design §5.4 measure 3).
    const outputConfig = body.output_config as { format: { type: string; schema: unknown } };
    assertEquals(outputConfig.format.type, "json_schema");
    assertEquals(outputConfig.format.schema, SCHEMA);
    // No `thinking`: these are classifications, and on Haiku 4.5 omitting it is "do not think".
    assertEquals(body.thinking, undefined);
    // No `output_config.effort`: it ERRORS on Haiku 4.5 (checked with the claude-api skill).
    assertEquals((body.output_config as Record<string, unknown>).effort, undefined);
  } finally {
    await server.shutdown();
  }
});

Deno.test("temperature is sent only when the pinned row asks for it", async () => {
  // `temperature` / `top_p` / `top_k` are REMOVED and return a 400 on Sonnet 5, Opus 5, Opus 4.8,
  // Opus 4.7 and Fable 5/5.1; they remain valid on Haiku 4.5 and the 4.6 generation (checked with
  // the claude-api skill, 2026-09-09). §11 R8 says the eval picks the model — so the day it picks
  // one above Haiku, a hard-coded `temperature: 0` would 400 every call and every judgment would
  // read as `model failed`. The parameter therefore travels in the `models` row, not in this file.
  const seen: Array<Record<string, unknown>> = [];
  const server = Deno.serve({ hostname: "127.0.0.1", port: 0, onListen: () => {} }, async (req) => {
    seen.push(await req.json());
    return messagesReply(ANSWER);
  });
  try {
    const model = new AnthropicModel({
      apiKey: "test-key-not-a-secret",
      baseURL: `http://127.0.0.1:${server.addr.port}`,
    });
    await model.complete(request({ sampling: { temperature: 0 } }));
    await model.complete(request({ model: "claude-sonnet-5", sampling: {} }));
    assertEquals(seen[0].temperature, 0);
    assertEquals("temperature" in seen[1], false, "a model with sampling removed must not be sent one");
    assertEquals(seen[1].model, "claude-sonnet-5");
  } finally {
    await server.shutdown();
  }
});

Deno.test("a truncated reply says it was truncated, not that the model talked nonsense", async () => {
  // `stop_reason: "max_tokens"` with half a JSON object is a CONFIGURATION fault — the row's
  // `max_tokens` is too small for that schema — and it must not read as a provider fault, because
  // the pipeline logs both as `model failed` and only the words tell them apart.
  const server = Deno.serve(
    { hostname: "127.0.0.1", port: 0, onListen: () => {} },
    () => messagesReply('{"course":"cs-100","effort_ho', "max_tokens"),
  );
  try {
    const model = new AnthropicModel({
      apiKey: "test-key-not-a-secret",
      baseURL: `http://127.0.0.1:${server.addr.port}`,
    });
    await assertRejects(() => model.complete(request()), Error, "hit max_tokens");
  } finally {
    await server.shutdown();
  }
});

Deno.test("a refusal is its own error class, whatever the model", async () => {
  // `stop_reason: "refusal"` is HTTP 200 with no usable content. Unreachable on Haiku 4.5, and
  // exactly what a pin change to Fable- or Opus-class would introduce — the branch exists so the
  // eval can move the pin without this file becoming the thing that breaks.
  const server = Deno.serve(
    { hostname: "127.0.0.1", port: 0, onListen: () => {} },
    () => messagesReply("", "refusal"),
  );
  try {
    const model = new AnthropicModel({
      apiKey: "test-key-not-a-secret",
      baseURL: `http://127.0.0.1:${server.addr.port}`,
    });
    await assertRejects(() => model.complete(request()), ModelRefused);
  } finally {
    await server.shutdown();
  }
});

Deno.test("a reply that is not the schema's JSON is an error, not a silent empty answer", async () => {
  const server = Deno.serve(
    { hostname: "127.0.0.1", port: 0, onListen: () => {} },
    () => messagesReply("I am afraid I cannot answer that."),
  );
  try {
    const model = new AnthropicModel({
      apiKey: "test-key-not-a-secret",
      baseURL: `http://127.0.0.1:${server.addr.port}`,
    });
    await assertRejects(() => model.complete(request()), Error, "did not parse as JSON");
  } finally {
    await server.shutdown();
  }
});

Deno.test("the scripted fake records what it was asked and answers in order", async () => {
  const fake = new ScriptedModel([{ importance: 3 }, new Error("boom")]);
  const first = await fake.complete(request());
  assertEquals(first.json.importance, 3);
  assertEquals(fake.seen.length, 1);
  assertEquals(fake.seen[0].user, "Title: CS 100 HW 01");
  await assertRejects(() => fake.complete(request({ user: "u2" })), Error, "boom");
  assertEquals(fake.seen.length, 2);
  assert(fake.seen[1].user === "u2");
});
