// TDD, per CLAUDE.md: written against `calibration.ts`'s intended shape before every export below
// existed. Every case here is synthetic (T6 brief: "test everything on synthetic rows... park it").
//
// Where a test needs a specific AUROC or calibration-error value, rows are built from an explicit
// rank assignment (`rowsFromRanks`) so the expected number is computed by hand from the same
// rank-sum identity `auc()` implements (Hanley & McNeil), not eyeballed from a script's output.
import { assert, assertAlmostEquals, assertEquals, assertThrows } from "@std/assert";
import {
  auc,
  AUROC_COSMETIC_MAX,
  AUROC_USABLE_MIN,
  aurocForWrong,
  bootstrapAucCi,
  buildCalibrationReport,
  buildGroupReport,
  type CalibrationRow,
  costOptimalThreshold,
  DECORATION_MAX_DISTINCT_VALUES,
  distinctConfidenceValues,
  equalMassBins,
  groupKey,
  groupRows,
  isDecorationByDistinctValues,
  kernelSmoothedCalibration,
  MIN_BIN_N,
  MIN_CLASS_N,
  parseCalibrationRows,
  silvermanBandwidth,
  suggestedBinCount,
  validateCalibrationRow,
} from "./calibration.ts";

// ---------------------------------------------------------------------------------------------
// Fixture builders
// ---------------------------------------------------------------------------------------------

function row(overrides: Partial<CalibrationRow> = {}): CalibrationRow {
  return {
    kind: "event",
    prompt_version: "event-1",
    prompt_hash: "hash-a",
    confidence: 0.9,
    verdict: "obligation",
    wrong: false,
    ...overrides,
  };
}

/**
 * `n` rows whose confidence is strictly decreasing in rank (`confidence = 1 - rank/(n+1)`, ranks
 * `1..n`, all distinct — no ties), with `wrong = true` for exactly the ranks in `wrongRanks`. Since
 * confidence strictly decreases as rank increases, `score = -confidence` strictly INCREASES with
 * rank — so the AUROC `aurocForWrong` computes on these rows equals the plain rank-sum AUC of
 * `wrongRanks` against `1..n`, computable by hand and asserted against `auc()` directly below.
 */
function rowsFromRanks(
  n: number,
  wrongRanks: ReadonlySet<number>,
  overrides: Partial<CalibrationRow> = {},
): CalibrationRow[] {
  return Array.from({ length: n }, (_, i) => {
    const rank = i + 1;
    return row({ confidence: 1 - rank / (n + 1), wrong: wrongRanks.has(rank), ...overrides });
  });
}

// ---------------------------------------------------------------------------------------------
// Row parsing
// ---------------------------------------------------------------------------------------------

Deno.test("validateCalibrationRow accepts a well-formed row", () => {
  assertEquals(validateCalibrationRow(row()), []);
});

Deno.test("validateCalibrationRow rejects a bad kind, an out-of-range confidence, and a non-boolean wrong", () => {
  const violations = validateCalibrationRow({
    kind: "nope",
    prompt_version: null,
    prompt_hash: null,
    confidence: 1.5,
    verdict: null,
    wrong: "yes",
  });
  assert(violations.some((v) => v.includes("kind")));
  assert(violations.some((v) => v.includes("confidence")));
  assert(violations.some((v) => v.includes("wrong")));
});

Deno.test("validateCalibrationRow rejects a non-object", () => {
  assertEquals(validateCalibrationRow("not a row"), ["the row is not an object"]);
});

Deno.test("parseCalibrationRows reads a JSON array", () => {
  const text = JSON.stringify([row({ confidence: 0.7 }), row({ confidence: 0.8 })]);
  const rows = parseCalibrationRows(text);
  assertEquals(rows.length, 2);
  assertEquals(rows[0].confidence, 0.7);
});

