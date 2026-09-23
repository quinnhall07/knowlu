// Reads one decisions-endpoint reply (jev_request.ts's header note has the verified shape) and scores
// a whole pass against the corpus labels — procedure §Step 5: agreement with a Wilson 95% CI (the
// same `wilsonInterval` the baselines use, so the numbers compare like for like), the full confusion
// matrix, the demotion cell (a Critical graded Minor — stop rule 2), the `blocks` probability against
// "is this Critical or Important" (AUROC and Brier), and whether each choice's `confidence` separates
// right answers from wrong ones (stop rule 4).
//
// Failed items (HTTP error, unparseable reply) are listed apart and left out of every accuracy's n;
// the report says how many there were, so a pass with failures cannot pass for a clean one.

import { type BaselineResult, wilsonInterval } from "./baselines.ts";
import { DISPOSITION_CRITERIA, DISPOSITION_OPTION_TO_LABEL, SEVERITY_CRITERIA } from "./jev_request.ts";
import type { CorpusSet, Disposition, Severity } from "./types.ts";

export interface ChoiceAnswer {
  choice: string;
  probabilities: Record<string, number>;
  confidence: number;
}

export interface DecisionReply {
  model: string;
  id: string | null;
  provider: string | null;
  grade: ChoiceAnswer;
  disposition: ChoiceAnswer;
  /** The `blocks` noul: the probability that "this finding must be fixed before the branch merges". */
  blocks: number;
  usage: { inputTokens: number; outputTokens: number; costUsd: number };
}

export type ParseResult = { ok: true; reply: DecisionReply } | { ok: false; error: string };

type Json = Record<string, unknown>;
const isObj = (v: unknown): v is Json => typeof v === "object" && v !== null && !Array.isArray(v);
const isNum = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v);

function readChoice(answers: Json, key: string, options: string[]): ChoiceAnswer | string {
  const a = answers[key];
  if (!isObj(a)) return `answer ${key} missing`;
  if (typeof a.choice !== "string" || !options.includes(a.choice)) {
    return `answer ${key}: choice ${JSON.stringify(a.choice)} is not one of ${JSON.stringify(options)}`;
  }
  if (!isNum(a.confidence)) return `answer ${key}: confidence missing`;
  const probabilities: Record<string, number> = {};
  if (isObj(a.probabilities)) {
    for (const [k, v] of Object.entries(a.probabilities)) if (isNum(v)) probabilities[k] = v;
  }
  return { choice: a.choice, probabilities, confidence: a.confidence };
}

function snippet(text: string): string {
  return text.length > 200 ? `${text.slice(0, 200)}…` : text;
}

/** Parses one HTTP reply from the decisions endpoint. Never throws. */
export function parseDecisionReply(status: number, text: string): ParseResult {
  let body: unknown;
  try {
    body = JSON.parse(text);
  } catch {
    return { ok: false, error: `HTTP ${status}: ${snippet(text)}` };
  }
  if (isObj(body) && isObj(body.error)) {
    return { ok: false, error: `HTTP ${status}: ${body.error.code} ${body.error.message}` };
  }
  if (status < 200 || status >= 300) return { ok: false, error: `HTTP ${status}: ${snippet(text)}` };
  if (!isObj(body) || !isObj(body.answers)) return { ok: false, error: `HTTP ${status}: no answers` };

  const grade = readChoice(body.answers, "grade", Object.keys(SEVERITY_CRITERIA));
  if (typeof grade === "string") return { ok: false, error: grade };
  const disposition = readChoice(body.answers, "disposition", Object.keys(DISPOSITION_CRITERIA));
  if (typeof disposition === "string") return { ok: false, error: disposition };
  const blocks = body.answers.blocks;
  if (!isObj(blocks) || !isNum(blocks.noul) || blocks.noul < 0 || blocks.noul > 1) {
    return { ok: false, error: "answer blocks: noul missing or outside [0, 1]" };
  }
  const usage = isObj(body.usage) ? body.usage : {};
  return {
    ok: true,
    reply: {
      model: typeof body.model === "string" ? body.model : "",
      id: typeof body.id === "string" ? body.id : null,
      provider: typeof body.provider === "string" ? body.provider : null,
      grade,
      disposition,
      blocks: blocks.noul,
      usage: {
        inputTokens: isNum(usage.input_tokens) ? usage.input_tokens : 0,
        outputTokens: isNum(usage.output_tokens) ? usage.output_tokens : 0,
        costUsd: isNum(usage.cost) ? usage.cost : 0,
      },
    },
  };
}

/** One item of a paid pass, as `replay.ts` records it and `paid-run-results.jsonl` stores it. */
export interface ItemResult {
  id: string;
  set: CorpusSet;
  severity: Severity;
  disposition: Disposition;
  /** HTTP status, or null when the request never got one (a network error). */
  status: number | null;
  elapsedMs: number;
  request?: unknown;
  reply?: DecisionReply;
  error?: string;
}

