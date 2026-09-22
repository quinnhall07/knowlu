// The third label (procedure §1): "each finding has a disposition in four values: fixed in this
// round, ruled against, handed off, deferred... it is the one worth predicting". Unlike severity,
// disposition is not spelled out in the source documents in one consistent, parseable shape — it is
// the controller's ruling, recorded across each task's re-review file, the final review's own
// re-review, and (for a few items the procedure names directly) progress.md's rulings. A generic
// parser over that prose would be more fragile than the thing it is trying to protect against, for
// a 60-item corpus small enough to read by hand once.
//
// So this is a curated table, not a parser — read against every re-review document that exists
// (task-1-rereview.md, task-3-rereview.md, task-7-rereview.md, final-rereview.md, and the plan
// review's own "Re-review after fix round 1/2" sections), 2026-09-22. Each entry's citation is
// where that ruling was read. `dispositionFor` throws on an id this table does not cover, so a
// change to the extractor's id scheme is caught here rather than silently mislabelling a finding.

import type { Disposition } from "./types.ts";

export interface DispositionEntry {
  disposition: Disposition;
  citedFrom: string;
}

// --- Set A: the plan review. Its own "Re-review after fix round 1" section resolves every one of
// the 24 original findings (the six new findings R1-R6 that round surfaced are not part of this
// corpus — they were never in the original Critical/Important/Minor lists the procedure counts).
// Every original finding reads as resolved-or-superseded-and-applied there, which is "fixed":
// nothing in Set A was ruled against, handed off or deferred. That is a true, if unbalanced,
// picture of how thorough that review's own fix-and-recheck loop was, not an extraction artefact —
// see docs/reports/2026-09-22-e2-review-replay-prep.md for the class-balance consequence.
const PLAN_REVIEW_CITATION =
  "docs/reports/2026-09-17-c1b-sign-in-plan-review.md, 'Re-review after fix round 1/2'";

const SET_A: Record<string, DispositionEntry> = Object.fromEntries(
  [
    "C1",
    "C2",
    "C3",
    "C4",
    "C5",
    "C6",
    "I1",
    "I2",
    "I3",
    "I4",
    "I5",
    "I6",
    "I7",
    "I8",
    "M1",
    "M2",
    "M3",
    "M4",
    "M5",
    "M6",
    "M7",
    "M8",
    "M9",
    "M10",
  ].map((tag) => [`A-${tag}`, { disposition: "fixed" as Disposition, citedFrom: PLAN_REVIEW_CITATION }]),
);

