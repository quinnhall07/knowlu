// T6 — the calibration harness's metrics module (stream J plan, `.superpowers/sdd/
// 2026-09-22-judgment-quality-plan/t6-brief.md`; background: `docs/notes/2026-09-22-
// confidence-calibration-and-the-floor.md`). Pure: no database, no network, no file I/O — every
// function here takes rows and returns numbers. `calibration_cli.ts` is the only file that reads a
// file or prints anything.
//
// **The question.** Does our `confidence` mean anything? Answered per `(kind, prompt_version,
// prompt_hash)` group, in the brief's own order, stopping early the moment an answer is bad:
//
//   1. distinct confidence values   <=3 is decoration (a model observed emitting exactly three
//                                    values had near-useless ranking ability)
//   2. AUROC for "was this wrong"   >=0.80 usable ranker, <0.70 cosmetic gate
//   3. calibration                  only computed if 1 and 2 both clear their bar
//
// **BRANCH B2** (the brief's own three outcomes, reported verbatim as `b2Outcome` below) plus one
// state the brief does not name: `"insufficient_data"`, for a group too small to say anything about
// safely (a floor of our own choosing — see `MIN_CLASS_N`). That is not a fourth branch of the
// brief's B2; it is this harness declining to force a real-but-noisy answer into one of the three.
//
// **What T6 explicitly does NOT do**, per the dispatch: no fitted scorer, no Dirichlet/vector
// scaling, no isotonic fitting (that is T7, and only for the `"ranks_but_miscalibrated"` branch);
// no cost-derived threshold with a baked-in default (`costOptimalThreshold` below takes both costs
// as required parameters and ships with no default value, and nothing in `buildCalibrationReport`
// calls it — a caller invokes it by hand once it has real costs to give it).
import type { Kind } from "../supabase/functions/_shared/judge_validate.ts";

export type { Kind };

// -------------------------------------------------------------------------------------------
// The row shape: what `calibration_query.sql` produces and what the CLI reads from a file.
// -------------------------------------------------------------------------------------------

export interface CalibrationRow {
  kind: Kind;
  /** Null when a judgment predates `prompt_version` being logged, or the column is absent. */
  prompt_version: string | null;
  prompt_hash: string | null;
  /** The model's self-reported probability, clamped to [0, 1] by `judge_validate.ts::validate`
   * before it is ever written — a row outside that range is a malformed input, not a rare judgment,
   * and `parseCalibrationRows` rejects it rather than silently clamping a second time. */
  confidence: number;
  /** `fields ->> 'verdict'` (event) or `fields ->> 'tier'` (email), coalesced; null for task, whose
   * decision is three separate labelled fields rather than one verdict key. Carried through for
   * transparency (`CalibrationGroupReport.verdictCounts`) and not used by any statistic below. */
  verdict: string | null;
  /** Whether a correction disagreed with this judgment. See `calibration_query.sql`'s header for
   * why "a matching `corrections` row exists" is the whole of this definition. */
  wrong: boolean;
}

const KINDS: readonly Kind[] = ["task", "event", "email"];

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** The violations in `value`, empty when it is a valid `CalibrationRow`. Mirrors `schema.ts`'s
 * `validateSeedRecord` in shape (explicit, enumerable, never throws on its own) so a caller can
 * decide whether to collect every bad line or fail on the first, the way `loadSeed` does. */
export function validateCalibrationRow(value: unknown): string[] {
  const violations: string[] = [];
  if (!isPlainObject(value)) return ["the row is not an object"];

  const kind = value.kind;
  if (typeof kind !== "string" || !KINDS.includes(kind as Kind)) {
    violations.push(`kind must be one of ${KINDS.join(", ")}; got ${JSON.stringify(kind)}`);
  }

  for (const key of ["prompt_version", "prompt_hash", "verdict"] as const) {
    const v = value[key];
    if (v !== null && typeof v !== "string") {
      violations.push(`${key} must be a string or null; got ${JSON.stringify(v)}`);
    }
  }

  const confidence = value.confidence;
  if (typeof confidence !== "number" || !Number.isFinite(confidence) || confidence < 0 || confidence > 1) {
    violations.push(`confidence must be a number in [0, 1]; got ${JSON.stringify(confidence)}`);
  }

  if (typeof value.wrong !== "boolean") {
    violations.push(`wrong must be a boolean; got ${JSON.stringify(value.wrong)}`);
  }

  return violations;
}

