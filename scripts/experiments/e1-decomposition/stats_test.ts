import { assert, assertAlmostEquals, assertEquals } from "@std/assert";
import type { Case } from "../../../cloud/eval/score.ts";
import {
  b1Verdict,
  bootstrapMeanCI,
  minimumShippableWins,
  pairedSummary,
  perCaseCredit,
  signTestP,
} from "./stats.ts";

const ev = (verdict: string): Case => ({ kind: "event", theirs: { verdict } });

Deno.test("perCaseCredit is score.ts's weighted_exact on one case at a time, and averages to the batch value", () => {
  const cases = [ev("obligation"), ev("obligation"), ev("drop"), ev("unsure"), ev("opportunity")];
  const answers = [
    { verdict: "obligation" },
    { verdict: "drop" },
    { verdict: "opportunity" },
    { verdict: "drop" },
    null,
  ];
  const credit = perCaseCredit(cases, answers);
  // obligation->obligation costs 0; obligation->drop 3; drop->opportunity 1; unsure->drop is an
  // unnamed cell and falls through to the default 1; a missing answer costs 3.
  assertEquals(credit, [1, 0, 2 / 3, 2 / 3, 0]);
});

Deno.test("bootstrapMeanCI is deterministic for a seed and brackets the mean", () => {
  const xs = [1, 0, 1, 1, 2 / 3, 0, 1, 1, 1 / 3, 1];
  const a = bootstrapMeanCI(xs, 2000, 7);
  const b = bootstrapMeanCI(xs, 2000, 7);
  assertEquals(a, b);
  const mean = xs.reduce((s, x) => s + x, 0) / xs.length;
  assert(a.lower <= mean && mean <= a.upper, JSON.stringify(a));
});

Deno.test("bootstrapMeanCI of a constant is that constant", () => {
  assertEquals(bootstrapMeanCI([1, 1, 1], 500, 1), { lower: 1, upper: 1 });
});

Deno.test("signTestP is the exact two-sided binomial p over the discordant cases", () => {
  assertEquals(signTestP(0, 0), 1);
  assertAlmostEquals(signTestP(6, 0), 2 / 64);
  assertAlmostEquals(signTestP(5, 0), 2 / 32);
  assertAlmostEquals(signTestP(0, 6), 2 / 64);
  assertEquals(signTestP(3, 3), 1);
});

Deno.test("pairedSummary counts wins, losses and ties of event-4 over event-3", () => {
  const s = pairedSummary([1, 0, 1, 2 / 3], [1, 1, 0, 1], 1000, 3);
  assertEquals([s.wins, s.losses, s.ties], [2, 1, 1]);
  assertAlmostEquals(s.meanDiff, (0 + 1 - 1 + 1 / 3) / 4);
});

Deno.test("B1: a decomposition that wins cleanly on enough cases ships", () => {
  const a = new Array(26).fill(1);
  const b = [...a];
  for (let i = 0; i < 8; i++) a[i] = 0; // event-3 misses 8 cases event-4 gets right
  const v = b1Verdict(a, b, 5000, 11);
  assertEquals(v.ship, true, v.reason);
});

Deno.test("B1: a tie never ships, and neither does a small win the seed can explain by noise", () => {
  const same = new Array(26).fill(2 / 3);
  assertEquals(b1Verdict(same, same, 2000, 1).ship, false);
  const a = new Array(26).fill(1);
  const b = [...a];
  for (let i = 0; i < 3; i++) a[i] = 0; // three clean wins: p = 0.25
  assertEquals(b1Verdict(a, b, 2000, 1).ship, false);
});

Deno.test("B1: a loss never ships", () => {
  const a = new Array(26).fill(1);
  const b = [...a];
  for (let i = 0; i < 10; i++) b[i] = 0;
  assertEquals(b1Verdict(a, b, 2000, 1).ship, false);
});

Deno.test("B1: the sign test's direction is wins against losses, not the mean — a few large wins against more small losses never ships", () => {
  // event-4 wins 6 cases by a full point and loses 17 by a third: the mean favours event-4
  // (6 - 17/3 > 0), and the sign test is significant (p ~ 0.035) — but AGAINST event-4, because
  // far more cases went the other way. Its direction must come from wins vs losses, not the mean.
  const a = new Array(26).fill(2 / 3);
  const b = [...a];
  for (let i = 0; i < 6; i++) {
    a[i] = 0;
    b[i] = 1;
  }
  for (let i = 6; i < 23; i++) b[i] = 1 / 3;
  const v = b1Verdict(a, b, 2000, 1);
  assert(v.paired.meanDiff > 0);
  assertEquals([v.paired.wins, v.paired.losses], [6, 17]);
  assert(v.paired.signP < 0.05, String(v.paired.signP));
  assertEquals(v.ship, false);
  assert(v.reason.includes("not < 0.05 in event-4's favour"), v.reason);
});

Deno.test("B1: many wins with a negative mean still does not ship — both conditions must hold", () => {
  const a = new Array(26).fill(1 / 3);
  const b = [...a];
  // 12 small wins (+1/3 each, +4 in all) against 8 heavier losses (2 x -1 and 6 x -2/3, -6 in all).
  for (let i = 0; i < 12; i++) b[i] = 2 / 3;
  for (const i of [12, 13]) {
    a[i] = 1;
    b[i] = 0;
  }
  for (let i = 14; i < 20; i++) {
    a[i] = 1;
    b[i] = 1 / 3;
  }
  const v = b1Verdict(a, b, 2000, 1);
  assert(v.paired.meanDiff < 0, String(v.paired.meanDiff));
  assertEquals(v.ship, false);
});

Deno.test("minimumShippableWins: six clean wins with no losses is the floor the sign test sets", () => {
  assertEquals(minimumShippableWins(0), 6);
  assert(minimumShippableWins(1) > 6);
  assert(minimumShippableWins(2) > minimumShippableWins(1));
});
