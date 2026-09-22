import { assertEquals } from "@std/assert";
import { ScriptedModel } from "./judge_anthropic.ts";
import { judgeHandler } from "./judge_handler.ts";
import type {
  CapStore,
  JudgmentRow,
  JudgmentSink,
  ModelRow,
  PipelineDeps,
  RuleTable,
} from "./judge_pipeline.ts";

const ROW: ModelRow = {
  kind: "task",
  provider: "anthropic",
  model_id: "claude-haiku-4-5",
  prompt_version: "task-1",
  grammar_version: "task-1",
  max_tokens: 256,
  sampling: { temperature: 0 },
  route: {},
  precision: "bf16",
  usd_per_m_in: 1.0,
  usd_per_m_out: 5.0,
};
const NO_RULES: RuleTable = { lookup: () => Promise.resolve(null) };
const ALWAYS: CapStore = {
  charge: () => Promise.resolve(true),
  withinBudget: () => Promise.resolve(true),
  recordTokens: () => Promise.resolve(),
};
const SINK: JudgmentSink = { write: (_row: JudgmentRow) => Promise.resolve("judgment-1") };

function deps(model: ScriptedModel): () => Promise<PipelineDeps> {
  return () =>
    Promise.resolve({
      row: ROW,
      model,
      rules: NO_RULES,
      caps: ALWAYS,
      log: SINK,
      origin: "device",
      now: () => 0,
    });
}

const BODY = JSON.stringify({
  kind: "task",
  item: {
    id: "task-abc123",
    title: "CS 100 HW 01",
    body: "sections",
    source_uid: "zybooks:1",
    created_by: "zybooks",
    course: null,
    due: "2026-09-18T23:59",
  },
  heuristics_seed: {
    course: null,
    effort_hours: null,
    slice_hours: 1.5,
    weights: "Homework 20%",
    preferences: "",
    known_courses: ["cs-100"],
  },
});

const OK = () => Promise.resolve({ account_id: "acct-1" });
const ANSWER = {
  course: "cs-100",
  effort_hours: 2.5,
  importance: 4,
  importance_reason: "20% of the grade",
  confidence: 0.82,
};

Deno.test("a judged task comes back as a verdict, a tier, the model, and the judgment id", async () => {
  const handler = judgeHandler("task", OK, deps(new ScriptedModel([ANSWER])));
  const response = await handler(new Request("http://127.0.0.1/judge-task", { method: "POST", body: BODY }));
  assertEquals(response.status, 200);
  const reply = await response.json();
  assertEquals(reply.tier, 3);
  assertEquals(reply.outcome, "answered");
  assertEquals(reply.verdict.effort_hours, 2.5);
  assertEquals(reply.model, "claude-haiku-4-5");
  assertEquals(reply.judgment_id, "judgment-1");
});

Deno.test("the entitlement check runs before anything is parsed or charged", async () => {
  const model = new ScriptedModel([]);
  const refuse = () => Promise.reject(Response.json({ error: "no active subscription" }, { status: 402 }));
  const handler = judgeHandler("task", refuse, deps(model));
  const response = await handler(new Request("http://127.0.0.1/judge-task", { method: "POST", body: BODY }));
  assertEquals(response.status, 402);
  assertEquals(model.seen.length, 0);
});

Deno.test("a body for the wrong kind is a 400 and never reaches the model", async () => {
  const model = new ScriptedModel([]);
  const handler = judgeHandler("task", OK, deps(model));
  const wrong = JSON.stringify({ kind: "event", item: {}, heuristics_seed: {} });
  const response = await handler(new Request("http://127.0.0.1/judge-task", { method: "POST", body: wrong }));
  assertEquals(response.status, 400);
  assertEquals(model.seen.length, 0);
});

Deno.test("a GET is a 405, so a browser cannot spend an account's cap by visiting the URL", async () => {
  const handler = judgeHandler("task", OK, deps(new ScriptedModel([])));
  const response = await handler(new Request("http://127.0.0.1/judge-task", { method: "GET" }));
  assertEquals(response.status, 405);
});

Deno.test("a thrown error is a 500 that names nothing from the request", async () => {
  const handler = judgeHandler(
    "task",
    OK,
    () => Promise.reject(new Error("connect ECONNREFUSED TRIPWIRE-9f2c")),
  );
  const response = await handler(new Request("http://127.0.0.1/judge-task", { method: "POST", body: BODY }));
  assertEquals(response.status, 500);
  const text = await response.text();
  assertEquals(text.includes("TRIPWIRE-9f2c"), false);
  assertEquals(text.includes("CS 100 HW 01"), false);
});

// Final review item 2: the handler carries the request's `accepts` through to the pipeline's gate.
Deno.test("the event handler answers unsure only to a request that declares it", async () => {
  const unsure = { verdict: "unsure", why: "the text does not say who it is for", confidence: 0.3 };
  const event = (extra: Record<string, unknown>) =>
    JSON.stringify({ kind: "event", item: { uid: "engage:1", title: "AI Club Kickoff" }, heuristics_seed: {}, ...extra });
  const ask = async (body: string) => {
    const handler = judgeHandler("event", OK, deps(new ScriptedModel([unsure])));
    const response = await handler(new Request("http://127.0.0.1/judge-event", { method: "POST", body }));
    assertEquals(response.status, 200);
    return await response.json();
  };
  const declared = await ask(event({ accepts: ["unsure"] }));
  assertEquals(declared.verdict.verdict, "unsure");
  const old = await ask(event({}));
  assertEquals(old.verdict, null);
  assertEquals(old.cause, "below floor");
  const malformed = await ask(event({ accepts: "unsure" }));
  assertEquals(malformed.verdict, null, "a non-array accepts declares nothing");
});