/**
 * Rows from a JSON array (`[{...}, {...}]`) or JSONL (one object per line) — `text.trim()`
 * starting with `[` picks the array branch, matching how `calibration_query.sql`'s two documented
 * export shapes (a client library's `json_agg` array, or a streaming NDJSON cursor) both land.
 * An invalid row throws naming its position (array index or line number, 1-based either way) —
 * the same "never drop a bad line silently" rule `loader.ts::loadSeed` applies to the seed corpus,
 * for the same reason: a dropped row is a hole in the answer nobody sees.
 */
export function parseCalibrationRows(text: string, sourceName = "<input>"): CalibrationRow[] {
  const trimmed = text.trim();
  if (trimmed === "") return [];

  if (trimmed.startsWith("[")) {
    let parsed: unknown;
    try {
      parsed = JSON.parse(trimmed);
    } catch (e) {
      throw new Error(`${sourceName}: not valid JSON (${e instanceof Error ? e.message : String(e)})`);
    }
    if (!Array.isArray(parsed)) throw new Error(`${sourceName}: top-level JSON value is not an array`);
    return parsed.map((row, i) => {
      const violations = validateCalibrationRow(row);
      if (violations.length > 0) {
        throw new Error(`${sourceName}[${i}]: ${violations.join("; ")}`);
      }
      return row as CalibrationRow;
    });
  }

  const rows: CalibrationRow[] = [];
  const lines = text.split("\n");
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (line.trim() === "") continue;
    const lineNo = i + 1;
    let parsed: unknown;
    try {
      parsed = JSON.parse(line);
    } catch (e) {
      throw new Error(
        `${sourceName}:${lineNo}: not valid JSON (${e instanceof Error ? e.message : String(e)})`,
      );
    }
    const violations = validateCalibrationRow(parsed);
    if (violations.length > 0) {
      throw new Error(`${sourceName}:${lineNo}: ${violations.join("; ")}`);
    }
    rows.push(parsed as CalibrationRow);
  }
  return rows;
}

// -------------------------------------------------------------------------------------------
// Grouping: always per kind (§ brief), and "re-fit on every prompt change" — group by
// (kind, prompt_version, prompt_hash) so a prompt edit's invalidated fit is a NEW group, never
// silently pooled with the fit it invalidated.
// -------------------------------------------------------------------------------------------

export function groupKey(row: Pick<CalibrationRow, "kind" | "prompt_version" | "prompt_hash">): string {
  return `${row.kind}\u0000${row.prompt_version ?? ""}\u0000${row.prompt_hash ?? ""}`;
}

export function groupRows(rows: readonly CalibrationRow[]): Map<string, CalibrationRow[]> {
  const groups = new Map<string, CalibrationRow[]>();
  for (const row of rows) {
    const key = groupKey(row);
    const bucket = groups.get(key);
    if (bucket) bucket.push(row);
    else groups.set(key, [row]);
  }
  return groups;
}

// -------------------------------------------------------------------------------------------
// Step 1: distinct confidence values. "Three or fewer and the field is decoration" (T6 brief) —
// one study found a model emitting exactly three values, near-useless as a ranker.
// -------------------------------------------------------------------------------------------

export const DECORATION_MAX_DISTINCT_VALUES = 3;

export function distinctConfidenceValues(rows: readonly CalibrationRow[]): number {
  return new Set(rows.map((r) => r.confidence)).size;
}

export function isDecorationByDistinctValues(distinctCount: number): boolean {
  return distinctCount <= DECORATION_MAX_DISTINCT_VALUES;
}

// -------------------------------------------------------------------------------------------
// Step 2: AUROC for "was this judgment wrong".
//
// Score = `-confidence`, label = `wrong`: a well-behaved confidence field puts its LOWEST values
// on WRONG judgments, so the wrong class should rank higher on `-confidence`, and AUROC should sit
// near 1. An AUROC well below 0.5 is not "no signal" — it is confidence running backwards (the
// Nyckel study's "raw relationship was inverted" finding, background note §2) — which is still
// unusable AS THE FIELD IS TODAY, so it is still graded "cosmetic" by the same <0.70 line; the
// `possiblyInverted` flag on the result exists only to say which failure this was.
// -------------------------------------------------------------------------------------------

