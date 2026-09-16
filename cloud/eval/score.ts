// The scorer. Pure, so the harness is tested with no key and no spend.
import type { Kind } from "../supabase/functions/_shared/judge_validate.ts";

export interface Case {
  kind: Kind;
  /** What the human set — the label. A real correction may label only a SUBSET of a kind's
   * scorable fields (Task 13's schema accepts any non-empty subset), so every metric below counts
   * only the cases that actually label its own field. */
  theirs: Record<string, unknown>;
}

export interface Scored {
  metric: string;
  /** `null` when no case in the batch labelled this metric's field — the metric was never
   * credited and never penalised, because there was nothing to compare. */
  value: number | null;
  /** `true` when a HIGHER value is better; false for an error metric. */
  higherIsBetter: boolean;
  /** How many cases contributed to `value`. `0` iff `value` is `null`. */
  n: number;
}

/**
 * The asymmetry is the whole point. A missed obligation is the failure this product exists to
 * prevent, and a task that reads as a newsletter is the second: both cost three times what the
 * harmless direction costs, so a model that gets "safe" by dropping everything scores worse.
 */
export const COST: Record<string, number> = {
  "obligation->drop": 3,
  "obligation->opportunity": 1,
  "opportunity->drop": 1,
  "drop->opportunity": 1,
  "task->information": 3,
  "task->borderline": 1,
  "information->borderline": 1,
  "information->task": 2,
};

function cost(theirs: string, ours: string): number {
  if (theirs === ours) return 0;
  // Task 14 fix (found running the brief's own test): a MISSING answer (the model failed, or a
  // dry run's null verdict) is never assumed cheap. Falling through to the `?? 1` default below
  // would score "no answer at all" as a mild, generic mismatch — cheaper than "obligation called
  // opportunity" — which contradicts this file's own asymmetry ("a model that gets 'safe' by
  // dropping everything scores worse"): an unusable answer is at least as bad as the worst named
  // mismatch for whatever `theirs` said, so it costs the maximum, 3, same as `weighted_exact`'s
  // own worst case.
  if (ours === "") return 3;
  return COST[`${theirs}->${ours}`] ?? 1;
}

/** Whether `theirs` labels `field` at all — a key that is ABSENT, not merely falsy or null, is
 * what "not labelled" means (a task's `course` may legitimately be labelled `null`). */
function labelled(theirs: Record<string, unknown>, field: string): boolean {
  return field in theirs;
}

export function score(kind: Kind, cases: Case[], answers: Array<Record<string, unknown> | null>): Scored[] {
  const n = cases.length;
  if (n === 0) return [];
  if (kind === "task") {
    let absError = 0, effortN = 0;
    let importanceHits = 0, importanceN = 0;
    let courseHits = 0, courseN = 0;
    for (let i = 0; i < n; i++) {
      const theirs = cases[i].theirs, ours = answers[i] ?? {};
      if (labelled(theirs, "effort_hours")) {
        absError += Math.abs(Number(theirs.effort_hours) - Number(ours.effort_hours ?? 0));
        effortN++;
      }
      if (labelled(theirs, "importance")) {
        if (Number(theirs.importance) === Number(ours.importance)) importanceHits++;
        importanceN++;
      }
      if (labelled(theirs, "course")) {
        if ((theirs.course ?? null) === (ours.course ?? null)) courseHits++;
        courseN++;
      }
    }
    return [
      { metric: "effort_mae", value: effortN === 0 ? null : absError / effortN, higherIsBetter: false, n: effortN },
      {
        metric: "importance_exact",
        value: importanceN === 0 ? null : importanceHits / importanceN,
        higherIsBetter: true,
        n: importanceN,
      },
      {
        metric: "course_exact",
        value: courseN === 0 ? null : courseHits / courseN,
        higherIsBetter: true,
        n: courseN,
      },
    ];
  }
  const field = kind === "event" ? "verdict" : "tier";
  let earned = 0, contributing = 0;
  for (let i = 0; i < n; i++) {
    if (!labelled(cases[i].theirs, field)) continue;
    const theirs = String(cases[i].theirs[field] ?? "");
    const ours = String((answers[i] ?? {})[field] ?? "");
    earned += 3 - Math.min(3, cost(theirs, ours));
    contributing++;
  }
  return [{
    metric: "weighted_exact",
    value: contributing === 0 ? null : earned / (contributing * 3),
    higherIsBetter: true,
    n: contributing,
  }];
}

/**
 * A metric fails when it is WORSE than its threshold. Never equal — a gate at the observed value
 * fires on noise. A metric with nothing to compare (`value: null`) never fails: there is nothing
 * to be wrong about. A metric that computed NaN (malformed input data, not "no data") always
 * fails: NaN is not "no worse than the threshold", it is a metric that broke.
 */
export function failed(s: Scored, threshold: number): boolean {
  if (s.value === null) return false;
  if (Number.isNaN(s.value)) return true;
  return s.higherIsBetter ? s.value < threshold : s.value > threshold;
}
