import { assert, assertAlmostEquals, assertEquals } from "@std/assert";
import { validate } from "../supabase/functions/_shared/judge_validate.ts";
import { dryRunAnswer, main } from "./run_eval.ts";
import { type Case, failed, score } from "./score.ts";

function tasks(n: number, theirs: Record<string, unknown>): Case[] {
  return Array.from({ length: n }, () => ({ kind: "task" as const, theirs }));
}

Deno.test("task metrics are mean absolute error and two exact-match rates", () => {
  const cases = tasks(2, { effort_hours: 2, importance: 4, course: "cs-100" });
  const got = score("task", cases, [
    { effort_hours: 3, importance: 4, course: "cs-100" },
    { effort_hours: 2, importance: 2, course: null },
  ]);
  assertAlmostEquals(got[0].value, 0.5);
  assertEquals(got[1].value, 0.5);
  assertEquals(got[2].value, 0.5);
});

Deno.test("a missed obligation costs three times a missed opportunity", () => {
  const obligation: Case[] = [{ kind: "event", theirs: { verdict: "obligation" } }];
  const opportunity: Case[] = [{ kind: "event", theirs: { verdict: "opportunity" } }];
  const missedObligation = score("event", obligation, [{ verdict: "drop" }])[0].value;
  const missedOpportunity = score("event", opportunity, [{ verdict: "drop" }])[0].value;
  assertEquals(missedObligation, 0);
  assertAlmostEquals(missedOpportunity, 2 / 3);
});

Deno.test("a task called information costs three times an information called borderline", () => {
  const task: Case[] = [{ kind: "email", theirs: { tier: "task" } }];
  const info: Case[] = [{ kind: "email", theirs: { tier: "information" } }];
  assertEquals(score("email", task, [{ tier: "information" }])[0].value, 0);
  assertAlmostEquals(score("email", info, [{ tier: "borderline" }])[0].value, 2 / 3);
});

Deno.test("a perfect run scores 1 and an empty set scores nothing at all", () => {
  const cases: Case[] = [{ kind: "event", theirs: { verdict: "drop" } }];
  assertEquals(score("event", cases, [{ verdict: "drop" }])[0].value, 1);
  assertEquals(score("event", [], []), []);
});

Deno.test("one point below a threshold fails and one point above passes", () => {
  const higher = { metric: "weighted_exact", value: 0.749, higherIsBetter: true };
  const lower = { metric: "effort_mae", value: 1.51, higherIsBetter: false };
  assertEquals(failed(higher, 0.75), true);
  assertEquals(failed({ ...higher, value: 0.751 }, 0.75), false);
  assertEquals(failed(lower, 1.5), true);
  assertEquals(failed({ ...lower, value: 1.49 }, 1.5), false);
  // Exactly at the threshold is a PASS: a gate at the observed value fires on noise.
  assertEquals(failed({ ...higher, value: 0.75 }, 0.75), false);
});

Deno.test("a null answer scores as wrong rather than throwing", () => {
  const cases: Case[] = [{ kind: "event", theirs: { verdict: "obligation" } }];
  assertEquals(score("event", cases, [null])[0].value, 0);
});

// ---------------------------------------------------------------------------------------------
// Ruling R-C2-E50 (2): a normal run with zero cases is a stated, PASSING outcome, and it must
// never touch a secret or a connection setting to get there — this is what lets `eval-gate`
// (hand-off H8, part 2) pass green on a PR that sets none of its three secrets. The real,
// committed `cloud/eval/seed/` directory is empty (Task 13, ruling R-C2-E12), so this drives
// `main()` against the genuine seed on disk rather than a mock, and proves the actual CI path.
// ---------------------------------------------------------------------------------------------

Deno.test("a normal run with zero cases prints the exact message, exits 0, and reads no secret", async () => {
  const throwing = (name: string): string => {
    throw new Error(`must not read '${name}' when there are no cases to score`);
  };
  const original = console.log;
  const logged: string[] = [];
  console.log = (...args: unknown[]) => {
    logged.push(args.map(String).join(" "));
  };
  let code: number;
  try {
    // No --load-seed, no --dry-run, no --thresholds: none of it should matter, because the
    // zero-case check runs before any of these other switches are even consulted.
    code = await main([], throwing);
  } finally {
    console.log = original;
  }
  assertEquals(code, 0);
  assertEquals(logged, ["0 cases — nothing to score"]);
});