export const AUROC_USABLE_MIN = 0.80;
export const AUROC_COSMETIC_MAX = 0.70;
/** Below this many examples in EITHER class, the point estimate is treated as too noisy to grade
 * at all (`verdict: "undefined"`) rather than graded and possibly wrong.
 *
 * The judgment-quality plan's T6 (`docs/plans/2026-09-22-judgment-quality-plan.md:263`) says, of
 * CALIBRATION BINS: "A bin under about thirty items is decoration: at three-quarters accuracy its
 * standard error is near nine points." That sentence is about bins, not about AUROC class sizes
 * (review M-6) — the thirty is borrowed from it as a round number, not applied because T6 names
 * AUROC. The reason thirty is also the right floor for an AUROC class is the Hanley-McNeil standard
 * error of AUROC, at AUROC 0.75 with equal classes:
 *
 *   | per class | SE    | 95% half-width |
 *   |-----------|-------|-----------------|
 *   | 10        | 0.112 | ±0.22           |
 *   | 30        | 0.063 | ±0.12           |
 *   | 50        | 0.049 | ±0.10           |
 *
 * At 10 per class the interval covers both the cosmetic (<0.70) and usable (>=0.80) bands entirely
 * — a grade would be noise with a label on it. Even at 30 per class the interval still spans both
 * bands, which is why this floor does not make B2's verdict automatic (changing that is T5/T7's
 * business, not this constant's) — it only makes the reported `ci95` narrow enough that a reader
 * (Quinn, looking at the report) can see which band the number leans toward. Named here so it is
 * one line to change, not a magic number buried in `aurocForWrong`. */
export const MIN_CLASS_N = 30;

/** Mann-Whitney U, rescaled to AUROC: P(score of a random positive > score of a random negative),
 * ties splitting 0.5 (Hanley & McNeil's rank-sum identity — this is the standard closed form, not
 * an approximation). `positive.length === scores.length` is the caller's contract; both arguments
 * come from this module's own call sites only. */
export function auc(scores: readonly number[], positive: readonly boolean[]): number {
  const n = scores.length;
  const nPos = positive.filter(Boolean).length;
  const nNeg = n - nPos;
  if (nPos === 0 || nNeg === 0) return NaN;

  const order = scores.map((_, i) => i).sort((a, b) => scores[a] - scores[b]);
  const ranks = new Array<number>(n);
  let i = 0;
  while (i < n) {
    let j = i;
    while (j + 1 < n && scores[order[j + 1]] === scores[order[i]]) j++;
    const averageRank = (i + j) / 2 + 1; // 1-based, tied observations share the mean rank
    for (let k = i; k <= j; k++) ranks[order[k]] = averageRank;
    i = j + 1;
  }

  let rankSumPositive = 0;
  for (let k = 0; k < n; k++) if (positive[k]) rankSumPositive += ranks[k];
  const u = rankSumPositive - (nPos * (nPos + 1)) / 2;
  return u / (nPos * nNeg);
}

