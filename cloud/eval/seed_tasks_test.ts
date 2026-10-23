// task-3 (effort anchors, 2026-10-23): the synthetic task seed. Every case in
// `cloud/eval/seed/tasks.jsonl` is hand-written and invented — no real student, course, item or
// institution — and is shaped like the low-information item that showed the defect: a learning-
// management feed item judged from a title, a course slug and a due date, with no body (a seed item
// may never carry one: `scrubViolations` bans the key). Each case labels `effort_hours` at the
// middle of its type's anchor range in the task-3 brief (the template now states one typical value), `course` as the slug the
// rules say to echo, and `importance: 3` only where no grade weights are given (the rule's own
// answer); a case with weights leaves importance unlabelled, because that is a judgment, not a rule.
//
// What it measures is rule-following on the anchors, not the real distribution of coursework:
// the labels come from the same anchors the prompt states. At this size it can rule a prompt or a
// model out (task-2's median of 8 hours on items like these scores far past `effort_mae_max`); it
// cannot rule one in.
import { assert, assertEquals } from "@std/assert";
import { loadSeed } from "./loader.ts";
import { scrubViolations, type SeedRecord, validateSeedRecord } from "./schema.ts";

async function taskSeed(): Promise<SeedRecord[]> {
  return (await loadSeed()).filter((r) => r.kind === "task");
}

const effort = (r: SeedRecord): number => Number(r.theirs.effort_hours);

Deno.test("the task seed loads and is between 15 and 40 cases", async () => {
  const tasks = await taskSeed();
  assert(tasks.length >= 15 && tasks.length <= 40, `expected 15-40 task cases, got ${tasks.length}`);
});

Deno.test("every task case passes validateSeedRecord and scrubViolations individually", async () => {
  const tasks = await taskSeed();
  assert(tasks.length > 0, "no task cases were loaded");
  for (const record of tasks) {
    assertEquals(validateSeedRecord(record), [], `seed-shape violations in ${record.id}`);
    assertEquals(scrubViolations(record), [], `scrub violations in ${record.id}`);
  }
});

Deno.test("every task id is unique and carries the seed-task- prefix", async () => {
  const ids = new Set<string>();
  for (const record of await taskSeed()) {
    assert(record.id.startsWith("seed-task-"), `${record.id} is missing the seed-task- prefix`);
    assert(!ids.has(record.id), `duplicate id ${record.id}`);
    ids.add(record.id);
  }
});

Deno.test("every task case's request is task_request's shape, less the body a seed never carries", async () => {
  const itemKeys = ["course", "created_by", "due", "id", "source_uid", "title"];
  const seedKeys = ["course", "effort_hours", "known_courses", "preferences", "slice_hours", "weights"];
  for (const record of await taskSeed()) {
    assertEquals(Object.keys(record.request.item).sort(), itemKeys, `${record.id}: request.item keys`);
    assertEquals(Object.keys(record.request.heuristics_seed).sort(), seedKeys, `${record.id}: heuristics_seed keys`);
    const known = record.request.heuristics_seed.known_courses as string[];
    const slug = record.request.heuristics_seed.course;
    assert(slug === null || known.includes(slug as string), `${record.id}: the seed's course is a known course`);
  }
});

Deno.test("every task case labels effort inside the clamp, and course as the slug given", async () => {
  for (const record of await taskSeed()) {
    const hours = effort(record);
    assert(hours >= 0.25 && hours <= 40, `${record.id}: effort ${hours} outside 0.25-40`);
    assertEquals(record.theirs.course, record.request.heuristics_seed.course, `${record.id}: course label`);
  }
});

Deno.test("importance is labelled 3 exactly where no weights are given, and never where they are", async () => {
  for (const record of await taskSeed()) {
    const weighted = record.request.heuristics_seed.weights !== "";
    if (weighted) assert(!("importance" in record.theirs), `${record.id}: a weighted case labels no importance`);
    else assertEquals(record.theirs.importance, 3, `${record.id}: no weights, so the rule's 3`);
  }
});

Deno.test("the seed is ordinary coursework: median effort at most 3 hours, and only stated-large items above 8", async () => {
  const tasks = await taskSeed();
  const sorted = tasks.map(effort).sort((a, b) => a - b);
  const median = sorted[Math.floor(sorted.length / 2)];
  assert(median <= 3, `median effort ${median} is not ordinary coursework`);
  for (const record of tasks.filter((r) => effort(r) > 8)) {
    const title = String(record.request.item.title).toLowerCase();
    assert(/project|paper/.test(title), `${record.id}: above 8 hours without a large project or paper in the title`);
  }
  assert(tasks.some((r) => effort(r) > 8), "no case covers a stated-large item (the above-8 rule)");
});

Deno.test("each anchor type and the vague-title rule are covered", async () => {
  const titles = (await taskSeed()).map((r) => String(r.request.item.title).toLowerCase());
  for (const word of ["reading", "discussion", "quiz", "homework", "problem set", "lab", "essay", "milestone", "exam"]) {
    assert(titles.some((t) => t.includes(word)), `no case covers '${word}'`);
  }
  const vague = (await taskSeed()).filter((r) => /^assignment \d+$/i.test(String(r.request.item.title)));
  assert(vague.length >= 1, "no vague-title case ('Assignment N')");
  for (const r of vague) assert(effort(r) <= 3, `${r.id}: a vague title is labelled the middle, not a large number`);
});

Deno.test("points and grade weights are covered", async () => {
  const tasks = await taskSeed();
  assert(tasks.some((r) => /\bpoints?\b|\bpts\b/i.test(String(r.request.item.title))), "no case names points");
  assert(tasks.some((r) => r.request.heuristics_seed.weights !== ""), "no case gives grade weights");
  assert(tasks.some((r) => r.request.heuristics_seed.course === null), "no case without a course slug");
});