Deno.test("the same zero-case exit holds with --dry-run and --thresholds present", async () => {
  const throwing = (name: string): string => {
    throw new Error(`must not read '${name}' when there are no cases to score`);
  };
  const original = console.log;
  const logged: string[] = [];
  console.log = (...args: unknown[]) => {
    logged.push(args.map(String).join(" "));
  };
  let code: number;
  try {
    code = await main(["--dry-run", "--thresholds", "cloud/eval/thresholds.json"], throwing);
  } finally {
    console.log = original;
  }
  assertEquals(code, 0);
  assertEquals(logged, ["0 cases — nothing to score"]);
});

// ---------------------------------------------------------------------------------------------
// Ruling R-C2-E10: the dry run's scripted answer must be a COMPLETE verdict for its kind, so the
// real `validate()` accepts it, and the real `score()` must then read every metric as exactly
// 1.0 because the labelled fields came straight from `theirs`. No model, no network — two or
// three inline synthetic cases per kind through the real functions.
// ---------------------------------------------------------------------------------------------

Deno.test("R-C2-E10: the dry-run answer validates and scores 1.0 exactly, per kind", () => {
  const taskTheirsA = { effort_hours: 2, importance: 4, course: "cs-101" };
  const taskSeedA = { known_courses: ["cs-101"] };
  const taskCheckedA = validate("task", dryRunAnswer("task", taskTheirsA), taskSeedA);
  assert(taskCheckedA.ok, "the dry-run task answer must validate");

  const taskTheirsB = { effort_hours: 0.5, importance: 1, course: null };
  const taskCheckedB = validate("task", dryRunAnswer("task", taskTheirsB), {});
  assert(taskCheckedB.ok, "a task label with no course must still validate");

  const taskScores = score("task", [
    { kind: "task", theirs: taskTheirsA },
    { kind: "task", theirs: taskTheirsB },
  ], [taskCheckedA.verdict ?? null, taskCheckedB.verdict ?? null]);
  assertEquals(taskScores.find((s) => s.metric === "effort_mae")?.value, 0);
  assertEquals(taskScores.find((s) => s.metric === "importance_exact")?.value, 1);
  assertEquals(taskScores.find((s) => s.metric === "course_exact")?.value, 1);

  const eventTheirs = { verdict: "obligation" };
  const eventChecked = validate("event", dryRunAnswer("event", eventTheirs), {});
  assert(eventChecked.ok, "the dry-run event answer must validate");
  const eventScores = score("event", [{ kind: "event", theirs: eventTheirs }], [eventChecked.verdict ?? null]);
  assertEquals(eventScores[0].value, 1);

  const emailTheirs = { tier: "borderline" };
  const emailChecked = validate("email", dryRunAnswer("email", emailTheirs), {});
  assert(emailChecked.ok, "the dry-run email answer must validate");
  const emailScores = score("email", [{ kind: "email", theirs: emailTheirs }], [emailChecked.verdict ?? null]);
  assertEquals(emailScores[0].value, 1);
});

// ---------------------------------------------------------------------------------------------
// The thresholds file carries a top-level "_note" documenting that its numbers are provisional
// (ruling R-C2-E50, step 6). The reader must ignore it — this reads the real, committed file and
// proves the per-kind numbers still resolve correctly with "_note" sitting alongside them.
// ---------------------------------------------------------------------------------------------

Deno.test("thresholds.json's _note is provisional documentation, ignored by the reader", async () => {
  const raw = await Deno.readTextFile(new URL("./thresholds.json", import.meta.url));
  const thresholds = JSON.parse(raw) as Record<string, unknown>;
  assertEquals(typeof thresholds._note, "string");
  const task = thresholds.task as Record<string, number>;
  assertEquals(task.effort_mae_max, 1.5);
  assertEquals(task.importance_exact_min, 0.55);
  assertEquals(task.course_exact_min, 0.85);
  const event = thresholds.event as Record<string, number>;
  assertEquals(event.weighted_exact_min, 0.75);
  const email = thresholds.email as Record<string, number>;
  assertEquals(email.weighted_exact_min, 0.70);
});
