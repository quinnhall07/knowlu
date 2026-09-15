// The scorer. Pure, so the harness is tested with no key and no spend.
import type { Kind } from "../supabase/functions/_shared/judge_validate.ts";

export interface Case {
  kind: Kind;
  /** What the human set — the label. */
  theirs: Record<string, unknown>;
}

export interface Scored {
  metric: string;
  value: number;
  /** `true` when a HIGHER value is better; false for an error metric. */
  higherIsBetter: boolean;
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

export function score(kind: Kind, cases: Case[], answers: Array<Record<string, unknown> | null>): Scored[] {
  const n = cases.length;
  if (n === 0) return [];
  if (kind === "task") {
    let absError = 0, importanceHits = 0, courseHits = 0;
    for (let i = 0; i < n; i++) {
      const theirs = cases[i].theirs, ours = answers[i] ?? {};
      absError += Math.abs(Number(theirs.effort_hours ?? 0) - Number(ours.effort_hours ?? 0));
      if (Number(theirs.importance) === Number(ours.importance)) importanceHits++;
      if ((theirs.course ?? null) === (ours.course ?? null)) courseHits++;
    }
    return [
      { metric: "effort_mae", value: absError / n, higherIsBetter: false },
      { metric: "importance_exact", value: importanceHits / n, higherIsBetter: true },
      { metric: "course_exact", value: courseHits / n, higherIsBetter: true },
    ];
  }
  const field = kind === "event" ? "verdict" : "tier";
  let earned = 0, possible = 0;
  for (let i = 0; i < n; i++) {
    const theirs = String(cases[i].theirs[field] ?? "");
    const ours = String((answers[i] ?? {})[field] ?? "");
    possible += 3;
    earned += 3 - Math.min(3, cost(theirs, ours));
  }
  return [{ metric: "weighted_exact", value: possible === 0 ? 0 : earned / possible, higherIsBetter: true }];
}

/** A metric fails when it is WORSE than its threshold. Never equal — a gate at the observed value fires on noise. */
export function failed(s: Scored, threshold: number): boolean {
  return s.higherIsBetter ? s.value < threshold : s.value > threshold;
}
