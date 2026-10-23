// DRAFT, NOT WIRED (task-3 model study, PR #61). Nothing imports this file but its test.
//
// A deterministic ceiling on a task's `effort_hours`, read from the title alone, for the cases the
// pinned model would not keep under task-3's own ceilings (on ba6a177's staging run: Midterm 8 h,
// Final Exam 24 h, a 12-page paper 60 h). It only ever LOWERS an estimate; it never raises one,
// never touches importance, and needs no model. Wiring it would be one call in `validate()`'s task
// branch (or `judge_pipeline.ts` after it) with the item's title, after the 0.25-40 clamp — and
// because the device applies the same clamps as `validate()` ("must not drift"), the wiring change
// is where that twin, and the eval gate, get decided.
//
// The rules, first match wins:
//   1. a stated page count ("12 pages", "10-12 pages") caps at 2 hours a page (the larger number);
//   2. a large-work word (project, paper, thesis, capstone, portfolio, dissertation) keeps the full
//      40, unless a small-part word (milestone, proposal, outline, draft, check-in) caps it at 8;
//   3. an exam word (exam, midterm, quiz, test) caps at 6;
//   4. anything else caps at 8.
//
// Failure modes, by design: a genuinely long item with a vague title ("Assignment 5" that is really a
// 15-hour report, "Final submission") is capped at 8 and under-estimated; a take-home exam that is
// really 10 hours is capped at 6; a page count only counts when it is in the title (a body that says
// "12 pages" does not raise a cap); "paper" in an unrelated sense ("Paper airplane lab") keeps 40.
// Under-estimates are the cheaper error for ranking (a task looks smaller and surfaces sooner), and
// the student's own edit always wins (judge once).

const LARGE = /\b(project|paper|thesis|capstone|portfolio|dissertation)\b/i;
const SMALL_PART = /\b(milestone|proposal|outline|draft|check-?in)\b/i;
const EXAM = /\b(exam|midterm|quiz|test)\b/i;
const PAGES = /(\d{1,3})\s*(?:[-–]\s*(\d{1,3})\s*)?pages?\b/i;

/** The most hours a task with this title may be estimated at (0.25-40 is `validate()`'s clamp). */
export function effortCeiling(title: string): number {
  const pages = PAGES.exec(title);
  if (pages !== null) {
    const most = Math.max(Number(pages[1]), Number(pages[2] ?? pages[1]));
    return Math.min(40, Math.max(0.25, 2 * most));
  }
  if (LARGE.test(title)) return SMALL_PART.test(title) ? 8 : 40;
  if (EXAM.test(title)) return 6;
  return 8;
}

/** `effort` lowered to the title's ceiling; never raised. */
export function guardEffort(effort: number, title: string): number {
  return Math.min(effort, effortCeiling(title));
}