Deno.test("parseCalibrationRows reads JSONL, one row per line, skipping blank lines", () => {
  const text = `${JSON.stringify(row({ confidence: 0.7 }))}\n\n${JSON.stringify(row({ confidence: 0.8 }))}\n`;
  const rows = parseCalibrationRows(text);
  assertEquals(rows.length, 2);
  assertEquals(rows[1].confidence, 0.8);
});

Deno.test("parseCalibrationRows on empty or whitespace-only text returns []", () => {
  assertEquals(parseCalibrationRows(""), []);
  assertEquals(parseCalibrationRows("   \n  "), []);
});

Deno.test("parseCalibrationRows throws naming the array index on a bad element", () => {
  const text = JSON.stringify([row(), { kind: "task" }]);
  const err = assertThrows(() => parseCalibrationRows(text, "rows.json"));
  assert(err instanceof Error && err.message.startsWith("rows.json[1]:"), String(err));
});

Deno.test("parseCalibrationRows throws naming the line number on a bad JSONL line", () => {
  const text = `${JSON.stringify(row())}\nnot json\n`;
  const err = assertThrows(() => parseCalibrationRows(text, "rows.jsonl"));
  assert(err instanceof Error && err.message.startsWith("rows.jsonl:2:"), String(err));
});

Deno.test("parseCalibrationRows throws naming the line number on a structurally invalid JSONL row", () => {
  const text = `${JSON.stringify(row())}\n${JSON.stringify({ kind: "task" })}\n`;
  const err = assertThrows(() => parseCalibrationRows(text, "rows.jsonl"));
  assert(err instanceof Error && err.message.startsWith("rows.jsonl:2:"), String(err));
});

// ---------------------------------------------------------------------------------------------
// Grouping — always per kind; a prompt change is a new group
// ---------------------------------------------------------------------------------------------

Deno.test("groupKey differs on kind, prompt_version or prompt_hash alone", () => {
  const a = groupKey({ kind: "event", prompt_version: "event-1", prompt_hash: "h1" });
  const b = groupKey({ kind: "email", prompt_version: "event-1", prompt_hash: "h1" });
  const c = groupKey({ kind: "event", prompt_version: "event-2", prompt_hash: "h1" });
  const d = groupKey({ kind: "event", prompt_version: "event-1", prompt_hash: "h2" });
  const set = new Set([a, b, c, d]);
  assertEquals(set.size, 4);
});

Deno.test("groupRows splits by (kind, prompt_version, prompt_hash) and preserves row order within a group", () => {
  const rows = [
    row({ kind: "event", prompt_version: "event-1", confidence: 0.1 }),
    row({ kind: "email", prompt_version: "email-1", confidence: 0.2 }),
    row({ kind: "event", prompt_version: "event-1", confidence: 0.3 }),
    row({ kind: "event", prompt_version: "event-2", confidence: 0.4 }),
  ];
  const groups = groupRows(rows);
  assertEquals(groups.size, 3);
  const eventV1 = groups.get(groupKey({ kind: "event", prompt_version: "event-1", prompt_hash: "hash-a" }))!;
  assertEquals(eventV1.map((r) => r.confidence), [0.1, 0.3]);
});

// ---------------------------------------------------------------------------------------------
// Step 1: distinct confidence values
// ---------------------------------------------------------------------------------------------

Deno.test("distinctConfidenceValues counts unique confidence numbers", () => {
  const rows = [row({ confidence: 0.6 }), row({ confidence: 0.6 }), row({ confidence: 0.8 })];
  assertEquals(distinctConfidenceValues(rows), 2);
});

Deno.test("three or fewer distinct values is decoration; four is not", () => {
  assertEquals(DECORATION_MAX_DISTINCT_VALUES, 3);
  assert(isDecorationByDistinctValues(1));
  assert(isDecorationByDistinctValues(3));
  assert(!isDecorationByDistinctValues(4));
});

// ---------------------------------------------------------------------------------------------
// Step 2: AUROC — `auc()` against hand-computed rank-sum values
// ---------------------------------------------------------------------------------------------

