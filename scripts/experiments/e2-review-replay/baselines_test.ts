import { assert, assertAlmostEquals, assertEquals } from "@std/assert";
import {
  cosineSimilarity,
  type LabeledItem,
  leaveOneOutLexicalKNN,
  leaveOneOutMajorityClass,
  tokenize,
  wilsonInterval,
} from "./baselines.ts";

Deno.test("wilsonInterval brackets 0.5 widely at n=1 and narrows as n grows", () => {
  const small = wilsonInterval(1, 2);
  const large = wilsonInterval(500, 1000);
  assert(small.upper - small.lower > large.upper - large.lower);
  assert(small.lower <= 0.5 && small.upper >= 0.5);
});

Deno.test("wilsonInterval handles the n=0 edge without dividing by zero", () => {
  assertEquals(wilsonInterval(0, 0), { lower: 0, upper: 0 });
});

Deno.test("wilsonInterval is within [0,1] and centred near the observed rate at large n", () => {
  const iv = wilsonInterval(760, 1000);
  assert(iv.lower >= 0 && iv.upper <= 1);
  assert(iv.lower < 0.76 && iv.upper > 0.76);
  assertAlmostEquals((iv.lower + iv.upper) / 2, 0.76, 0.03);
});

Deno.test("leaveOneOutMajorityClass scores 42% on a 24-item, 10-minor/8-important/6-critical mix", () => {
  const items: LabeledItem[] = [
    ...Array(6).fill(0).map((_, i) => ({ id: `c${i}`, text: "x", label: "critical" })),
    ...Array(8).fill(0).map((_, i) => ({ id: `i${i}`, text: "x", label: "important" })),
    ...Array(10).fill(0).map((_, i) => ({ id: `m${i}`, text: "x", label: "minor" })),
  ];
  const result = leaveOneOutMajorityClass(items);
  assertEquals(result.n, 24);
  // Leaving one minor out still leaves minor as the mode of the rest (9 vs 8 vs 6), so every minor
  // is correctly predicted; every critical/important item is not (mode of the rest is still minor).
  assertEquals(result.correct, 10);
  assertAlmostEquals(result.accuracy, 10 / 24, 1e-9);
});

Deno.test("leaveOneOutMajorityClass is 100% on a corpus that is all one label", () => {
  const items: LabeledItem[] = Array(5).fill(0).map((_, i) => ({
    id: `x${i}`,
    text: "x",
    label: "fixed",
  }));
  const result = leaveOneOutMajorityClass(items);
  assertEquals(result.correct, 5);
  assertEquals(result.accuracy, 1);
});

Deno.test("tokenize drops stopwords, short tokens and inline-code spans", () => {
  const tokens = tokenize("The `handler.ts:148` reads a body of literal null and the check fails");
  assertEquals(tokens.includes("the"), false);
  assertEquals(tokens.includes("handler"), false); // was inside a code span, stripped whole
  assertEquals(tokens.includes("literal"), true);
  assertEquals(tokens.includes("null"), true);
  assertEquals(tokens.includes("fails"), true);
});

Deno.test("cosineSimilarity is 1 for identical vectors and 0 for disjoint ones", () => {
  const a = new Map([["x", 1], ["y", 2]]);
  const b = new Map([["x", 1], ["y", 2]]);
  assertAlmostEquals(cosineSimilarity(a, b), 1, 1e-9);
  const c = new Map([["z", 1]]);
  assertEquals(cosineSimilarity(a, c), 0);
});

Deno.test("leaveOneOutLexicalKNN recovers an obvious lexical split", () => {
  const items: LabeledItem[] = [
    { id: "a1", text: "the listener never checks the deadline and spins forever", label: "critical" },
    { id: "a2", text: "the deadline check never runs on the busy path so it spins", label: "critical" },
    { id: "a3", text: "the loop never re-tests the deadline and can spin past it", label: "critical" },
    { id: "b1", text: "a line cite is off by one in the spec cross reference", label: "minor" },
    { id: "b2", text: "the cross reference cites the wrong line number in the spec", label: "minor" },
    { id: "b3", text: "one more line cite drifted by a line in the spec text", label: "minor" },
  ];
  const result = leaveOneOutLexicalKNN(items, 1);
  assert(
    result.correct >= 5,
    `expected the obvious lexical split to mostly recover, got ${result.correct}/6`,
  );
});

Deno.test("leaveOneOutLexicalKNN never lets the held-out item's own text vote for itself", () => {
  // A single wildly distinct item should not trivially match itself — with it held out, its
  // nearest neighbour is necessarily one of the other, unrelated items.
  const items: LabeledItem[] = [
    { id: "odd", text: "zzqx wobble frobnicate unique nonsense tokens nowhere else", label: "critical" },
    { id: "n1", text: "ordinary review prose about a migration and a trigger function", label: "minor" },
    { id: "n2", text: "ordinary review prose about a listener and a deadline check", label: "minor" },
  ];
  const result = leaveOneOutLexicalKNN(items, 1);
  assertEquals(result.n, 3);
  // The "odd" item cannot match itself once held out, so it is necessarily misclassified as minor.
  assertEquals(result.correct, 2);
});
