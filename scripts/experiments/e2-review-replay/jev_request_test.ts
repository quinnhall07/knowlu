import { assert, assertAlmostEquals, assertEquals, assertThrows } from "@std/assert";
import type { CorpusFinding } from "./types.ts";
import {
  buildDecisionsBody,
  buildState,
  contextFor,
  DECISION_QUESTIONS,
  DISPOSITION_OPTION_TO_LABEL,
  estimateCorpusCost,
  estimateTokens,
  INPUT_COST_PER_MILLION_USD,
  JEV_MODEL_ID,
  OPENROUTER_DECISIONS_URL,
  primaryFileFor,
  ZDR_ROUTE,
} from "./jev_request.ts";
import { findLeaks } from "./severity.ts";

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

Deno.test("the transport is OpenRouter's decisions endpoint, not chat/completions", () => {
  assertEquals(OPENROUTER_DECISIONS_URL, "https://openrouter.ai/api/alpha/decisions");
});

Deno.test("the ZDR route is the provider block the controller's smoke call verified", () => {
  assertEquals(ZDR_ROUTE, { order: ["typesafe"], allow_fallbacks: false, zdr: true });
});

Deno.test("buildDecisionsBody is {model, state, questions, provider} with the state as structured JSON", () => {
  const f = fixture();
  const body = buildDecisionsBody(f);
  assertEquals(Object.keys(body).sort(), ["model", "provider", "questions", "state"]);
  assertEquals(body.model, JEV_MODEL_ID);
  assertEquals(body.provider, ZDR_ROUTE);
  assertEquals(body.state, buildState(f));
  assertEquals(body.questions, DECISION_QUESTIONS);
});

Deno.test("grade and disposition are choice questions whose every option carries a criterion", () => {
  const { grade, disposition, blocks } = DECISION_QUESTIONS;
  assertEquals(grade.type, "choice");
  assertEquals(Object.keys(grade.criteria).sort(), ["critical", "important", "minor"]);
  assertEquals(disposition.type, "choice");
  assertEquals(Object.keys(disposition.criteria).sort(), ["defer", "fix", "hand_off", "rule_against"]);
  for (const q of [grade, disposition]) {
    assert(q.instructions.length > 0);
    for (const d of Object.values(q.criteria)) assert(d.length > 20, `criterion too thin: ${d}`);
  }
  assertEquals(blocks.type, "noul");
  assertEquals(blocks.instructions, "This finding must be fixed before the branch merges.");
});

Deno.test("every disposition option maps to exactly one corpus disposition label", () => {
  assertEquals(DISPOSITION_OPTION_TO_LABEL, {
    fix: "fixed",
    rule_against: "ruled_against",
    hand_off: "handed_off",
    defer: "deferred",
  });
});

Deno.test("buildDecisionsBody refuses a state that still carries a severity word", () => {
  const leaky = fixture({ text: "should-fix — the header still says four routes" });
  assertThrows(() => buildDecisionsBody(leaky), Error, "leak");
});

Deno.test("buildDecisionsBody does not mistake a JSON newline escape before 'it' for a leak", () => {
  // JSON.stringify writes "\n" + "it" as `\nit`, and `\bnit\b` matches that — the paid run's first
  // pass stopped on exactly this at B-task3-2. The guard reads the state's own strings instead.
  const f = fixture({ text: "the listener binds before the deadline\nit then polls `accept` in a loop." });
  assertEquals(findLeaks(f.text), []);
  buildDecisionsBody(f);
});

Deno.test("buildDecisionsBody still refuses a leak in the state's context or file, not only the finding", () => {
  const leaky = fixture({ source: "nit-notes.md" }); // unmapped source -> context "Source: nit-notes.md"
  assertThrows(() => buildDecisionsBody(leaky), Error, "leak");
});

Deno.test("the state of a clean finding has no leak terms", () => {
  assertEquals(findLeaks(JSON.stringify(buildDecisionsBody(fixture()).state)), []);
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
  // chars/4 is rounded up once over the whole pass, not per item, so 3 x ceil(c/4) and ceil(3c/4)
  // differ by at most two tokens.
  assert(Math.abs(three.estimatedInputTokens - one.estimatedInputTokens * 3) <= 2);
  assertAlmostEquals(
    three.estimatedCostUsd,
    one.estimatedCostUsd * 3,
    (2 * INPUT_COST_PER_MILLION_USD) / 1e6,
  );
  assert(one.estimatedCostUsd > 0);
  // Sanity bound: the whole 60-item corpus, minimal state, should stay under a dollar by a wide
  // margin (the note's own table puts the generous-state, both-arms figure at $0.018).
  const wholeCorpus = estimateCorpusCost(Array.from({ length: 60 }, (_, i) => fixture({ id: `id${i}` })));
  assert(wholeCorpus.estimatedCostUsd < 1);
});
