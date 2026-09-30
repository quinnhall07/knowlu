import { assert, assertEquals } from "@std/assert";
import { runPass, type SendFn } from "./replay.ts";
import type { CorpusFinding } from "./types.ts";

function finding(id: string): CorpusFinding {
  return {
    id,
    set: "B",
    source: "task-1-review.md",
    severityRaw: "nit",
    severity: "minor",
    disposition: "fixed",
    dispositionSource: "fixture",
    text: `the \`handler.ts:1\` header for ${id} is stale.`,
    rawText: "",
    kept: true,
  };
}

const OK = JSON.stringify({
  model: "typesafe/jev-1.13-20260917",
  answers: {
    grade: { type: "choice", choice: "minor", probabilities: { minor: 1 }, confidence: 0.5 },
    blocks: { type: "noul", noul: 0.2 },
    disposition: { type: "choice", choice: "fix", probabilities: { fix: 1 }, confidence: 0.5 },
  },
  usage: { input_tokens: 10, output_tokens: 5, cost: 0.000001 },
});

const quiet = () => {};

Deno.test("runPass records every item and keeps going past an isolated failure", async () => {
  const outcomes = [OK, "fail", OK];
  let i = 0;
  const send: SendFn = () => {
    const o = outcomes[i++];
    return Promise.resolve(o === "fail" ? { status: 500, text: "{}" } : { status: 200, text: o });
  };
  const r = await runPass([finding("a"), finding("b"), finding("c")], send, { log: quiet });
  assertEquals(r.stoppedEarly, false);
  assertEquals(r.results.map((x) => x.id), ["a", "b", "c"]);
  assert(r.results[0].reply);
  assert(r.results[1].error?.startsWith("HTTP 500"));
  assert(r.results[2].reply);
});

Deno.test("runPass records a thrown network error as that item's failure", async () => {
  const send: SendFn = () => Promise.reject(new Error("connection reset"));
  const r = await runPass([finding("a")], send, { log: quiet });
  assertEquals(r.results[0].status, null);
  assertEquals(r.results[0].error, "network: connection reset");
});

Deno.test("runPass stops after five consecutive failures and says so", async () => {
  let calls = 0;
  const send: SendFn = () => {
    calls++;
    return Promise.resolve({
      status: 429,
      text: JSON.stringify({ error: { code: 429, message: "slow down" } }),
    });
  };
  const items = Array.from({ length: 8 }, (_, k) => finding(`f${k}`));
  const r = await runPass(items, send, { log: quiet });
  assertEquals(calls, 5);
  assertEquals(r.results.length, 5);
  assertEquals(r.stoppedEarly, true);
});

Deno.test("runPass resets the consecutive-failure count on a success", async () => {
  const pattern = ["f", "f", "f", "f", "ok", "f", "f", "f", "f", "ok"];
  let i = 0;
  const send: SendFn = () =>
    Promise.resolve(pattern[i++] === "ok" ? { status: 200, text: OK } : { status: 500, text: "" });
  const items = pattern.map((_, k) => finding(`f${k}`));
  const r = await runPass(items, send, { log: quiet });
  assertEquals(r.stoppedEarly, false);
  assertEquals(r.results.length, 10);
});

Deno.test("runPass sends the decisions body and keeps it in the record", async () => {
  let sent: unknown;
  const send: SendFn = (body) => {
    sent = body;
    return Promise.resolve({ status: 200, text: OK });
  };
  const r = await runPass([finding("a")], send, { log: quiet });
  assertEquals((sent as { model: string }).model, "typesafe/jev-1.13");
  assertEquals(r.results[0].request, sent);
});

Deno.test("runPass never records anything shaped like an OpenRouter key in an error", async () => {
  const send: SendFn = () => Promise.reject(new Error("bad header Bearer sk-or-v1-abc123DEF")); // not a real key
  const r = await runPass([finding("a")], send, { log: quiet });
  assertEquals(r.results[0].error, "network: bad header Bearer [redacted]");
});