/**
 * Area under the ROC curve by the Mann-Whitney count (ties score a half): the chance a random
 * positive scores above a random negative. Null when either class is empty.
 */
export function auroc(scores: number[], positive: boolean[]): number | null {
  const pos = scores.filter((_, i) => positive[i]);
  const neg = scores.filter((_, i) => !positive[i]);
  if (pos.length === 0 || neg.length === 0) return null;
  let wins = 0;
  for (const p of pos) for (const n of neg) wins += p > n ? 1 : p === n ? 0.5 : 0;
  return wins / (pos.length * neg.length);
}

/** Mean squared distance between a probability and the 0/1 outcome. */
export function brier(probabilities: number[], outcome: boolean[]): number {
  let sum = 0;
  for (let i = 0; i < probabilities.length; i++) sum += (probabilities[i] - (outcome[i] ? 1 : 0)) ** 2;
  return sum / probabilities.length;
}

/** Nearest-rank percentile; null for an empty list. */
export function nearestRankPercentile(values: number[], p: number): number | null {
  if (values.length === 0) return null;
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.max(0, Math.ceil((p / 100) * sorted.length) - 1)];
}

export interface ConfidenceSeparation {
  meanRight: number | null;
  meanWrong: number | null;
  /** AUROC of `confidence` predicting "this answer is right" — 0.5 means it separates nothing. */
  auroc: number | null;
}

export interface LabelScore {
  accuracy: BaselineResult;
  /** truth → predicted → count. */
  confusion: Record<string, Record<string, number>>;
  confidence: ConfidenceSeparation;
}

export interface ScoredSummary {
  items: number;
  answered: number;
  failed: string[];
  severity: LabelScore & { demotions: string[] };
  disposition: LabelScore;
  blocks: { n: number; positives: number; auroc: number | null; brier: number | null };
  costUsd: number;
  inputTokens: number;
  outputTokens: number;
  latencyMs: { p50: number | null; p99: number | null };
  models: string[];
}

const mean = (xs: number[]) => (xs.length === 0 ? null : xs.reduce((a, b) => a + b, 0) / xs.length);

function scoreLabel(pairs: { truth: string; predicted: string; confidence: number }[]): LabelScore {
  const confusion: Record<string, Record<string, number>> = {};
  let correct = 0;
  for (const p of pairs) {
    confusion[p.truth] ??= {};
    confusion[p.truth][p.predicted] = (confusion[p.truth][p.predicted] ?? 0) + 1;
    if (p.truth === p.predicted) correct++;
  }
  const n = pairs.length;
  const right = pairs.map((p) => p.truth === p.predicted);
  const conf = pairs.map((p) => p.confidence);
  return {
    accuracy: { n, correct, accuracy: n === 0 ? 0 : correct / n, ci95: wilsonInterval(correct, n) },
    confusion,
    confidence: {
      meanRight: mean(conf.filter((_, i) => right[i])),
      meanWrong: mean(conf.filter((_, i) => !right[i])),
      auroc: auroc(conf, right),
    },
  };
}

/** Scores a whole pass. */
export function scoreResults(items: ItemResult[]): ScoredSummary {
  const answered = items.filter((it) => it.reply !== undefined) as (ItemResult & { reply: DecisionReply })[];
  const severity = scoreLabel(answered.map((it) => ({
    truth: it.severity,
    predicted: it.reply.grade.choice,
    confidence: it.reply.grade.confidence,
  })));
  const disposition = scoreLabel(answered.map((it) => ({
    truth: it.disposition,
    predicted: DISPOSITION_OPTION_TO_LABEL[it.reply.disposition.choice as keyof typeof DISPOSITION_CRITERIA],
    confidence: it.reply.disposition.confidence,
  })));
  const serious = answered.map((it) => it.severity === "critical" || it.severity === "important");
  const blocksP = answered.map((it) => it.reply.blocks);
  return {
    items: items.length,
    answered: answered.length,
    failed: items.filter((it) => it.reply === undefined).map((it) => it.id),
    severity: {
      ...severity,
      demotions: answered.filter((it) => it.severity === "critical" && it.reply.grade.choice === "minor").map(
        (it) => it.id,
      ),
    },
    disposition,
    blocks: {
      n: answered.length,
      positives: serious.filter(Boolean).length,
      auroc: auroc(blocksP, serious),
      brier: answered.length === 0 ? null : brier(blocksP, serious),
    },
    costUsd: answered.reduce((a, it) => a + it.reply.usage.costUsd, 0),
    inputTokens: answered.reduce((a, it) => a + it.reply.usage.inputTokens, 0),
    outputTokens: answered.reduce((a, it) => a + it.reply.usage.outputTokens, 0),
    latencyMs: {
      p50: nearestRankPercentile(items.map((it) => it.elapsedMs), 50),
      p99: nearestRankPercentile(items.map((it) => it.elapsedMs), 99),
    },
    models: [...new Set(answered.map((it) => it.reply.model))],
  };
}