Deno.test("auc: perfect separation (all positives rank above all negatives) is 1.0", () => {
  const scores = [1, 2, 3, 4, 5, 6];
  const positive = [false, false, false, true, true, true];
  assertEquals(auc(scores, positive), 1.0);
});

Deno.test("auc: perfect anti-separation (all positives rank below all negatives) is 0.0", () => {
  const scores = [1, 2, 3, 4, 5, 6];
  const positive = [true, true, true, false, false, false];
  assertEquals(auc(scores, positive), 0.0);
});

Deno.test("auc: identical score sets for both classes averages ties to 0.5", () => {
  // Ranks 1..20, positives on the odd ranks — see this file's header for the by-hand derivation:
  // rank-sum(positives) = 1+3+...+19 = 100, U = 100 - 10*11/2 = 45, AUC = 45 / (10*10) = 0.45.
  const scores = Array.from({ length: 20 }, (_, i) => i + 1);
  const positiveRanks = new Set([1, 3, 5, 7, 9, 11, 13, 15, 17, 19]);
  const positive = scores.map((s) => positiveRanks.has(s));
  assertAlmostEquals(auc(scores, positive), 0.45);
});

Deno.test("auc: a hand-picked rank assignment gives exactly 0.75", () => {
  // rank-sum(positives) = (1+5) + (12+13+...+19) = 6 + 124 = 130. nPos=nNeg=10.
  // U = 130 - 10*11/2 = 130 - 55 = 75. AUC = 75 / 100 = 0.75.
  const scores = Array.from({ length: 20 }, (_, i) => i + 1);
  const positiveRanks = new Set([1, 5, 12, 13, 14, 15, 16, 17, 18, 19]);
  const positive = scores.map((s) => positiveRanks.has(s));
  assertAlmostEquals(auc(scores, positive), 0.75);
});

Deno.test("auc: all one class is undefined (NaN)", () => {
  assert(Number.isNaN(auc([1, 2, 3], [true, true, true])));
  assert(Number.isNaN(auc([], [])));
});

Deno.test("MIN_CLASS_N is thirty, the T6 floor", () => {
  assertEquals(MIN_CLASS_N, 30);
});

Deno.test("aurocForWrong: fewer than MIN_CLASS_N in either class reports verdict 'undefined' and no CI", () => {
  const rows = rowsFromRanks(12, new Set([1, 2])); // 2 wrong, 10 correct — 2 < MIN_CLASS_N
  const result = aurocForWrong(rows);
  assertEquals(result.verdict, "undefined");
  assertEquals(result.value, null);
  assertEquals(result.ci95, null);
});

Deno.test("29 in a class is undefined and 30 is graded", () => {
  // 29 wrong, 30 correct (n=59) — one class one below the floor: undefined.
  const under = rowsFromRanks(59, new Set(Array.from({ length: 29 }, (_, i) => i + 1)));
  assertEquals(aurocForWrong(under).verdict, "undefined");

  // 30 wrong, 30 correct (n=60) — both classes exactly at the floor: graded, not undefined.
  const at = rowsFromRanks(60, new Set(Array.from({ length: 30 }, (_, i) => i + 1)));
  assert(aurocForWrong(at).verdict !== "undefined");
});

Deno.test("aurocForWrong: perfect separation is graded 'usable' and matches auc() on the same rows", () => {
  // 30 wrong, 30 correct (n=60), wrong ranks are the top 30 — every wrong rank outranks every
  // correct one, so AUC is exactly 1.0 regardless of class size.
  const rows = rowsFromRanks(60, new Set(Array.from({ length: 30 }, (_, i) => i + 31)));
  const result = aurocForWrong(rows, { samples: 50 });
  assertEquals(result.value, 1.0);
  assertEquals(result.verdict, "usable");
  assert(result.value! >= AUROC_USABLE_MIN);
  assert(!result.possiblyInverted);
});

