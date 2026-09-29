import { assert, assertAlmostEquals, assertEquals } from "@std/assert";
import {
  auroc,
  brier,
  type ItemResult,
  nearestRankPercentile,
  parseDecisionReply,
  scoreResults,
} from "./score.ts";
import type { DecisionReply } from "./score.ts";

// The reply the controller's smoke call got on 2026-09-23 (id shortened), plus a disposition answer.
const VERIFIED_REPLY = {
  model: "typesafe/jev-1.13-20260917",
  answers: {
    grade: {
      type: "choice",
      choice: "minor",
      probabilities: { minor: 0.72, critical: 0, important: 0.28 },
      confidence: 0.58,
    },
    blocks: { type: "noul", noul: 0.77 },
    disposition: {
      type: "choice",
      choice: "fix",
      probabilities: { fix: 0.9, rule_against: 0.05, hand_off: 0.03, defer: 0.02 },
      confidence: 0.8,
    },
  },
  usage: { input_tokens: 399, output_tokens: 55, cost: 0.000016758 },
  id: "gen-dec-x",
  provider: "TypeSafe",
};

Deno.test("parseDecisionReply reads the verified reply shape", () => {
  const r = parseDecisionReply(200, JSON.stringify(VERIFIED_REPLY));
  assert(r.ok);
  assertEquals(r.reply.model, "typesafe/jev-1.13-20260917");
  assertEquals(r.reply.grade.choice, "minor");
  assertEquals(r.reply.grade.confidence, 0.58);
  assertEquals(r.reply.grade.probabilities.important, 0.28);
  assertEquals(r.reply.disposition.choice, "fix");
  assertEquals(r.reply.blocks, 0.77);
  assertEquals(r.reply.usage, { inputTokens: 399, outputTokens: 55, costUsd: 0.000016758 });
  assertEquals(r.reply.provider, "TypeSafe");
});

Deno.test("parseDecisionReply names the error body's code and message on a non-2xx", () => {
  const r = parseDecisionReply(400, JSON.stringify({ error: { code: 400, message: "bad state" } }));
  assert(!r.ok);
  assertEquals(r.error, "HTTP 400: 400 bad state");
});

Deno.test("parseDecisionReply reports an error body even on a 200", () => {
  const r = parseDecisionReply(200, JSON.stringify({ error: { code: 502, message: "upstream" } }));
  assert(!r.ok);
  assertEquals(r.error, "HTTP 200: 502 upstream");
});

Deno.test("parseDecisionReply rejects non-JSON, missing answers and unknown options", () => {
  assert(!parseDecisionReply(200, "<html>").ok);
  assert(!parseDecisionReply(200, JSON.stringify({ model: "m", answers: {} })).ok);
  const badChoice = structuredClone(VERIFIED_REPLY);
  badChoice.answers.grade.choice = "blocker";
  assert(!parseDecisionReply(200, JSON.stringify(badChoice)).ok);
  const badNoul = structuredClone(VERIFIED_REPLY);
  badNoul.answers.blocks.noul = 1.5;
  assert(!parseDecisionReply(200, JSON.stringify(badNoul)).ok);
});

Deno.test("parseDecisionReply truncates a non-JSON error body in its message", () => {
  const r = parseDecisionReply(503, "x".repeat(1000));
  assert(!r.ok);
  assert(r.error.startsWith("HTTP 503: "));
  assert(r.error.length < 260);
});

Deno.test("auroc is 1 for perfect separation, 0 for reversed, 0.5 for all ties, null when degenerate", () => {
  assertEquals(auroc([0.9, 0.8, 0.2, 0.1], [true, true, false, false]), 1);
  assertEquals(auroc([0.1, 0.2, 0.8, 0.9], [true, true, false, false]), 0);
  assertEquals(auroc([0.5, 0.5, 0.5], [true, false, false]), 0.5);
  assertEquals(auroc([0.1, 0.2], [true, true]), null);
  // One pair of four inverted: 3/4.
  assertEquals(auroc([0.9, 0.3, 0.4, 0.1], [true, true, false, false]), 0.75);
});

