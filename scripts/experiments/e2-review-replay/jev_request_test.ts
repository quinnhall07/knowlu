import { assert, assertEquals } from "@std/assert";
import type { CorpusFinding } from "./types.ts";
import {
  buildJevNativeBody,
  buildOpenRouterChatBody,
  buildState,
  contextFor,
  estimateCorpusCost,
  estimateTokens,
  JEV_MODEL_ID,
  primaryFileFor,
  QUESTIONS,
  ZDR_ROUTE,
} from "./jev_request.ts";

function fixture(overrides: Partial<CorpusFinding> = {}): CorpusFinding {
  return {
    id: "B-task1-5",
    set: "B",
    source: "task-1-review.md",
    severityRaw: "should-fix",
    severity: "important",
    disposition: "fixed",
    dispositionSource: "test fixture",
    text: "the `cloud/supabase/functions/account/index.ts:100-132` idempotence is a check-then-act.",
    rawText:
      "5. should-fix — the `cloud/supabase/functions/account/index.ts:100-132` idempotence is a check-then-act.",
    kept: true,
    ...overrides,
  };
}

Deno.test("primaryFileFor pulls the first backtick-quoted file path", () => {
  assertEquals(
    primaryFileFor("the `cloud/supabase/functions/account/index.ts:100-132` idempotence check"),
    "cloud/supabase/functions/account/index.ts",
  );
  assertEquals(primaryFileFor("no file named here at all"), null);
});

Deno.test("contextFor maps a known source file to its one-line task description", () => {
  assertEquals(contextFor("task-3-review.md"), TASK3_DESCRIPTION());
  assertEquals(contextFor("2026-09-17-c1b-sign-in-plan-review.md").includes("plan review"), true);
});
function TASK3_DESCRIPTION() {
  return "Task 3 — the loopback listener, the PKCE pair, and google_sign_in.";
}

Deno.test("contextFor falls back to naming the source for an unmapped file, never throws", () => {
  assertEquals(contextFor("some-other-file.md"), "Source: some-other-file.md");
});

Deno.test("buildState assembles finding/file/context exactly as procedure §Step 2 specifies", () => {
  const f = fixture();
  const state = buildState(f);
  assertEquals(state.finding, f.text);
  assertEquals(state.file, "cloud/supabase/functions/account/index.ts");
  assertEquals(state.context, contextFor(f.source));
});

Deno.test("buildJevNativeBody carries the model id and all three questions", () => {
  const body = buildJevNativeBody(fixture());
  assertEquals(body.model, JEV_MODEL_ID);
  assertEquals(body.questions, QUESTIONS);
  assert("state" in body);
});

Deno.test("buildOpenRouterChatBody is pinned zero-retention and carries the state as JSON in the user turn", () => {
  const body = buildOpenRouterChatBody(fixture()) as {
    model: string;
    messages: { role: string; content: string }[];
    provider: typeof ZDR_ROUTE;
  };
  assertEquals(body.model, JEV_MODEL_ID);
  assertEquals(body.provider, ZDR_ROUTE);
  assertEquals(body.messages.length, 1);
  assertEquals(body.messages[0].role, "user");
  assert(body.messages[0].content.includes("account/index.ts"));
  assert(body.messages[0].content.includes("grade"));
  assert(body.messages[0].content.includes("disposition"));
});

Deno.test("estimateTokens is roughly chars/4 and monotone in length", () => {
  assertEquals(estimateTokens(""), 0);
  assertEquals(estimateTokens("abcd"), 1);
  assert(estimateTokens("a".repeat(4000)) < estimateTokens("a".repeat(8000)));
});

Deno.test("estimateCorpusCost scales linearly with item count and matches the per-item token math", () => {
  const one = estimateCorpusCost([fixture()]);
  const three = estimateCorpusCost([fixture(), fixture({ id: "x2" }), fixture({ id: "x3" })]);
  assertEquals(three.items, 3);
  assertEquals(three.totalInputChars, one.totalInputChars * 3);
  assertEquals(three.estimatedCostUsd, one.estimatedCostUsd * 3);
  assert(one.estimatedCostUsd > 0);
  // Sanity bound: the whole 60-item corpus, minimal state, should stay under a dollar by a wide
  // margin (the note's own table puts the generous-state, both-arms figure at $0.018).
  const wholeCorpus = estimateCorpusCost(Array.from({ length: 60 }, (_, i) => fixture({ id: `id${i}` })));
  assert(wholeCorpus.estimatedCostUsd < 1);
});
