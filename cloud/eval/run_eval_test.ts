import { assert, assertEquals, assertAlmostEquals } from "@std/assert";
import { ModelRefused, ScriptedModel } from "../supabase/functions/_shared/judge_anthropic.ts";
import type { Db } from "../supabase/functions/_shared/judge_db.ts";
import { validate } from "../supabase/functions/_shared/judge_validate.ts";
import { dryRunAnswer, main, MAX_ATTEMPTS, RETRY_BACKOFF_MS } from "./run_eval.ts";
import type { SeedRecord } from "./schema.ts";
import { type Case, failed, score } from "./score.ts";

function tasks(n: number, theirs: Record<string, unknown>): Case[] {
  return Array.from({ length: n }, () => ({ kind: "task" as const, theirs }));
}

function fakeDb(overrides: Partial<Db> = {}): Db {
  return {
    select: () => Promise.resolve([]),
    insert: () => Promise.resolve(null),
    update: () => Promise.resolve(),
    rpc: () => Promise.resolve(null),
    ...overrides,
  };
}

function captureConsole(): { log: string[]; error: string[]; restore: () => void } {
  const originalLog = console.log;
  const originalError = console.error;
  const log: string[] = [];
  const error: string[] = [];
  console.log = (...args: unknown[]) => {
    log.push(args.map(String).join(" "));
  };
  console.error = (...args: unknown[]) => {
    error.push(args.map(String).join(" "));
  };
  return {
    log,
    error,
    restore: () => {
      console.log = originalLog;
      console.error = originalError;
    },
  };
}

Deno.test("task metrics are mean absolute error and two exact-match rates", () => {
  const cases = tasks(2, { effort_hours: 2, importance: 4, course: "cs-100" });
  const got = score("task", cases, [
    { effort_hours: 3, importance: 4, course: "cs-100" },
    { effort_hours: 2, importance: 2, course: null },
  ]);
  assertAlmostEquals(got[0].value as number, 0.5);
  assertEquals(got[1].value, 0.5);
  assertEquals(got[2].value, 0.5);
});

Deno.test("a missed obligation costs three times a missed opportunity", () => {
  const obligation: Case[] = [{ kind: "event", theirs: { verdict: "obligation" } }];
  const opportunity: Case[] = [{ kind: "event", theirs: { verdict: "opportunity" } }];
  const missedObligation = score("event", obligation, [{ verdict: "drop" }])[0].value;
  const missedOpportunity = score("event", opportunity, [{ verdict: "drop" }])[0].value;
  assertEquals(missedObligation, 0);
  assertAlmostEquals(missedOpportunity as number, 2 / 3);
});

Deno.test("a task called information costs three times an information called borderline", () => {
  const task: Case[] = [{ kind: "email", theirs: { tier: "task" } }];
  const info: Case[] = [{ kind: "email", theirs: { tier: "information" } }];
  assertEquals(score("email", task, [{ tier: "information" }])[0].value, 0);
  assertAlmostEquals(score("email", info, [{ tier: "borderline" }])[0].value as number, 2 / 3);
});

Deno.test("a perfect run scores 1 and an empty set scores nothing at all", () => {
  const cases: Case[] = [{ kind: "event", theirs: { verdict: "drop" } }];
  assertEquals(score("event", cases, [{ verdict: "drop" }])[0].value, 1);
  assertEquals(score("event", [], []), []);
});