Deno.test("brier is the mean squared distance from the outcome", () => {
  assertAlmostEquals(brier([1, 0, 0.5], [true, false, true]), (0 + 0 + 0.25) / 3);
});

Deno.test("nearestRankPercentile", () => {
  assertEquals(nearestRankPercentile([5, 1, 3, 2, 4], 50), 3);
  assertEquals(nearestRankPercentile([5, 1, 3, 2, 4], 99), 5);
  assertEquals(nearestRankPercentile([], 50), null);
});

function reply(
  grade: string,
  disposition: string,
  blocks: number,
  gradeConf = 0.6,
  dispConf = 0.7,
): DecisionReply {
  return {
    model: "m",
    id: null,
    provider: null,
    grade: { choice: grade, probabilities: {}, confidence: gradeConf },
    disposition: { choice: disposition, probabilities: {}, confidence: dispConf },
    blocks,
    usage: { inputTokens: 100, outputTokens: 10, costUsd: 0.00001 },
  };
}

function item(
  id: string,
  severity: ItemResult["severity"],
  disposition: ItemResult["disposition"],
  r: DecisionReply | null,
  elapsedMs = 100,
): ItemResult {
  return {
    id,
    set: "A",
    severity,
    disposition,
    status: r ? 200 : 500,
    elapsedMs,
    reply: r ?? undefined,
    error: r ? undefined : "HTTP 500: boom",
  };
}

Deno.test("scoreResults: accuracy with a Wilson CI, confusion, demotions, disposition mapping, failures apart", () => {
  const s = scoreResults([
    item("a", "critical", "fixed", reply("critical", "fix", 0.9, 0.9, 0.9), 100),
    item("b", "critical", "fixed", reply("minor", "fix", 0.2, 0.3, 0.9), 300),
    item("c", "minor", "ruled_against", reply("minor", "defer", 0.1, 0.8, 0.2), 200),
    item("d", "important", "fixed", null, 50),
  ]);
  assertEquals(s.items, 4);
  assertEquals(s.answered, 3);
  assertEquals(s.failed, ["d"]);
  assertEquals(s.severity.accuracy.correct, 2);
  assertEquals(s.severity.accuracy.n, 3);
  assert(s.severity.accuracy.ci95.lower < 2 / 3 && s.severity.accuracy.ci95.upper > 2 / 3);
  assertEquals(s.severity.confusion.critical.minor, 1);
  assertEquals(s.severity.confusion.critical.critical, 1);
  assertEquals(s.severity.demotions, ["b"]);
  assertEquals(s.disposition.accuracy.correct, 2);
  assertEquals(s.disposition.confusion.ruled_against.deferred, 1);
  // blocks vs severity in {critical, important}: a=0.9 (pos), b=0.2 (pos), c=0.1 (neg) -> both pairs right.
  assertEquals(s.blocks.auroc, 1);
  assertEquals(s.blocks.positives, 2);
  // blocks >= 0.5 read as "critical or important": a (0.9, pos) right, b (0.2, pos) wrong, c (0.1, neg) right.
  assertEquals(s.blocks.accuracyAtHalf.correct, 2);
  assertEquals(s.blocks.accuracyAtHalf.n, 3);
  assertAlmostEquals(s.blocks.brier!, ((0.1) ** 2 + (0.8) ** 2 + (0.1) ** 2) / 3);
  // grade confidence: right a=0.9, c=0.8; wrong b=0.3 -> separates perfectly.
  assertEquals(s.severity.confidence.auroc, 1);
  assertAlmostEquals(s.severity.confidence.meanRight!, 0.85);
  assertAlmostEquals(s.severity.confidence.meanWrong!, 0.3);
  assertAlmostEquals(s.costUsd, 0.00003);
  assertEquals(s.inputTokens, 300);
  assertEquals(s.latencyMs, { p50: 100, p99: 300 });
});

Deno.test("scoreResults: a critical graded important is not a demotion; only critical -> minor is", () => {
  const s = scoreResults([item("a", "critical", "fixed", reply("important", "fix", 0.5))]);
  assertEquals(s.severity.demotions, []);
});
