// TDD, per CLAUDE.md: written before `COST` in `score.ts` names every off-diagonal cell — RED
// until it does.
//
// Stream J Task T0: `docs/notes/2026-09-22-cost-matrices.md` (ratified by Quinn 2026-09-22, "looks
// good") works out every off-diagonal cell of the event 3x3 (§3) and the email 5x5 (§4) from five
// stated principles; the controller's dispatch ruled the twelve `unsure` cells (event, still a 4x4
// with `unsure` counted) and the ten `completion` cells (email, a 6x6) that the note's matrices
// don't cover. This file writes out that full ruling as its OWN table — independently of
// `score.ts`'s `COST` map — and checks every one of the 42 cells against it, then checks that no
// off-diagonal pair of either kind is missing from `COST` at all.
import { assertEquals } from "@std/assert";
import { EMAIL_TIERS, EVENT_VERDICTS } from "../supabase/functions/_shared/judge_validate.ts";
import { COST } from "./score.ts";

// The event 4x4 (obligation, opportunity, drop, unsure), 12 off-diagonal cells.
// obligation/opportunity/drop are `docs/notes/2026-09-22-cost-matrices.md` §3 (ratified); the six
// `unsure` cells are the controller's dispatch ruling (verbatim): "obligation->unsure 3 (NOT 2:
// while unsure events never surface, calling an obligation unsure hides it exactly like drop;
// revisit to 2 when surfacing lands), opportunity->unsure 1, drop->unsure 1, unsure->obligation 2,
// unsure->opportunity 1, unsure->drop 2."
const EVENT_TABLE: Record<string, number> = {
  "obligation->opportunity": 1,
  "obligation->drop": 3,
  "obligation->unsure": 3,
  "opportunity->obligation": 2,
  "opportunity->drop": 1,
  "opportunity->unsure": 1,
  "drop->obligation": 2,
  "drop->opportunity": 1,
  "drop->unsure": 1,
  "unsure->obligation": 2,
  "unsure->opportunity": 1,
  "unsure->drop": 2,
};

// The email 6x6 (task, borderline, event, opportunity, information, completion), 30 off-diagonal
// cells. task/borderline/event/opportunity/information are the note's §4 (ratified); the ten
// `completion` cells are the controller's dispatch ruling (verbatim): "task->completion 3,
// completion->task 2, completion->information 1, information->completion 2, completion->borderline
// 1, completion->event 1, completion->opportunity 1, borderline->completion 2, event->completion 2,
// opportunity->completion 3."
const EMAIL_TABLE: Record<string, number> = {
  "task->borderline": 1,
  "task->event": 2,
  "task->opportunity": 2,
  "task->information": 3,
  "task->completion": 3,
  "borderline->task": 1,
  "borderline->event": 1,
  "borderline->opportunity": 1,
  "borderline->information": 2,
  "borderline->completion": 2,
  "event->task": 2,
  "event->borderline": 1,
  "event->opportunity": 1,
  "event->information": 2,
  "event->completion": 2,
  "opportunity->task": 2,
  "opportunity->borderline": 1,
  "opportunity->event": 1,
  "opportunity->information": 3,
  "opportunity->completion": 3,
  "information->task": 2,
  "information->borderline": 1,
  "information->event": 1,
  "information->opportunity": 1,
  "information->completion": 2,
  "completion->task": 2,
  "completion->borderline": 1,
  "completion->event": 1,
  "completion->opportunity": 1,
  "completion->information": 1,
};

Deno.test("every ruled event cell (12) matches COST exactly", () => {
  const keys = Object.keys(EVENT_TABLE);
  assertEquals(keys.length, 12);
  for (const key of keys) {
    assertEquals(COST[key], EVENT_TABLE[key], `COST["${key}"] should be ${EVENT_TABLE[key]}`);
  }
});

Deno.test("every ruled email cell (30) matches COST exactly", () => {
  const keys = Object.keys(EMAIL_TABLE);
  assertEquals(keys.length, 30);
  for (const key of keys) {
    assertEquals(COST[key], EMAIL_TABLE[key], `COST["${key}"] should be ${EMAIL_TABLE[key]}`);
  }
});

Deno.test("no off-diagonal event pair is missing from COST", () => {
  for (const truth of EVENT_VERDICTS) {
    for (const ours of EVENT_VERDICTS) {
      if (truth === ours) continue;
      const key = `${truth}->${ours}`;
      assertEquals(key in COST, true, `COST is missing "${key}"`);
    }
  }
});

Deno.test("no off-diagonal email pair is missing from COST", () => {
  for (const truth of EMAIL_TIERS) {
    for (const ours of EMAIL_TIERS) {
      if (truth === ours) continue;
      const key = `${truth}->${ours}`;
      assertEquals(key in COST, true, `COST is missing "${key}"`);
    }
  }
});

Deno.test("COST carries exactly 42 off-diagonal cells (12 event + 30 email), nothing extra", () => {
  assertEquals(Object.keys(COST).length, 42);
});
