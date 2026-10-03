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
 *
 * Every off-diagonal cell of the event 4x4 (obligation, opportunity, drop, unsure) and the email
 * 6x6 (task, borderline, event, opportunity, information, completion) is named here — 12 + 30 = 42
 * cells — so none fall through `cost()`'s `?? 1` default any more. Source: T0's ratification,
 * `docs/notes/2026-09-22-cost-matrices.md` (RULED by Quinn 2026-09-22) for obligation/opportunity/
 * drop (event §3) and task/borderline/event/opportunity/information (email §4); the `unsure` and
 * `completion` cells the note left open were ruled by the stream controller the same day (see the
 * note's "Rulings after T0" section for the one-line reason each cell carries).
 */
export const COST: Record<string, number> = {
  // --- event, 4x4 (obligation, opportunity, drop, unsure) ---
  "obligation->drop": 3,
  "obligation->opportunity": 1,
  "opportunity->drop": 1,
  "drop->opportunity": 1,
  "opportunity->obligation": 2,
  "drop->obligation": 2,
  // `unsure` never surfaces to the student today, so calling an obligation `unsure` hides it
  // exactly the way `drop` does: 3, not the 2 a merely-invented obligation would cost. Revisit to
  // 2 once `unsure` surfaces somewhere the student can see and correct it.
  "obligation->unsure": 3,
  "opportunity->unsure": 1,
  "drop->unsure": 1,
  "unsure->obligation": 2,
  "unsure->opportunity": 1,
  "unsure->drop": 2,

  // --- email, 6x6 (task, borderline, event, opportunity, information, completion) ---
  "task->information": 3,
  "task->borderline": 1,
  "information->borderline": 1,
  "information->task": 2,
  "task->event": 2,
  "task->opportunity": 2,
  "borderline->task": 1,
  "borderline->event": 1,
  "borderline->opportunity": 1,
  "borderline->information": 2,
  "event->task": 2,
  "event->borderline": 1,
  "event->opportunity": 1,
  "event->information": 2,
  "opportunity->task": 2,
  "opportunity->borderline": 1,
  "opportunity->event": 1,
  "opportunity->information": 3,
  "information->event": 1,
  "information->opportunity": 1,
  // `completion` (T9): a completion email that matches no active task writes nothing (T9's
  // `propose_done` needs a title match), the same outcome as filing it as `information` — so
  // `completion->information` costs what `X->information` already costs for a real item, 1, not
  // more. `task->completion` costs the same as `task->information`, 3: a real task's completion
  // signal is itself a needle that only matters if the task was tracked, but calling a live task
  // "already done" hides it exactly as hard as losing it to `information` does.
  "task->completion": 3,
  "completion->task": 2,
  "completion->information": 1,
  "information->completion": 2,
  "completion->borderline": 1,
  "completion->event": 1,
  "completion->opportunity": 1,
  "borderline->completion": 2,
  "event->completion": 2,
  "opportunity->completion": 3,
};

function cost(theirs: string, ours: string): number {
  if (theirs === ours) return 0;
  // Task 14 fix (found running the brief's own test): a MISSING answer (a refusal, a truncation,
  // an invalid or below-floor reply, or a dry run's null verdict) is never assumed cheap. A call
  // that never answered at all (`model failed`) does not reach here: `run_eval.ts` leaves it
  // unscored (see `providerError` below). Falling through to the `?? 1` default below
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
 * A case whose model call never answered — cause `model failed` (transport, auth, provider,
 * timeout, an error envelope, an unparseable reply), still failing after `run_eval.ts`'s retries —
 * is not scored at all: it says nothing about the model's judgment. PR #33's eval-gate
 * (2026-10-02) scored such cases as the worst miss and read 0.564 where replays of the same seed
 * read 0.897-0.936. A refusal or a truncation is the model answering and stays scored as before.
 *
 * A few unscored cases are noise and the kind is scored on the rest. More than
 * `UNSCORED_ALLOWED_MIN` cases or `UNSCORED_ALLOWED_FRACTION` of them, whichever is larger, is an
 * outage: the gate fails as a provider error, never quietly passes on what is left.
 */
export const UNSCORED_ALLOWED_MIN = 2;
export const UNSCORED_ALLOWED_FRACTION = 0.1;

/** How many of `total` cases may go unscored before the kind is a provider error. */
export function unscoredAllowed(total: number): number {
  return Math.max(UNSCORED_ALLOWED_MIN, total * UNSCORED_ALLOWED_FRACTION);
}

/** Whether `unscored` of `total` cases is an outage rather than a quality result: past the bound,
 * or nothing left to score at all (a one-case kind whose case failed must not pass vacuously). */
export function providerError(unscored: number, total: number): boolean {
  if (unscored <= 0) return false;
  return unscored > unscoredAllowed(total) || unscored >= total;
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