Deno.test("aurocForWrong: 0.483 (below the cosmetic line) is graded 'cosmetic'", () => {
  // 30 wrong, 30 correct (n=60), wrong ranks are the odd ranks 1,3,...,59.
  // rank-sum(positives) = 1+3+...+59 = 30^2 = 900. nPos=nNeg=30.
  // U = 900 - 30*31/2 = 900 - 465 = 435. AUC = 435/900 = 29/60 ≈ 0.483.
  const rows = rowsFromRanks(60, new Set(Array.from({ length: 30 }, (_, i) => i * 2 + 1)));
  const result = aurocForWrong(rows, { samples: 50 });
  assertAlmostEquals(result.value!, 29 / 60);
  assertEquals(result.verdict, "cosmetic");
  assert(result.value! < AUROC_COSMETIC_MAX);
});

Deno.test("aurocForWrong: 0.733 (between the two lines) is graded 'marginal', not 'usable'", () => {
  // 30 wrong, 30 correct (n=60): wrong ranks are the lowest 8 {1..8} plus the highest 22 {39..60}.
  // rank-sum(positives) = (1+...+8) + (39+...+60) = 36 + 1089 = 1125. nPos=nNeg=30.
  // U = 1125 - 30*31/2 = 1125 - 465 = 660. AUC = 660/900 ≈ 0.733 — inside the marginal band.
  const rows = rowsFromRanks(
    60,
    new Set([
      ...Array.from({ length: 8 }, (_, i) => i + 1),
      ...Array.from({ length: 22 }, (_, i) => i + 39),
    ]),
  );
  const result = aurocForWrong(rows, { samples: 50 });
  assertAlmostEquals(result.value!, 660 / 900);
  assertEquals(result.verdict, "marginal");
});

Deno.test("aurocForWrong: possiblyInverted flags a clearly-below-chance AUROC", () => {
  // All 30 wrong ranks are the 30 LOWEST -confidence ranks (n=60) -> AUC 0.0, well under
  // 1 - 0.80 = 0.20.
  const rows = rowsFromRanks(60, new Set(Array.from({ length: 30 }, (_, i) => i + 1)));
  const result = aurocForWrong(rows, { samples: 50 });
  assertEquals(result.value, 0.0);
  assert(result.possiblyInverted);
});

Deno.test("bootstrapAucCi is deterministic under a fixed seed and brackets the point estimate loosely", () => {
  const rows = rowsFromRanks(40, new Set([21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35]));
  const scores = rows.map((r) => -r.confidence);
  const labels = rows.map((r) => r.wrong);
  const a = bootstrapAucCi(scores, labels, { seed: 7, samples: 200 });
  const b = bootstrapAucCi(scores, labels, { seed: 7, samples: 200 });
  assertEquals(a, b);
  assert(a.ci !== null);
  const [lo, hi] = a.ci!;
  assert(lo <= hi);
  assert(lo >= 0 && hi <= 1);
});

Deno.test("bootstrapAucCi with a different seed can give a different interval", () => {
  const rows = rowsFromRanks(40, new Set([21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35]));
  const scores = rows.map((r) => -r.confidence);
  const labels = rows.map((r) => r.wrong);
  const a = bootstrapAucCi(scores, labels, { seed: 1, samples: 200 });
  const b = bootstrapAucCi(scores, labels, { seed: 2, samples: 200 });
  // Not asserting inequality (two seeds could coincidentally agree); asserting both are usable.
  assert(a.ci !== null && b.ci !== null);
});

// ---------------------------------------------------------------------------------------------
// Step 3: calibration
// ---------------------------------------------------------------------------------------------

Deno.test("silvermanBandwidth is positive and floored at 0.02 for near-constant confidence", () => {
  const constant = Array.from({ length: 50 }, () => 0.9);
  assertEquals(silvermanBandwidth(constant), 0.02);
  const spread = Array.from({ length: 50 }, (_, i) => i / 50);
  assert(silvermanBandwidth(spread) > 0.02);
});

