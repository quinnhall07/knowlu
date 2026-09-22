// E1's statistics. Every number is built from `cloud/eval/score.ts`'s own `score()` — imported,
// never modified — so "weighted_exact" here means exactly what the eval suite means by it,
// including its unruled COST map: every cell it does not name (every `unsure` cell among them)
// costs the default 1, and a missing answer costs 3.
//
// The B1 gate. With 26 cases a paired comparison is small, so the rule is two tests that must BOTH
// pass, and the more conservative of the two binds:
//   1. the paired bootstrap 95% interval on (event-4 − event-3) weighted_exact lies wholly above 0;
//   2. the exact two-sided sign test over the cases where the arms disagree gives p < 0.05.
// The sign test is what binds at this size: with no losses at all it needs SIX cases where event-4
// scores higher (2 × 0.5^6 = 0.031; five gives 0.0625), and each loss raises that — see
// `minimumShippableWins`. In weighted_exact terms six wins is a gap of at least 6/26 × 1/3 ≈ 0.08
// when every win is the smallest possible one-cost step, and ≈ 0.23 when every win is a full
// miss-to-hit; in practice something like 0.15–0.20. Anything smaller is inside what 26 cases can
// explain by noise, and B1 says: record the number and close the idea.
import { type Case, score } from "../../../cloud/eval/score.ts";

export interface Interval {
  lower: number;
  upper: number;
}

/** Each case's own weighted_exact (0, 1/3, 2/3 or 1), by calling `score()` on that one case. */
export function perCaseCredit(cases: Case[], answers: Array<Record<string, unknown> | null>): number[] {
  return cases.map((c, i) => {
    const value = score("event", [c], [answers[i]])[0]?.value;
    if (value === null || value === undefined) throw new Error(`case ${i} labels no verdict`);
    return value;
  });
}

function mean(xs: number[]): number {
  return xs.reduce((s, x) => s + x, 0) / xs.length;
}

/** mulberry32: a small seeded PRNG, so a bootstrap interval is the same on every run. */
function rng(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** Percentile bootstrap 95% interval on the mean of `xs`. */
export function bootstrapMeanCI(xs: number[], resamples = 10_000, seed = 20260922): Interval {
  const next = rng(seed);
  const means: number[] = [];
  for (let r = 0; r < resamples; r++) {
    let sum = 0;
    for (let i = 0; i < xs.length; i++) sum += xs[Math.floor(next() * xs.length)];
    means.push(sum / xs.length);
  }
  means.sort((a, b) => a - b);
  return {
    lower: means[Math.floor(0.025 * (resamples - 1))],
    upper: means[Math.ceil(0.975 * (resamples - 1))],
  };
}

function choose(n: number, k: number): number {
  let c = 1;
  for (let i = 1; i <= k; i++) c = (c * (n - k + i)) / i;
  return c;
}

/** Exact two-sided sign test: p of a split at least this uneven among `wins + losses` fair coins. */
export function signTestP(wins: number, losses: number): number {
  const n = wins + losses;
  if (n === 0) return 1;
  const k = Math.min(wins, losses);
  let tail = 0;
  for (let i = 0; i <= k; i++) tail += choose(n, i);
  return Math.min(1, (2 * tail) / 2 ** n);
}

export interface Paired {
  meanDiff: number;
  ci: Interval;
  wins: number;
  losses: number;
  ties: number;
  signP: number;
}

/** event-4 (`b`) minus event-3 (`a`), case by case. */
export function pairedSummary(a: number[], b: number[], resamples = 10_000, seed = 20260922): Paired {
  if (a.length !== b.length) throw new Error("the arms scored different case counts");
  const d = b.map((x, i) => x - a[i]);
  const eps = 1e-9;
  const wins = d.filter((x) => x > eps).length;
  const losses = d.filter((x) => x < -eps).length;
  return {
    meanDiff: mean(d),
    ci: bootstrapMeanCI(d, resamples, seed),
    wins,
    losses,
    ties: d.length - wins - losses,
    signP: signTestP(wins, losses),
  };
}

export interface B1 {
  ship: boolean;
  reason: string;
  paired: Paired;
}

export function b1Verdict(a: number[], b: number[], resamples = 10_000, seed = 20260922): B1 {
  const paired = pairedSummary(a, b, resamples, seed);
  const ciAbove = paired.ci.lower > 0;
  const signOk = paired.meanDiff > 0 && paired.signP < 0.05;
  const ship = ciAbove && signOk;
  const reason = ship
    ? `SHIP: event-4 beats event-3 by ${paired.meanDiff.toFixed(3)}, 95% CI [${paired.ci.lower.toFixed(3)}, ` +
      `${paired.ci.upper.toFixed(3)}], sign test ${paired.wins}-${paired.losses} p=${paired.signP.toFixed(4)}`
    : `DO NOT SHIP — record the number and close the idea: difference ${paired.meanDiff.toFixed(3)}, ` +
      `95% CI [${paired.ci.lower.toFixed(3)}, ${paired.ci.upper.toFixed(3)}]` +
      `${ciAbove ? "" : " (includes or sits below 0)"}, sign test ${paired.wins}-${paired.losses} ` +
      `p=${paired.signP.toFixed(4)}${signOk ? "" : " (not < 0.05 in event-4's favour)"}`;
  return { ship, reason, paired };
}

/** The fewest cases event-4 must win, given `losses` cases it loses, for the sign test to pass. */
export function minimumShippableWins(losses: number, max = 1000): number {
  for (let w = losses + 1; w <= max; w++) if (signTestP(w, losses) < 0.05) return w;
  return Infinity;
}