// --- Set B: the seven task reviews and the final review.
const SET_B: Record<string, DispositionEntry> = {
  // Task 1 — task-1-rereview.md: items 1-9 resolved; item 10 was always "correctly handled by the
  // plan's own ordering", not a code change.
  "B-task1-1": { disposition: "fixed", citedFrom: "task-1-rereview.md item 1" },
  "B-task1-2": { disposition: "fixed", citedFrom: "task-1-rereview.md item 2" },
  "B-task1-3": { disposition: "fixed", citedFrom: "task-1-rereview.md item 3" },
  "B-task1-4": { disposition: "fixed", citedFrom: "task-1-rereview.md item 4" },
  "B-task1-5": { disposition: "fixed", citedFrom: "task-1-rereview.md item 5 ('resolved, as ruled')" },
  "B-task1-6": {
    disposition: "fixed",
    citedFrom: "task-1-rereview.md item 6 (already fixed on main, 8c11d3b)",
  },
  "B-task1-7": { disposition: "fixed", citedFrom: "task-1-rereview.md item 7" },
  "B-task1-8": { disposition: "fixed", citedFrom: "task-1-rereview.md item 8" },
  "B-task1-9": { disposition: "fixed", citedFrom: "task-1-rereview.md item 9" },
  "B-task1-10": {
    disposition: "ruled_against",
    citedFrom: "task-1-review.md item 10: 'already handled by the plan', Task 7's ordering",
  },

  // Task 2 — no rereview file exists (0 should-fix items); both nits explicitly asked for no fix.
  "B-task2-1": { disposition: "ruled_against", citedFrom: "task-2-review.md finding 1: 'No fix needed.'" },
  "B-task2-2": {
    disposition: "ruled_against",
    citedFrom: "task-2-review.md finding 2: a no-op observation, not a deviation",
  },

  // Task 3 — task-3-rereview.md: items 1,2,5 resolved; item 4 resolved (doc comment added, no
  // behavior change asked); item 3 is dropped by the secret filter regardless of its disposition;
  // item 6 explicitly handed to Task 5's review.
  "B-task3-1": { disposition: "fixed", citedFrom: "task-3-rereview.md item 1" },
  "B-task3-2": { disposition: "fixed", citedFrom: "task-3-rereview.md item 2 (resolved, with a caveat)" },
  "B-task3-3": { disposition: "fixed", citedFrom: "task-3-rereview.md item 3 (dropped by secret filter)" },
  "B-task3-4": { disposition: "fixed", citedFrom: "task-3-rereview.md item 4 (doc comment added)" },
  "B-task3-5": { disposition: "fixed", citedFrom: "task-3-rereview.md item 5" },
  "B-task3-6": {
    disposition: "handed_off",
    citedFrom: "task-3-rereview.md: 'correctly left for Task 5'",
  },

  // Task 4 — no rereview file (0 should-fix items); the one nit needed no code change.
  "B-task4-1": {
    disposition: "ruled_against",
    citedFrom: "task-4-review.md finding 1: 'No code or scope issue'",
  },

  // Task 5 — no rereview file; the one nit was explicitly deferred to a future review of Task 6.
  "B-task5-1": {
    disposition: "deferred",
    citedFrom: "task-5-review.md finding 1: 'not this task's brief to fix'",
  },

  // Task 6 — no rereview file; both nits needed no code change.
  "B-task6-1": {
    disposition: "ruled_against",
    citedFrom: "task-6-review.md finding 1: brief-template note, 'no code change needed here'",
  },
  "B-task6-2": {
    disposition: "ruled_against",
    citedFrom: "task-6-review.md finding 2: 'No functional issue — flagged only'",
  },

  // Task 7 — task-7-rereview.md: both the blocking finding and the nit resolved.
  "B-task7-1": { disposition: "fixed", citedFrom: "task-7-rereview.md: 'Both ... are resolved'" },
  "B-task7-2": { disposition: "fixed", citedFrom: "task-7-rereview.md: 'Strengthened pin'" },

  // Final review — final-rereview.md resolves F1,F3,F4,F6,F9,F10,F12 (F2,F9 folded together);
  // F2,F7,F8,F11 are dropped by the secret filter regardless of disposition; F5 is the one the
  // procedure names as handed off (H5), and final-rereview.md confirms it was untouched "as scoped".
  "B-final-F1": { disposition: "fixed", citedFrom: "final-rereview.md F1" },
  "B-final-F2": { disposition: "fixed", citedFrom: "final-rereview.md F2 (dropped by secret filter)" },
  "B-final-F3": { disposition: "fixed", citedFrom: "final-rereview.md F3" },
  "B-final-F4": { disposition: "fixed", citedFrom: "final-rereview.md F4" },
  "B-final-F5": {
    disposition: "handed_off",
    citedFrom:
      "e2 procedure §1 ('handed to the controller ... H5'); final-rereview.md: 'untouched, as scoped'",
  },
  "B-final-F6": { disposition: "fixed", citedFrom: "final-rereview.md F6" },
  "B-final-F7": {
    disposition: "deferred",
    citedFrom: "final-review.md 'Before merge' item 3 (staging proof pending); dropped by secret filter",
  },
  "B-final-F8": {
    disposition: "deferred",
    citedFrom: "e2 procedure §1 ('recorded as a follow-up'); dropped by secret filter",
  },
  "B-final-F9": { disposition: "fixed", citedFrom: "final-rereview.md F9 (folded into F2)" },
  "B-final-F10": { disposition: "fixed", citedFrom: "final-rereview.md F10" },
  "B-final-F11": { disposition: "fixed", citedFrom: "final-rereview.md F11 (dropped by secret filter)" },
  "B-final-F12": { disposition: "fixed", citedFrom: "final-rereview.md F12" },
};

const TABLE: Record<string, DispositionEntry> = { ...SET_A, ...SET_B };

export function dispositionFor(id: string): DispositionEntry {
  const entry = TABLE[id];
  if (!entry) {
    throw new Error(
      `dispositionFor: no curated disposition for id ${JSON.stringify(id)} — the extractor's id ` +
        "scheme changed, or this is a finding dispositions.ts has not been read against yet.",
    );
  }
  return entry;
}

export function allCuratedIds(): string[] {
  return Object.keys(TABLE);
}