Deno.test("kernelSmoothedCalibration: a group whose bucketed accuracy matches its stated confidence has low calibration error", () => {
  // Eight buckets, confidence 0.60..0.95 in steps of 0.05, wrong-fraction = 1 - confidence exactly
  // (20 rows per bucket) — a textbook well-calibrated field.
  const rows: CalibrationRow[] = [];
  for (let level = 0; level < 8; level++) {
    const confidence = 0.6 + level * 0.05;
    const n = 20;
    const wrongCount = Math.round((1 - confidence) * n);
    for (let i = 0; i < n; i++) rows.push(row({ confidence, wrong: i < wrongCount }));
  }
  const result = kernelSmoothedCalibration(rows);
  assert(result.calibrationError < 0.05, `expected a low calibration error, got ${result.calibrationError}`);
});

Deno.test("kernelSmoothedCalibration: two badly mismatched confidence/accuracy clusters have high calibration error", () => {
  // Bucket A: confidence 0.55, always wrong (0% accuracy vs a stated 55%).
  // Bucket B: confidence 0.95, always correct (100% accuracy vs a stated 95%, nearly right).
  // The clusters are far apart relative to Silverman's bandwidth here, so cross-bucket smoothing
  // is negligible and bucket A alone should dominate the mean error.
  const rows: CalibrationRow[] = [
    ...Array.from({ length: 20 }, () => row({ confidence: 0.55, wrong: true })),
    ...Array.from({ length: 20 }, () => row({ confidence: 0.95, wrong: false })),
  ];
  const result = kernelSmoothedCalibration(rows);
  assert(result.calibrationError > 0.15, `expected a high calibration error, got ${result.calibrationError}`);
});

Deno.test("suggestedBinCount matches the brief's three reference points loosely: none at 50, a few at 200, ten at 1000", () => {
  assertEquals(suggestedBinCount(50), 1);
  assert(suggestedBinCount(200) >= 3 && suggestedBinCount(200) <= 5);
  assertEquals(suggestedBinCount(1000), 10);
});

Deno.test("equalMassBins splits rows into roughly equal groups, undecorated once each bin clears MIN_BIN_N", () => {
  assertEquals(MIN_BIN_N, 30);
  const rows = Array.from({ length: 200 }, (_, i) => row({ confidence: i / 200, wrong: i % 5 === 0 }));
  const bins = equalMassBins(rows, 4); // 50 rows/bin, clear of the 30-row floor
  assertEquals(bins.length, 4);
  assertEquals(bins.reduce((sum, b) => sum + b.n, 0), 200);
  assert(bins.every((b) => !b.decoration));
});

Deno.test("equalMassBins: a bin under 30 rows is flagged decoration", () => {
  const rows = Array.from({ length: 40 }, (_, i) => row({ confidence: i / 40, wrong: false }));
  const bins = equalMassBins(rows, 2); // 20 rows per bin
  assert(bins.every((b) => b.n === 20 && b.decoration));
});

Deno.test("equalMassBins on an empty input returns no bins", () => {
  assertEquals(equalMassBins([], 5), []);
});

// ---------------------------------------------------------------------------------------------
// The cost-optimal threshold: required parameters, no default, matches the background note's
// worked example (§4: obligation->drop costs 3, every other named mistake costs 1, t* = 1/(1+3)).
// ---------------------------------------------------------------------------------------------

Deno.test("costOptimalThreshold matches Elkan's formula on the background note's own worked example", () => {
  assertAlmostEquals(costOptimalThreshold(1, 3), 0.25);
});

Deno.test("costOptimalThreshold rejects a non-positive cost", () => {
  assertThrows(() => costOptimalThreshold(0, 1));
  assertThrows(() => costOptimalThreshold(1, -1));
});

// ---------------------------------------------------------------------------------------------
// The report: the full stop-early cascade and B2's machine-readable outcome
// ---------------------------------------------------------------------------------------------