Deno.test("one point below a threshold fails and one point above passes", () => {
  const higher = { metric: "weighted_exact", value: 0.749, higherIsBetter: true, n: 1 };
  const lower = { metric: "effort_mae", value: 1.51, higherIsBetter: false, n: 1 };
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

Deno.test("a metric with nothing labelling it is null and never fails; NaN always fails", () => {
  const scored = { metric: "effort_mae", value: null, higherIsBetter: false, n: 0 };
  assertEquals(failed(scored, 1.5), false);
  const nan = { metric: "effort_mae", value: NaN, higherIsBetter: false, n: 1 };
  assertEquals(failed(nan, 1.5), true);
  const nanHigher = { metric: "weighted_exact", value: NaN, higherIsBetter: true, n: 1 };
  assertEquals(failed(nanHigher, 0.5), true);
});

// ---------------------------------------------------------------------------------------------
// R-C2-E51 fix 1, finding 1: Task 13's schema accepts a `theirs` that labels any non-empty
// SUBSET of a kind's scorable fields (a real single-field correction). A case must contribute
// only to the metric(s) it actually labels — never credited, never penalised on a field it never
// spoke to — and a metric nothing labels reports `null`/`n: 0` rather than a number.
// ---------------------------------------------------------------------------------------------

Deno.test("a case that labels only one task field contributes only to that field's metric", () => {
  const effortOnly: Case = { kind: "task", theirs: { effort_hours: 2 } };
  const importanceOnly: Case = { kind: "task", theirs: { importance: 4 } };
  const courseOnly: Case = { kind: "task", theirs: { course: "cs-101" } };
  const scored = score("task", [effortOnly, importanceOnly, courseOnly], [
    { effort_hours: 2 }, // exact match
    { importance: 4 }, // exact match
    { course: "cs-101" }, // exact match
  ]);
  const effort = scored.find((s) => s.metric === "effort_mae")!;
  const importance = scored.find((s) => s.metric === "importance_exact")!;
  const course = scored.find((s) => s.metric === "course_exact")!;
  assertEquals(effort.n, 1);
  assertEquals(effort.value, 0);
  assertEquals(importance.n, 1);
  assertEquals(importance.value, 1);
  assertEquals(course.n, 1);
  assertEquals(course.value, 1);
});

Deno.test("a task case that labels nothing scorable scores no metric at all", () => {
  // Structurally impossible under `validateSeedRecord` (theirs must be non-empty), but score()
  // itself must not crash or silently credit a field nobody labelled.
  const nothing: Case = { kind: "task", theirs: {} };
  const scored = score("task", [nothing], [{ effort_hours: 99, importance: 1, course: "x" }]);
  for (const s of scored) {
    assertEquals(s.n, 0);
    assertEquals(s.value, null);
  }
});

Deno.test("an email case that labels only 'course' (never 'tier') contributes nothing to weighted_exact", () => {
  const courseOnly: Case = { kind: "email", theirs: { course: "cs-101" } };
  const scored = score("email", [courseOnly], [{ tier: "information", course: "cs-101" }]);
  assertEquals(scored[0].n, 0);
  assertEquals(scored[0].value, null);
});

// ---------------------------------------------------------------------------------------------
// Ruling R-C2-E50 (2): a normal run with zero cases is a stated, PASSING outcome, and it must
// never touch a secret or a connection setting to get there — this is what lets `eval-gate`
// (hand-off H8, part 2) pass green on a PR that sets none of its secrets. T2 (stream J,
// 2026-09-22) filled `cloud/eval/seed/events.jsonl`, so this path is no longer exercised by the
// REAL, committed seed (Task 13, ruling R-C2-E12's empty state) — the four tests below inject an
// empty `loadSeed` instead, so they still test the code path `main()` takes when its seed truly
// has nothing, independent of what is on disk today.
// ---------------------------------------------------------------------------------------------

// A-4: the ONE env read the empty-seed path is now allowed — `SUPABASE_SERVICE_ROLE_KEY`, to ask
// whether it is even worth asking the database — answered here as absent, exactly as a fork PR
// with no secrets sees it. Anything else read is still a bug this fake catches by throwing.
function throwingExceptServiceKey(): (name: string) => string | undefined {
  return (name: string) => {
    if (name === "SUPABASE_SERVICE_ROLE_KEY") return undefined;
    throw new Error(`must not read '${name}' when there are no cases to score and no service key`);
  };
}

Deno.test("a normal run with zero cases and no service-role key prints the exact message, exits 0, and reads no other secret", async () => {
  const captured = captureConsole();
  let code: number;
  try {
    // No --load-seed, no --dry-run, no --thresholds: none of it should matter, because the
    // zero-case check runs before any of these other switches are even consulted. `loadSeed` is
    // injected as empty rather than read from disk — `cloud/eval/seed/events.jsonl` is real cases
    // now (T2), and this test is about the zero-case CODE PATH, not about what is committed today.
    code = await main([], { envGet: throwingExceptServiceKey(), loadSeed: () => Promise.resolve([]) });
  } finally {
    captured.restore();
  }
  assertEquals(code, 0);
  assertEquals(captured.log, ["0 cases — nothing to score (no seed, no database access)"]);
});

Deno.test("the same zero-case exit holds with --dry-run and --thresholds present", async () => {
  const captured = captureConsole();
  let code: number;
  try {
    code = await main(
      ["--dry-run", "--thresholds", "cloud/eval/thresholds.json"],
      { envGet: throwingExceptServiceKey(), loadSeed: () => Promise.resolve([]) },
    );
  } finally {
    captured.restore();
  }
  assertEquals(code, 0);
  assertEquals(captured.log, ["0 cases — nothing to score (no seed, no database access)"]);
});

// ---------------------------------------------------------------------------------------------
// A-4: an empty local seed with a service-role key present and NOTHING in `eval_cases` either
// must still answer the same "nothing to score" — the key alone is not cases.
// ---------------------------------------------------------------------------------------------

Deno.test("an empty seed with a service-role key but an empty database still exits 0 with nothing to score", async () => {
  let dbCalls = 0;
  const db = fakeDb();
  const captured = captureConsole();
  let code: number;
  try {
    code = await main([], {
      envGet: (name) => (name === "SUPABASE_SERVICE_ROLE_KEY" ? "service-role-not-a-secret" : undefined),
      loadSeed: () => Promise.resolve([]),
      db: () => {
        dbCalls += 1;
        return db;
      },
    });
  } finally {
    captured.restore();
  }
  assertEquals(code, 0);
  assertEquals(captured.log, ["0 cases — nothing to score (no seed, no database access)"]);
  assertEquals(dbCalls, 1, "the count check reaches the database exactly once");
});

// ---------------------------------------------------------------------------------------------
// A-4: the gate's whole point — an empty LOCAL seed with real rows in `eval_cases` (the shape a
// consented correction takes once C4's (c) opt-in exists) must wake the gate up, not answer
// "nothing to score" forever.
// ---------------------------------------------------------------------------------------------

Deno.test("an empty seed with a service-role key and cases already in eval_cases runs the suite for real", async () => {
  const consentedTheirs = { effort_hours: 2, importance: 4, course: "cs-101" };
  const evalRunsInserted: Array<Record<string, unknown>> = [];
  const db: Db = fakeDb({
    select: (path: string) => {
      if (path === "eval_cases?select=id&limit=1") {
        // The database has ONE consented case (no local seed names it at all) — this is what a
        // fork of no seed and a live `eval_cases` corpus actually looks like.
        return Promise.resolve([{ id: 1 }]);
      }
      if (path.startsWith("eval_cases?kind=eq.task")) {
        return Promise.resolve([{
          id: 1,
          kind: "task",
          request: { kind: "task", item: { id: "x", title: "t" }, heuristics_seed: { known_courses: ["cs-101"] } },
          ours: null,
          theirs: consentedTheirs,
        }]);
      }
      if (path.startsWith("eval_cases?kind=eq.")) return Promise.resolve([]);
      if (path.startsWith("models?kind=eq.task")) {
        return Promise.resolve([{
          kind: "task", provider: "anthropic", model_id: "claude-haiku-4-5",
          prompt_version: "v1", grammar_version: "v1", max_tokens: 256, sampling: {},
          usd_per_m_in: 1, usd_per_m_out: 5,
        }]);
      }
      return Promise.resolve([]);
    },
    insert: (table: string, row: Record<string, unknown>) => {
      if (table === "eval_runs") evalRunsInserted.push(row);
      return Promise.resolve(null);
    },
  });
  const captured = captureConsole();
  let code: number;
  try {
    code = await main(
      ["--dry-run", "--thresholds", "cloud/eval/thresholds.json"],
      {
        db: () => db,
        loadSeed: () => Promise.resolve([]),
        envGet: (name) => (name === "SUPABASE_SERVICE_ROLE_KEY" ? "service-role-not-a-secret" : undefined),
      },
    );
  } finally {
    captured.restore();
  }
  assertEquals(code, 0);
  assertEquals(captured.log.filter((l) => l.includes("nothing to score")), [], "the gate must not have short-circuited");
  // task's three metrics, all fully labelled by consentedTheirs, each score 1.0/0 exactly and write a row —
  // proof the run actually scored the case a fake model answered, not merely that it declined to exit early.
  assertEquals(evalRunsInserted.length, 3);
});

// ---------------------------------------------------------------------------------------------
// R-C2-E51 fix 1, finding 2: a non-empty LOCAL seed with a kind the database corpus does not
// hold (because `--load-seed` was never run against it) must not be a silent skip — the gate
// would go green having scored nothing. Tested against a fake `Db` so no real connection is
// needed either way.
// ---------------------------------------------------------------------------------------------

const SYNTHETIC_SEED: SeedRecord[] = [
  {
    id: "seed-0001",
    kind: "task",
    request: { kind: "task", item: { id: "x", title: "t" }, heuristics_seed: { known_courses: ["cs-101"] } },
    theirs: { effort_hours: 1 },
  },
];

Deno.test("a non-empty local seed with an unloaded corpus fails the gate, naming --load-seed", async () => {
  const db = fakeDb();
  const captured = captureConsole();
  let code: number;
  try {
    code = await main(
      ["--dry-run", "--thresholds", "cloud/eval/thresholds.json"],
      { db: () => db, loadSeed: () => Promise.resolve(SYNTHETIC_SEED), envGet: () => undefined },
    );
  } finally {
    captured.restore();
  }
  assertEquals(code, 2);
  assert(
    captured.error.some((l) => l.includes("task") && l.includes("--load-seed")),
    `expected an error naming task and --load-seed, got: ${JSON.stringify(captured.error)}`,
  );
  // event/email were never in the seed at all, so their absence from `eval_cases` is a legitimate
  // skip, not the same failure — the corpus-not-loaded message must name only "task".
  assert(!captured.error.some((l) => l.includes("event") || l.includes("email")));
});

Deno.test("a seeded, loaded corpus scores normally under --dry-run, stamping one run_id and dry_run on every row", async () => {
  const seedTheirs = { effort_hours: 2, importance: 4, course: "cs-101" };
  const records: SeedRecord[] = [
    {
      id: "seed-0001",
      kind: "task",
      request: { kind: "task", item: { id: "x", title: "t" }, heuristics_seed: { known_courses: ["cs-101"] } },
      theirs: seedTheirs,
    },
  ];
  const evalRunsInserted: Array<Record<string, unknown>> = [];
  const db: Db = fakeDb({
    select: (path: string) => {
      if (path.startsWith("eval_cases?kind=eq.task")) {
        return Promise.resolve([{ id: 1, kind: "task", request: records[0].request, ours: null, theirs: seedTheirs }]);
      }
      if (path.startsWith("eval_cases?kind=eq.")) return Promise.resolve([]);
      if (path.startsWith("models?kind=eq.task")) {
        return Promise.resolve([{
          kind: "task",
          provider: "anthropic",
          model_id: "claude-haiku-4-5",
          prompt_version: "v1",
          grammar_version: "v1",
          max_tokens: 256,
          sampling: {},
          usd_per_m_in: 1,
          usd_per_m_out: 5,
        }]);
      }
      return Promise.resolve([]);
    },
    insert: (table: string, row: Record<string, unknown>) => {
      if (table === "eval_runs") evalRunsInserted.push(row);
      return Promise.resolve(null);
    },
  });
  const captured = captureConsole();
  let code: number;
  try {
    code = await main(
      ["--dry-run", "--thresholds", "cloud/eval/thresholds.json"],
      { db: () => db, loadSeed: () => Promise.resolve(records), envGet: () => undefined },
    );
  } finally {
    captured.restore();
  }
  assertEquals(code, 0);
  assertEquals(captured.error.filter((l) => l.includes("--load-seed")), []);
  // task's three metrics, all fully labelled by seedTheirs, each score 1.0/0 exactly and write a row.
  assertEquals(evalRunsInserted.length, 3);
  const runIds = new Set(evalRunsInserted.map((r) => r.run_id));
  assertEquals(runIds.size, 1, "every row of one run must share the same run_id");
  assert(typeof [...runIds][0] === "string" && ([...runIds][0] as string).length > 0);
  for (const row of evalRunsInserted) {
    assertEquals(row.dry_run, true);
  }
});

// ---------------------------------------------------------------------------------------------
// R-C2-E51 fix 1, finding 5: `--load-seed` is idempotent — it inserts only the `seed_id`s not
// already present among `source = 'seed'` rows, and reports both counts.
// ---------------------------------------------------------------------------------------------

Deno.test("--load-seed inserts only missing seed_ids and reports both counts", async () => {
  const records: SeedRecord[] = [
    {
      id: "seed-0001",
      kind: "task",
      request: { kind: "task", item: {}, heuristics_seed: {} },
      theirs: { effort_hours: 1 },
    },
    {
      id: "seed-0002",
      kind: "task",
      request: { kind: "task", item: {}, heuristics_seed: {} },
      theirs: { effort_hours: 2 },
    },
  ];
  const inserted: Array<Record<string, unknown>> = [];
  const db: Db = fakeDb({
    select: (path: string) => {
      if (path.startsWith("eval_cases?source=eq.seed")) return Promise.resolve([{ seed_id: "seed-0001" }]);
      return Promise.resolve([]);
    },
    insert: (_table: string, row: Record<string, unknown>) => {
      inserted.push(row);
      return Promise.resolve(null);
    },
  });
  const captured = captureConsole();
  let code: number;
  try {
    code = await main(["--load-seed"], { db: () => db, loadSeed: () => Promise.resolve(records) });
  } finally {
    captured.restore();
  }
  assertEquals(code, 0);
  assertEquals(inserted.length, 1);
  assertEquals(inserted[0].seed_id, "seed-0002");
  assertEquals(captured.log, ["1 inserted, 1 already present"]);
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

Deno.test("R-C2-E10 + partial labels: a single-field label still scores 1.0 on a dry run, per kind", () => {
  const seed = { known_courses: ["cs-101"] };

  const effortOnly = { effort_hours: 2 };
  const effortChecked = validate("task", dryRunAnswer("task", effortOnly), seed);
  assert(effortChecked.ok);
  const effortScored = score("task", [{ kind: "task", theirs: effortOnly }], [effortChecked.verdict ?? null]);
  const effortMetric = effortScored.find((s) => s.metric === "effort_mae")!;
  assertEquals(effortMetric.n, 1);
  assertEquals(effortMetric.value, 0);
  for (const s of effortScored.filter((s) => s.metric !== "effort_mae")) {
    assertEquals(s.n, 0);
    assertEquals(s.value, null);
  }

  const importanceOnly = { importance: 4 };
  const importanceChecked = validate("task", dryRunAnswer("task", importanceOnly), seed);
  assert(importanceChecked.ok);
  const importanceScored = score("task", [{ kind: "task", theirs: importanceOnly }], [importanceChecked.verdict ?? null]);
  assertEquals(importanceScored.find((s) => s.metric === "importance_exact")?.value, 1);

  const courseOnly = { course: "cs-101" };
  const courseChecked = validate("task", dryRunAnswer("task", courseOnly), seed);
  assert(courseChecked.ok);
  const courseScored = score("task", [{ kind: "task", theirs: courseOnly }], [courseChecked.verdict ?? null]);
  assertEquals(courseScored.find((s) => s.metric === "course_exact")?.value, 1);

  // An email case that labels only "course" (never "tier") must validate fine but contribute
  // NOTHING to weighted_exact — the only metric `score()` computes for email.
  const emailCourseOnly = { course: "cs-101" };
  const emailChecked = validate("email", dryRunAnswer("email", emailCourseOnly), seed);
  assert(emailChecked.ok);
  const emailScored = score("email", [{ kind: "email", theirs: emailCourseOnly }], [emailChecked.verdict ?? null]);
  assertEquals(emailScored[0].n, 0);
  assertEquals(emailScored[0].value, null);
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
  // Final review item 5: the note must not still say the seed is empty once it holds cases.
  const seedEvents = (await Deno.readTextFile(new URL("./seed/events.jsonl", import.meta.url)))
    .split("\n").filter((l) => l.trim() !== "").length;
  const note = thresholds._note as string;
  assert(!note.includes("the seed is empty"), note);
  assert(note.includes(`${seedEvents} event cases`), `the note should count the seed's ${seedEvents} event cases: ${note}`);
  const task = thresholds.task as Record<string, number>;
  assertEquals(task.effort_mae_max, 1.5);
  assertEquals(task.importance_exact_min, 0.55);
  assertEquals(task.course_exact_min, 0.85);
  const event = thresholds.event as Record<string, number>;
  assertEquals(event.weighted_exact_min, 0.75);
  const email = thresholds.email as Record<string, number>;
  assertEquals(email.weighted_exact_min, 0.70);
});

// Final review item 2: the eval calls `judge()` directly, and `judge()` only answers `unsure` to a
// request that declares it — so the eval must declare it too, or every `unsure`-labelled event
// case would score as a miss against the pre-T1 `below floor` shape.
Deno.test("an unsure-labelled event case scores 1.0 on a dry run, so the eval declares accepts unsure", async () => {
  const theirs = { verdict: "unsure" };
  const request = { kind: "event" as const, item: { uid: "engage:1", title: "AI Club Kickoff" }, heuristics_seed: {} };
  const values: number[] = [];
  const db: Db = fakeDb({
    select: (path: string) => {
      if (path.startsWith("eval_cases?kind=eq.event")) {
        return Promise.resolve([{ id: 1, kind: "event", request, ours: null, theirs }]);
      }
      if (path.startsWith("eval_cases?kind=eq.")) return Promise.resolve([]);
      if (path.startsWith("models?kind=eq.event")) {
        return Promise.resolve([{
          kind: "event", provider: "anthropic", model_id: "claude-haiku-4-5", prompt_version: "event-3",
          grammar_version: "event-3", max_tokens: 256, sampling: {}, usd_per_m_in: 1, usd_per_m_out: 5,
        }]);
      }
      return Promise.resolve([]);
    },
    insert: (table: string, row: Record<string, unknown>) => {
      if (table === "eval_runs") values.push(row.value as number);
      return Promise.resolve(null);
    },
  });
  const records: SeedRecord[] = [{ id: "seed-e1", kind: "event", request, theirs }];
  const captured = captureConsole();
  try {
    await main(
      ["--dry-run", "--thresholds", "cloud/eval/thresholds.json"],
      { db: () => db, loadSeed: () => Promise.resolve(records), envGet: () => undefined },
    );
  } finally {
    captured.restore();
  }
  assertEquals(values, [1]);
});

// ---------------------------------------------------------------------------------------------
// PR #33's eval-gate (2026-10-02) scored event.weighted_exact 0.564 in CI against 0.897-0.936 on
// local and re-run replays of the same seed: provider blips (`model failed` — the model never
// answered) were scored as the worst miss. A failed call is retried; a case still failing is
// unscored and counted; past the bound the kind is a provider error, never a quality result. A
// refusal or a truncation is the model answering and stays scored exactly as before.
// ---------------------------------------------------------------------------------------------

const ANSWER = { confidence: 1, why: "scripted", verdict: "obligation" };

/** `n` event cases labelled `obligation`, a live (non-dry) run against `script`, no key, no sleep. */
async function liveEventRun(n: number, script: Array<Record<string, unknown> | Error>) {
  const request = { kind: "event" as const, item: { uid: "engage:1", title: "Lab report due" }, heuristics_seed: {} };
  const rows = Array.from({ length: n }, (_, i) => ({ id: i + 1, kind: "event", request, ours: null, theirs: { verdict: "obligation" } }));
  const evalRuns: Array<Record<string, unknown>> = [];
  const db: Db = fakeDb({
    select: (path: string) => {
      if (path.startsWith("eval_cases?kind=eq.event")) return Promise.resolve(rows);
      if (path.startsWith("eval_cases?kind=eq.")) return Promise.resolve([]);
      if (path.startsWith("models?kind=eq.event")) {
        return Promise.resolve([{
          kind: "event", provider: "openrouter", model_id: "scripted/model", prompt_version: "event-3",
          grammar_version: "event-3", max_tokens: 256, sampling: {}, usd_per_m_in: 1, usd_per_m_out: 5,
        }]);
      }
      return Promise.resolve([]);
    },
    insert: (table: string, row: Record<string, unknown>) => {
      if (table === "eval_runs") evalRuns.push(row);
      return Promise.resolve(null);
    },
  });
  const model = new ScriptedModel(script);
  const sleeps: number[] = [];
  const records: SeedRecord[] = [{ id: "seed-e1", kind: "event", request, theirs: { verdict: "obligation" } }];
  const captured = captureConsole();
  let code: number;
  try {
    code = await main(["--thresholds", "cloud/eval/thresholds.json"], {
      db: () => db,
      loadSeed: () => Promise.resolve(records),
      envGet: () => undefined,
      modelFor: () => model,
      sleep: (ms) => {
        sleeps.push(ms);
        return Promise.resolve();
      },
    });
  } finally {
    captured.restore();
  }
  return { code, evalRuns, calls: model.seen.length, sleeps, log: captured.log, error: captured.error };
}

const blip = () => new Error("provider returned 503");

Deno.test("a case whose call fails then answers is retried, and the answer is what is scored", async () => {
  assertEquals(MAX_ATTEMPTS, 3);
  const run = await liveEventRun(2, [blip(), blip(), ANSWER, ANSWER]);
  assertEquals(run.code, 0);
  assertEquals(run.calls, 4);
  assertEquals(run.sleeps, [RETRY_BACKOFF_MS, RETRY_BACKOFF_MS]);
  assertEquals(run.evalRuns.map((r) => [r.value, r.cases]), [[1, 2]]);
  assert(run.log.includes("event: 2 cases, <= 6 model calls (3 attempts per case at most, cap 200)"), run.log.join(" | "));
  assert(run.log.includes("event: 4 model calls made"), run.log.join(" | "));
  assert(!run.log.some((l) => l.includes("unscored")), run.log.join(" | "));
});

Deno.test("cases that never answer are unscored, and an outage fails as a provider error, not a quality result", async () => {
  const run = await liveEventRun(3, Array.from({ length: 9 }, blip));
  assertEquals(run.code, 2);
  assertEquals(run.calls, 9, "three attempts per case, never more");
  assertEquals(run.evalRuns, [], "an outage writes no eval_runs row");
  assert(run.log.includes("event: 3 of 3 cases unscored (model failed)"), run.log.join(" | "));
  assert(
    run.error.includes("event: provider error, 3 of 3 cases failed after 3 attempts each: not a quality result"),
    run.error.join(" | "),
  );
  assert(!run.log.some((l) => l.includes("FAIL")), "never reported as a failed metric");
});

Deno.test("a long outage stops calling once the bound is passed", async () => {
  // 30 cases allow 3 unscored; the fourth failed case decides the outcome, so the run stops there.
  const run = await liveEventRun(30, Array.from({ length: 90 }, blip));
  assertEquals(run.code, 2);
  assertEquals(run.calls, 12);
  assert(
    run.error.includes("event: provider error, 4 of 30 cases failed after 3 attempts each (stopped early): not a quality result"),
    run.error.join(" | "),
  );
  assertEquals(run.evalRuns, []);
});

Deno.test("failures within the bound are unscored and the kind is scored on the rest", async () => {
  const run = await liveEventRun(10, [blip(), blip(), blip(), ...Array.from({ length: 9 }, () => ANSWER)]);
  assertEquals(run.code, 0);
  assertEquals(run.evalRuns.map((r) => [r.value, r.cases]), [[1, 9]]);
  assert(run.log.includes("event: 1 of 10 cases unscored (model failed)"), run.log.join(" | "));
  assert(run.log.some((l) => l.startsWith("event.weighted_exact = 1.000") && l.includes("n=9 of 10 cases")), run.log.join(" | "));
  assertEquals(run.error, []);
});

Deno.test("a refusal is the model answering: scored as a miss, never retried", async () => {
  const run = await liveEventRun(1, [new ModelRefused("refused")]);
  assertEquals(run.code, 1);
  assertEquals(run.calls, 1);
  assertEquals(run.sleeps, []);
  assertEquals(run.evalRuns.map((r) => [r.value, r.cases]), [[0, 1]]);
  assert(!run.log.some((l) => l.includes("unscored")));
});

Deno.test("a truncation is the model answering: scored as a miss, never retried", async () => {
  const run = await liveEventRun(1, [new Error("stop_reason max_tokens")]);
  assertEquals(run.code, 1);
  assertEquals(run.calls, 1);
  assertEquals(run.evalRuns.map((r) => [r.value, r.cases]), [[0, 1]]);
});
