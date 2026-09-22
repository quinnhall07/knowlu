import { assert, assertEquals, assertThrows } from "@std/assert";
import { promptHash } from "../../../cloud/supabase/functions/_shared/judge_prompts.ts";
import type { ModelRow } from "../../../cloud/supabase/functions/_shared/judge_models.ts";
import { ARMS, estimateCost, requestBody } from "./arms.ts";
import { EVENT3_PROMPT_HASH, event3Hash } from "./event3_frozen.ts";

const ROW: ModelRow = {
  kind: "event",
  provider: "openrouter",
  model_id: "ibm-granite/granite-4.2-8b",
  prompt_version: "event-4",
  grammar_version: "event-3",
  max_tokens: 256,
  sampling: { temperature: 0, reasoning: { enabled: false } },
  route: { order: ["CoreWeave"], allow_fallbacks: false, zdr: true, require_parameters: true },
  precision: "bf16 (CoreWeave)",
  usd_per_m_in: 0.10,
  usd_per_m_out: 0.15,
};
const ITEM = { uid: "seed-x", title: "Office hours", start: "a", end: "b", source: "s", description: "Weekly." };

Deno.test("the frozen event-3 arm is byte-for-byte the prompt committed at 7c127e2 (its prompt hash)", async () => {
  assertEquals(await event3Hash(), EVENT3_PROMPT_HASH);
  assertEquals(await ARMS["event-3"].promptHash(), EVENT3_PROMPT_HASH);
});

Deno.test("the event-4 arm is the live product prompt, and differs from the frozen event-3 one", async () => {
  assertEquals(await ARMS["event-4"].promptHash(), await promptHash("event"));
  assert((await ARMS["event-4"].promptHash()) !== EVENT3_PROMPT_HASH);
});

Deno.test("both arms send the same user message, the pinned route and the row's model — only system and schema differ", () => {
  const a = requestBody(ARMS["event-3"], ITEM, {}, ROW);
  const b = requestBody(ARMS["event-4"], ITEM, {}, ROW);
  const msgs = (x: Record<string, unknown>) => x.messages as { role: string; content: string }[];
  assertEquals(msgs(a)[1], msgs(b)[1]);
  assert(msgs(a)[0].content !== msgs(b)[0].content);
  assert(JSON.stringify(a.response_format) !== JSON.stringify(b.response_format));
  for (const body of [a, b]) {
    assertEquals(body.model, ROW.model_id);
    assertEquals(body.provider, ROW.route);
    assertEquals(body.max_tokens, 256);
    assertEquals(body.temperature, 0);
  }
});

Deno.test("an unpinned route is refused before any body is built", () => {
  assertThrows(() => requestBody(ARMS["event-4"], ITEM, {}, { ...ROW, route: {} }));
});

Deno.test("each arm reads a reply through its own validation: event-4 combines the rule fields, event-3 cannot", () => {
  const reply = {
    audience_excludes_student: false,
    standing_or_drop_in: true,
    verdict: "opportunity",
    why: "weekly",
    confidence: 0.9,
  };
  assertEquals(ARMS["event-4"].interpret(reply).verdict?.verdict, "drop");
  assertEquals(ARMS["event-4"].interpret(reply).rule, "standing");
  assertEquals(
    ARMS["event-3"].interpret({ verdict: "opportunity", why: "weekly", confidence: 0.9 }).verdict?.verdict,
    "opportunity",
  );
  assertEquals(ARMS["event-3"].interpret({ verdict: "drop", why: "x", confidence: 0.1 }).verdict, null);
  assertEquals(ARMS["event-3"].interpret({ verdict: "drop", why: "x", confidence: 0.1 }).cause, "below floor");
  assertEquals(
    ARMS["event-3"].interpret({ verdict: "unsure", why: "x", confidence: 0.1 }).verdict?.verdict,
    "unsure",
  );
});

Deno.test("the cost estimate prices both arms over every case at the row's rates", () => {
  const est = estimateCost([{ item: ITEM, seed: {} }, { item: ITEM, seed: {} }], ROW);
  assertEquals(est.calls, 4);
  assert(est.inputTokens > 0 && est.outputTokens > 0);
  const expected = (est.inputTokens * ROW.usd_per_m_in + est.outputTokens * ROW.usd_per_m_out) / 1e6;
  assertEquals(est.usd, expected);
});