Deno.test("buildGroupReport stops at step 1 when the field is decoration by distinct value count", () => {
  const rows = [
    row({ confidence: 0.6, wrong: true }),
    row({ confidence: 0.7, wrong: false }),
    row({ confidence: 0.6, wrong: false }),
    row({ confidence: 0.7, wrong: true }),
  ];
  const report = buildGroupReport(rows);
  assertEquals(report.stoppedAt, 1);
  assertEquals(report.discrimination.decoration, true);
  assertEquals(report.auroc, null);
  assertEquals(report.calibration, null);
  assertEquals(report.b2Outcome, "does_not_rank");
  assert(report.nextStep.includes("T5 and T7 both vanish"));
});

Deno.test("buildGroupReport reports 'insufficient_data' when a class is too small to grade, without calling it decoration or cosmetic", () => {
  const rows = rowsFromRanks(12, new Set([1, 2])); // 2 wrong < MIN_CLASS_N, but 5+ distinct values
  const report = buildGroupReport(rows);
  assertEquals(report.stoppedAt, 2);
  assertEquals(report.discrimination.decoration, false);
  assertEquals(report.b2Outcome, "insufficient_data");
  assertEquals(report.calibration, null);
});

Deno.test("buildGroupReport stops at step 2, 'does_not_rank', on a cosmetic AUROC", () => {
  // Same 30/30 construction as the "0.483 (below the cosmetic line)" aurocForWrong test above.
  const rows = rowsFromRanks(60, new Set(Array.from({ length: 30 }, (_, i) => i * 2 + 1))); // AUROC ≈ 0.483
  const report = buildGroupReport(rows);
  assertEquals(report.stoppedAt, 2);
  assertEquals(report.b2Outcome, "does_not_rank");
  assertEquals(report.calibration, null);
  assert(report.nextStep.includes("cosmetic"));
});

Deno.test("buildGroupReport stops at step 2, 'does_not_rank', on a marginal (0.70-0.80) AUROC — the middle band does not clear the bar", () => {
  // Same 30/30 construction as the "0.733 (between the two lines)" aurocForWrong test above.
  const rows = rowsFromRanks(
    60,
    new Set([
      ...Array.from({ length: 8 }, (_, i) => i + 1),
      ...Array.from({ length: 22 }, (_, i) => i + 39),
    ]),
  ); // AUROC ≈ 0.733
  const report = buildGroupReport(rows);
  assertEquals(report.auroc!.verdict, "marginal");
  assertEquals(report.stoppedAt, 2);
  assertEquals(report.b2Outcome, "does_not_rank");
  assert(report.nextStep.includes("undefined middle band"));
});

Deno.test("buildGroupReport reaches step 3 and reports 'ranks_and_calibrated' on a usable, well-calibrated field", () => {
  // Two clusters at the extremes, each spread over 4 distinct confidence values (8 distinct
  // overall): stated confidence 0.01-0.04, ALWAYS wrong (accuracy ~0%, close to the stated near-0
  // confidence); stated confidence 0.97-1.00, ALWAYS correct (accuracy 100%, close to the stated
  // near-1 confidence). The clusters are far enough apart, relative to the resulting Silverman
  // bandwidth, that ranking is a clean separation (AUROC 1.0) and each point's own local estimate
  // stays near its cluster's true accuracy (mean absolute error ~0.02, comfortably under the
  // 0.05 default threshold) — a textbook "the field ranks and is roughly calibrated" case.
  const rows: CalibrationRow[] = [];
  for (const confidence of [0.01, 0.02, 0.03, 0.04]) {
    for (let i = 0; i < 10; i++) rows.push(row({ confidence, wrong: true }));
  }
  for (const confidence of [0.97, 0.98, 0.99, 1.0]) {
    for (let i = 0; i < 10; i++) rows.push(row({ confidence, wrong: false }));
  }
  const report = buildGroupReport(rows, { ciOptions: { samples: 100 } });
  assertEquals(report.discrimination.decoration, false);
  assertEquals(report.stoppedAt, 3);
  assert(report.auroc!.value! >= AUROC_USABLE_MIN, `expected a usable AUROC, got ${report.auroc!.value}`);
  assert(report.calibration !== null);
  assert(
    report.calibration!.kernel.calibrationError < report.calibration!.threshold,
    `expected calibration error under ${report.calibration!.threshold}, got ${
      report.calibration!.kernel.calibrationError
    }`,
  );
  assertEquals(report.calibration!.roughlyCalibrated, true);
  assertEquals(report.b2Outcome, "ranks_and_calibrated");
  assert(report.nextStep.includes("go straight to T5"));
});