function mulberry32(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function percentile(sortedAscending: readonly number[], p: number): number {
  const n = sortedAscending.length;
  if (n === 0) return NaN;
  if (n === 1) return sortedAscending[0];
  const idx = (n - 1) * p;
  const lo = Math.floor(idx), hi = Math.ceil(idx);
  if (lo === hi) return sortedAscending[lo];
  const frac = idx - lo;
  return sortedAscending[lo] * (1 - frac) + sortedAscending[hi] * frac;
}

export interface BootstrapCiOptions {
  /** Deterministic — the T6 dispatch requires reproducible tests, not a fresh answer every run. */
  seed?: number;
  samples?: number;
  /** 0.05 for a 95% interval, matching the note's own "bootstrap or DeLong" framing. Not
   * cost-derived, so a default is fine (see this module's header on which thresholds may not
   * default). */
  alpha?: number;
}

export const DEFAULT_BOOTSTRAP_SEED = 20260922; // today, stream J's start date — arbitrary but fixed
export const DEFAULT_BOOTSTRAP_SAMPLES = 1000;

export interface BootstrapCiResult {
  ci: [number, number] | null;
  /** How many of `samples` resamples produced a defined AUROC (both classes present). Less than
   * `samples` only on a very small or very imbalanced group; reported so a caller can see the CI
   * rests on fewer draws than it asked for, rather than silently trusting it. */
  usableSamples: number;
}

/** Percentile bootstrap CI for `auc(scores, positive)`. Resamples `scores`/`positive` together
 * (paired, i.e. by row) with replacement, `options.samples` times, and takes the
 * `[alpha/2, 1 - alpha/2]` percentiles of the resulting AUROC distribution. A resample that lands
 * all-one-class is skipped (its AUROC is undefined), not counted as zero. */
export function bootstrapAucCi(
  scores: readonly number[],
  positive: readonly boolean[],
  options: BootstrapCiOptions = {},
): BootstrapCiResult {
  const { seed = DEFAULT_BOOTSTRAP_SEED, samples = DEFAULT_BOOTSTRAP_SAMPLES, alpha = 0.05 } = options;
  const n = scores.length;
  if (n === 0) return { ci: null, usableSamples: 0 };

  const rng = mulberry32(seed);
  const draws: number[] = [];
  for (let s = 0; s < samples; s++) {
    const sampleScores = new Array<number>(n);
    const samplePositive = new Array<boolean>(n);
    for (let k = 0; k < n; k++) {
      const idx = Math.floor(rng() * n);
      sampleScores[k] = scores[idx];
      samplePositive[k] = positive[idx];
    }
    const a = auc(sampleScores, samplePositive);
    if (!Number.isNaN(a)) draws.push(a);
  }
  if (draws.length === 0) return { ci: null, usableSamples: 0 };
  draws.sort((x, y) => x - y);
  return {
    ci: [percentile(draws, alpha / 2), percentile(draws, 1 - alpha / 2)],
    usableSamples: draws.length,
  };
}

export type AurocVerdict = "usable" | "marginal" | "cosmetic" | "undefined";

export interface AurocResult {
  n: number;
  positives: number; // count of `wrong === true`
  negatives: number;
  value: number | null;
  ci95: [number, number] | null;
  bootstrapSamples: number;
  verdict: AurocVerdict;
  /** True only when `value` is defined and clearly below chance (`< 1 - AUROC_USABLE_MIN`,
   * i.e. < 0.20) — a coarse "this looks inverted, not just noisy" flag. See this section's header. */
  possiblyInverted: boolean;
}

export function aurocForWrong(
  rows: readonly CalibrationRow[],
  ciOptions: BootstrapCiOptions = {},
): AurocResult {
  const n = rows.length;
  const positives = rows.filter((r) => r.wrong).length;
  const negatives = n - positives;

  if (positives < MIN_CLASS_N || negatives < MIN_CLASS_N) {
    return {
      n,
      positives,
      negatives,
      value: null,
      ci95: null,
      bootstrapSamples: 0,
      verdict: "undefined",
      possiblyInverted: false,
    };
  }

  const scores = rows.map((r) => -r.confidence);
  const labels = rows.map((r) => r.wrong);
  const value = auc(scores, labels);
  const { ci, usableSamples } = bootstrapAucCi(scores, labels, ciOptions);

  const verdict: AurocVerdict = value >= AUROC_USABLE_MIN
    ? "usable"
    : value < AUROC_COSMETIC_MAX
    ? "cosmetic"
    : "marginal";

  return {
    n,
    positives,
    negatives,
    value,
    ci95: ci,
    bootstrapSamples: usableSamples,
    verdict,
    possiblyInverted: value < 1 - AUROC_USABLE_MIN,
  };
}

// -------------------------------------------------------------------------------------------
// Step 3: calibration. Kernel-smoothed below a few hundred labels (removes the bin-count
// decision, per the background note §5/§6); the binned view is reported ALONGSIDE it, not instead
// of it, because per-bin counts are what let a reader see a bin under ~30 is decoration.
// -------------------------------------------------------------------------------------------

export const MIN_BIN_N = 30;

/** Silverman's rule of thumb, the standard default bandwidth for a Gaussian kernel regression —
 * `1.06 * sd * n^(-1/5)`. Clamped to a floor of 0.02 so a group whose confidences are (near-)
 * constant (`sd` near 0, e.g. the tier-2-only case this query already excludes, or a degenerate
 * synthetic fixture) does not produce a bandwidth of 0 and a division by zero below. */
export function silvermanBandwidth(values: readonly number[]): number {
  const n = values.length;
  if (n < 2) return 0.1;
  const mean = values.reduce((a, b) => a + b, 0) / n;
  const variance = values.reduce((a, b) => a + (b - mean) ** 2, 0) / (n - 1);
  const sd = Math.sqrt(variance);
  return Math.max(0.02, 1.06 * sd * Math.pow(n, -1 / 5));
}

export interface KernelCalibrationPoint {
  confidence: number;
  /** Leave-one-out kernel-weighted accuracy at this point's confidence — "leave-one-out" so a
   * point is never used to smooth its own estimate, which would otherwise bias the error toward
   * zero exactly where the bandwidth is small. */
  estimatedAccuracy: number;
  /** Total kernel weight this point received from every OTHER row — near zero only when a
   * confidence value is far from the rest of the group's mass and the bandwidth is narrow; such a
   * point's `estimatedAccuracy` should be read with that in mind. */
  weight: number;
}

export interface KernelCalibrationResult {
  method: "kernel-smoothed";
  bandwidth: number;
  /** Mean absolute distance between each row's stated confidence and its leave-one-out estimated
   * accuracy — a smooth analogue of expected calibration error, chosen (per the background note
   * §5/§6a and Błasiok & Nakkiran's smooth-ECE) specifically to remove the bin-count decision that
   * dominates small-sample calibration error estimates. This is a simplified Nadaraya-Watson
   * version of that idea, not a re-implementation of the paper's exact estimator — good enough to
   * flag "does this need T7", not to publish a paper on our own numbers. */
  calibrationError: number;
  points: KernelCalibrationPoint[];
}

function gaussianKernel(u: number): number {
  return Math.exp(-0.5 * u * u);
}

export function kernelSmoothedCalibration(
  rows: readonly CalibrationRow[],
  bandwidth?: number,
): KernelCalibrationResult {
  const confidences = rows.map((r) => r.confidence);
  const correct = rows.map((r) => !r.wrong);
  const h = bandwidth ?? silvermanBandwidth(confidences);
  const n = rows.length;

  const points: KernelCalibrationPoint[] = new Array(n);
  let errorSum = 0;
  for (let i = 0; i < n; i++) {
    let weightSum = 0, correctSum = 0;
    for (let j = 0; j < n; j++) {
      if (j === i) continue; // leave-one-out
      const w = gaussianKernel((confidences[i] - confidences[j]) / h);
      weightSum += w;
      if (correct[j]) correctSum += w;
    }
    const estimatedAccuracy = weightSum > 0 ? correctSum / weightSum : correct[i] ? 1 : 0;
    points[i] = { confidence: confidences[i], estimatedAccuracy, weight: weightSum };
    errorSum += Math.abs(confidences[i] - estimatedAccuracy);
  }

  return { method: "kernel-smoothed", bandwidth: h, calibrationError: n > 0 ? errorSum / n : NaN, points };
}

export interface CalibrationBin {
  n: number;
  loConfidence: number;
  hiConfidence: number;
  meanConfidence: number;
  accuracy: number;
  /** A bin under `MIN_BIN_N` (~30) items — "at three-quarters accuracy its standard error is near
   * nine points" (T6 brief). Reported per bin so a binned view is never read past its own noise. */
  decoration: boolean;
}

/** `n / 40`, clamped to `[1, 10]` — the brief's own table gives no bins at 50 labels, ~3-5 at 200
 * and 10 at 1000; this is a smooth stand-in for those three named points, not a quote from either
 * source, and any caller of `equalMassBins` may pass an explicit `numBins` instead. */
export function suggestedBinCount(n: number): number {
  return Math.max(1, Math.min(10, Math.floor(n / 40)));
}

/** Equal-MASS bins (by count, not by confidence width) — "less biased than equal-width" per the
 * background note §5, citing Roelofs et al. Rows are sorted by confidence ascending and split into
 * `numBins` groups as equal in size as `n` allows (any remainder goes one-per-bin to the first
 * groups, so no bin is more than one row larger than another). */
export function equalMassBins(rows: readonly CalibrationRow[], numBins: number): CalibrationBin[] {
  const n = rows.length;
  if (n === 0 || numBins <= 0) return [];
  const sorted = [...rows].sort((a, b) => a.confidence - b.confidence);
  const base = Math.floor(n / numBins);
  const remainder = n % numBins;

  const bins: CalibrationBin[] = [];
  let start = 0;
  for (let b = 0; b < numBins && start < n; b++) {
    const size = base + (b < remainder ? 1 : 0);
    if (size === 0) continue;
    const slice = sorted.slice(start, start + size);
    const confidences = slice.map((r) => r.confidence);
    const correctCount = slice.filter((r) => !r.wrong).length;
    bins.push({
      n: slice.length,
      loConfidence: Math.min(...confidences),
      hiConfidence: Math.max(...confidences),
      meanConfidence: confidences.reduce((a, c) => a + c, 0) / slice.length,
      accuracy: correctCount / slice.length,
      decoration: slice.length < MIN_BIN_N,
    });
    start += size;
  }
  return bins;
}

/** "Roughly calibrated" is a judgment call the brief does not pin a number to; 0.05 (five points of
 * mean absolute error) is this harness's own default — NOT cost-derived (Elkan's `t*` below is the
 * thing that must never default; this is an ordinary statistical threshold, and defaulting it is
 * fine) — and a caller may override it via `buildGroupReport`'s options. */
export const DEFAULT_CALIBRATION_ERROR_THRESHOLD = 0.05;

// -------------------------------------------------------------------------------------------
// The cost-optimal threshold (Elkan 2001), background note §4: `t* = C_FP / (C_FP + C_FN)`.
// Not part of `buildCalibrationReport` and not called from anywhere in this module — the T6
// dispatch is explicit that a cost-derived threshold "must read its costs from a parameter and
// ship with no default value", so this function requires both costs and is exported for a caller
// (a human, or a later task) to invoke once real costs exist. `score.ts`'s `COST` map is read by
// nothing here, imported by nothing here, and stays exactly as `score.ts` owns it.
// -------------------------------------------------------------------------------------------

/**
 * Elkan's cost-optimal threshold for a binary decision with a zero-cost diagonal: act on the
 * positive prediction when `p >= t*`. Both costs are REQUIRED — there is no default cost ratio,
 * because a wrong default here would be silently wrong in the specific way T0's cost matrices are
 * "proposed, not ruled": whoever calls this decides the ratio, this function only does the
 * arithmetic. Throws on a non-positive cost, since `t*` is undefined (division by zero, or a
 * threshold outside `[0, 1]`) otherwise.
 */
export function costOptimalThreshold(costFalsePositive: number, costFalseNegative: number): number {
  if (!(costFalsePositive > 0) || !(costFalseNegative > 0)) {
    throw new Error("costOptimalThreshold: both costs must be positive numbers");
  }
  return costFalsePositive / (costFalsePositive + costFalseNegative);
}

// -------------------------------------------------------------------------------------------
// The report: one group (kind x prompt_version x prompt_hash), the brief's cascade, B2's outcome.
// -------------------------------------------------------------------------------------------

export type B2Outcome =
  | "does_not_rank"
  | "ranks_but_miscalibrated"
  | "ranks_and_calibrated"
  | "insufficient_data";

export interface CalibrationGroupReport {
  kind: Kind;
  prompt_version: string | null;
  prompt_hash: string | null;
  n: number;
  verdictCounts: Record<string, number>;
  stoppedAt: 1 | 2 | 3;
  discrimination: {
    distinctConfidenceValues: number;
    decoration: boolean;
  };
  auroc: AurocResult | null;
  calibration: {
    kernel: KernelCalibrationResult;
    bins: CalibrationBin[];
    threshold: number;
    roughlyCalibrated: boolean;
  } | null;
  b2Outcome: B2Outcome;
  /** A one-line pointer to what happens next, in the brief's own words where it gives them. Never
   * names a fitted-scorer/Dirichlet/isotonic step as anything but "T7, not built here" — the
   * dispatch's own boundary. */
  nextStep: string;
}

export interface BuildReportOptions {
  ciOptions?: BootstrapCiOptions;
  calibrationErrorThreshold?: number;
  numBins?: number;
  bandwidth?: number;
}

function countVerdicts(rows: readonly CalibrationRow[]): Record<string, number> {
  const counts: Record<string, number> = {};
  for (const r of rows) {
    const key = r.verdict ?? "(none)";
    counts[key] = (counts[key] ?? 0) + 1;
  }
  return counts;
}

/** One group's full report, following the brief's stop-early cascade exactly. */
export function buildGroupReport(
  rows: readonly CalibrationRow[],
  options: BuildReportOptions = {},
): CalibrationGroupReport {
  if (rows.length === 0) throw new Error("buildGroupReport: rows must be non-empty");
  const { kind, prompt_version, prompt_hash } = rows[0];

  const distinctCount = distinctConfidenceValues(rows);
  const decoration = isDecorationByDistinctValues(distinctCount);

  const base = {
    kind,
    prompt_version,
    prompt_hash,
    n: rows.length,
    verdictCounts: countVerdicts(rows),
    discrimination: { distinctConfidenceValues: distinctCount, decoration },
  };

  if (decoration) {
    return {
      ...base,
      stoppedAt: 1,
      auroc: null,
      calibration: null,
      b2Outcome: "does_not_rank",
      nextStep: "The field does not rank (decoration at step 1: <=3 distinct confidence values). " +
        "Replace it with the rules tier and a fixed policy; T5 and T7 both vanish.",
    };
  }

  const aurocResult = aurocForWrong(rows, options.ciOptions);

  if (aurocResult.verdict === "undefined") {
    return {
      ...base,
      stoppedAt: 2,
      auroc: aurocResult,
      calibration: null,
      b2Outcome: "insufficient_data",
      nextStep: `Too few examples of one class to estimate AUROC safely ` +
        `(positives=${aurocResult.positives}, negatives=${aurocResult.negatives}, floor=${MIN_CLASS_N} each). ` +
        "Wait for more corrections before grading this group at all.",
    };
  }

  if (aurocResult.verdict !== "usable") {
    const why = aurocResult.verdict === "cosmetic"
      ? `cosmetic (AUROC ${aurocResult.value!.toFixed(3)} < ${AUROC_COSMETIC_MAX})`
      : `in the undefined middle band (AUROC ${
        aurocResult.value!.toFixed(3)
      }, between ${AUROC_COSMETIC_MAX} and ${AUROC_USABLE_MIN}) ` +
        `— below the ${AUROC_USABLE_MIN} bar this harness requires before fitting a calibration curve`;
    return {
      ...base,
      stoppedAt: 2,
      auroc: aurocResult,
      calibration: null,
      b2Outcome: "does_not_rank",
      nextStep: `The field does not rank: AUROC is ${why}. ` +
        "Replace it with the rules tier and a fixed policy; T5 and T7 both vanish.",
    };
  }

  const numBins = options.numBins ?? suggestedBinCount(rows.length);
  const kernel = kernelSmoothedCalibration(rows, options.bandwidth);
  const bins = equalMassBins(rows, numBins);
  const threshold = options.calibrationErrorThreshold ?? DEFAULT_CALIBRATION_ERROR_THRESHOLD;
  const roughlyCalibrated = kernel.calibrationError <= threshold;

  return {
    ...base,
    stoppedAt: 3,
    auroc: aurocResult,
    calibration: { kernel, bins, threshold, roughlyCalibrated },
    b2Outcome: roughlyCalibrated ? "ranks_and_calibrated" : "ranks_but_miscalibrated",
    nextStep: roughlyCalibrated
      ? "It ranks and is roughly calibrated. The worry dissolves; go straight to T5."
      : "It ranks but is miscalibrated. Fit a map on our own logs (T7, not built here): " +
        "Dirichlet or vector scaling for this three/five-way verdict as the default, per-class " +
        "Platt if only one number per class is available; not isotonic below about a thousand " +
        "labels, though fit both and compare held out once there are enough.",
  };
}

/** Every group in `rows`, in a stable order (grouping order of first appearance). Convenience over
 * `groupRows` + a `buildGroupReport` per bucket — the CLI's whole job. */
export function buildCalibrationReport(
  rows: readonly CalibrationRow[],
  options: BuildReportOptions = {},
): CalibrationGroupReport[] {
  const groups = groupRows(rows);
  return [...groups.values()].map((groupRows_) => buildGroupReport(groupRows_, options));
}