Deno.test("buildGroupReport reaches step 3 and reports 'ranks_but_miscalibrated' on a usable but badly calibrated field, naming T7 without implementing it", () => {
  // Two 30-row clusters (clear of the MIN_CLASS_N=30 floor), each jittered across 5 distinct
  // confidence values (10 distinct overall, clear of the 3-value decoration line) so this exercises
  // calibration rather than tripping step 1 or step 2: a low cluster (~0.50-0.58) that is ALWAYS
  // wrong against its own stated confidence, and a high cluster (~0.90-0.98) that is always correct
  // and close to well-calibrated. The gap between clusters (~0.35) is many multiples of the
  // resulting Silverman bandwidth, so cross-cluster smoothing is negligible and the low cluster's
  // ~0.5 point calibration error dominates the mean.
  const rows: CalibrationRow[] = [
    ...Array.from({ length: 30 }, (_, i) => row({ confidence: 0.50 + (i % 5) * 0.02, wrong: true })),
    ...Array.from({ length: 30 }, (_, i) => row({ confidence: 0.90 + (i % 5) * 0.02, wrong: false })),
  ];
  const report = buildGroupReport(rows, { ciOptions: { samples: 100 } });
  assertEquals(report.discrimination.decoration, false);
  assertEquals(report.stoppedAt, 3);
  assert(report.auroc!.value! >= AUROC_USABLE_MIN, `expected a usable AUROC, got ${report.auroc!.value}`);
  assertEquals(report.b2Outcome, "ranks_but_miscalibrated");
  assertEquals(report.calibration!.roughlyCalibrated, false);
  assert(report.nextStep.includes("T7"));
  assert(report.nextStep.includes("Dirichlet"));
});

Deno.test("buildGroupReport carries kind/prompt_version/prompt_hash and a verdict histogram through unchanged", () => {
  const rows = rowsFromRanks(20, new Set([11, 12, 13, 14, 15, 16, 17, 18, 19, 20]), {
    kind: "email",
    prompt_version: "email-3",
    prompt_hash: "abc123",
    verdict: "task",
  });
  const report = buildGroupReport(rows);
  assertEquals(report.kind, "email");
  assertEquals(report.prompt_version, "email-3");
  assertEquals(report.prompt_hash, "abc123");
  assertEquals(report.verdictCounts, { task: 20 });
});

Deno.test("buildGroupReport throws on an empty group rather than fabricating a kind", () => {
  assertThrows(() => buildGroupReport([]));
});

Deno.test("buildCalibrationReport returns one report per group across kinds and prompt versions", () => {
  const rows = [
    ...rowsFromRanks(20, new Set([1, 3, 5, 7, 9, 11, 13, 15, 17, 19]), {
      kind: "event",
      prompt_version: "event-1",
    }),
    ...rowsFromRanks(20, new Set([1, 3, 5, 7, 9, 11, 13, 15, 17, 19]), {
      kind: "email",
      prompt_version: "email-1",
    }),
    ...rowsFromRanks(20, new Set([1, 3, 5, 7, 9, 11, 13, 15, 17, 19]), {
      kind: "event",
      prompt_version: "event-2",
    }),
  ];
  const reports = buildCalibrationReport(rows);
  assertEquals(reports.length, 3);
  const keys = reports.map((r) => `${r.kind}/${r.prompt_version}`).sort();
  assertEquals(keys, ["email/email-1", "event/event-1", "event/event-2"]);
});
