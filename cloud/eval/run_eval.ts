// The eval suite (§5.4 measure 2). Loads Task 13's seed, replays each case's `request` through the
// LIVE pipeline with the currently pinned model, scores the replies, writes one `eval_runs` row
// per metric, and exits 1 if any metric is worse than its threshold.
//
// Three switches:
//   --load-seed   insert Task 13's seed (`loadSeed`, `./loader.ts`) into `eval_cases` as
//                 `source = 'seed'` rows (skipping any `seed_id` already present), and exit.
//   --dry-run     score against a `ScriptedModel` seeded from `dryRunAnswer` below (ruling
//                 R-C2-E10), so the harness itself is tested with NO KEY AND NO SPEND.
//   --thresholds  the file the gate reads. Required once there is at least one case to score.
//
// Every run of this program without --dry-run spends real money (one model call per case, at
// roughly $0.0014 each — the arithmetic is in `judge_caps.ts`). `--dry-run` first, always.
//
// Ruling R-C2-E50 (2): `cloud/eval/seed/` is empty at merge (ruling R-C2-E12, Task 13) and, until
// C4's (c) opt-in ships a second path, it is the ONLY way a row ever lands in `eval_cases` — so an
// empty seed on disk means an empty case set, full stop. `loadSeed()` reads nothing but the local
// filesystem, so counting its result is the free, local, no-secret way to know there is nothing to
// do, and it is checked FIRST — before `--thresholds`, before `ANTHROPIC_API_KEY`, before
// `SUPABASE_URL`/`SUPABASE_SERVICE_ROLE_KEY` (both read inside `serviceDb()`), and before any
// connection opens or any `eval_runs` row is written. That is what lets `eval-gate` (hand-off H8,
// part 2) pass green on a PR that sets none of its secrets.
//
// R-C2-E51 fix 1, finding 2: a non-empty LOCAL seed that a kind's `eval_cases` corpus does not
// reflect (because `--load-seed` was never run against this database) is NOT the same as "nothing
// to do" — it is a gate that would otherwise go green having scored nothing. That case fails loud,
// naming `--load-seed`, instead of silently logging a skip.
//
// Ruling R-C2-E9: the eval suite is not a user, so it never spends an account's own daily cap or
// monthly budget — it passes the UNMETERED `EVAL_CAPS` below and records its OWN spend, summed
// from every `ModelReply`, as `input_tokens`/`output_tokens` on each `eval_runs` row instead.
// R-C2-E51 fix 1, finding 3: it is also not an ACCOUNT — every consumer of `judge()`'s `accountId`
// in this path is itself a stub (`rules.lookup`, `caps`, `log.write` below), so there is no real
// account to name and `EVAL_ACCOUNT_ID` named a secret that did nothing. `judge()` is called with
// the nil UUID instead.
import type { CapStore } from "../supabase/functions/_shared/judge_caps.ts";
import type { Db } from "../supabase/functions/_shared/judge_db.ts";
import { serviceDb } from "../supabase/functions/_shared/judge_db.ts";
import { AnthropicModel, type JudgeModel, ScriptedModel } from "../supabase/functions/_shared/judge_anthropic.ts";
import { modelRow } from "../supabase/functions/_shared/judge_models.ts";
import { judge, type Kind } from "../supabase/functions/_shared/judge_pipeline.ts";
import { promptHash } from "../supabase/functions/_shared/judge_prompts.ts";
import { loadSeed } from "./loader.ts";
import type { SeedRecord } from "./schema.ts";
import { type Case, failed, score } from "./score.ts";

const KINDS: Kind[] = ["task", "event", "email"];

/**
 * The ceiling on how many cases ONE run will score, per kind (C2 final review S-5).
 *
 * Every non-`--dry-run` case is one real model call at roughly $0.0014 (the arithmetic is in
 * `judge_caps.ts`), and the corpus is unbounded by construction: `eval_cases` grows with every
 * consented correction, and this gate runs on every PR that touches a prompt, a schema or a model
 * pin. Today the corpus is empty, so the gate costs nothing — which is exactly when a ceiling is
 * cheap to add and impossible to remember later. 200 per kind is 600 calls, under a dollar, and far
 * more than a regression needs to show itself; it is applied as PostgREST's own `&limit=`, so the
 * rows never leave the database, and it is printed before the loop so a run always says what it is
 * about to spend.
 */
export const MAX_CASES = 200;

/** The eval is not a user (R-C2-E9 / R-C2-E51 fix 1, finding 3): every stub below ignores this
 * value, and there is no real account behind an automated run, so it is the nil UUID rather than a
 * secret that named an account nothing here actually used. */
const EVAL_ACCOUNT = "00000000-0000-0000-0000-000000000000";

/**
 * Always true, always a no-op — ruling R-C2-E9; see the header. Written as `() => Promise.resolve(…)`
 * rather than `async () => …` only because `deno lint`'s `require-await` rejects an `async`
 * function with no `await` inside it; the contract and the runtime behaviour are identical either
 * way.
 */
const EVAL_CAPS: CapStore = {
  charge: () => Promise.resolve(true),
  withinBudget: () => Promise.resolve(true),
  recordTokens: () => Promise.resolve(),
};

interface Row {
  id: number;
  kind: Kind;
  request: { item: Record<string, unknown>; heuristics_seed: Record<string, unknown> };
  ours: Record<string, unknown> | null;
  theirs: Record<string, unknown>;
}

// `eval_cases`/`eval_runs` are the eval suite's own global tables — the whole shared corpus and
// the whole shared run history, never one account's rows — so neither carries an `account_id` and
// neither is scoped by one here. (`_shared/judge_db_test.ts`'s account-scoping guard would not
// reach this file either way: it scans `_shared/` and each function's own `index.ts`/`handler.ts`,
// and `cloud/eval/run_eval.ts` is neither — but the reason these selects are unscoped is the tables
// themselves, not the guard's blind spot.)
async function load(db: Db, kind: Kind): Promise<Row[]> {
  // The frozen seed, always; plus the last 90 days of corrections that arrived WITH a replayable
  // request (ruling R-C2-4 — that is only ever true under the (c) opt-in, whose UI is C4).
  const since = new Date(Date.now() - 90 * 86_400_000).toISOString();
  return await db.select(
    `eval_cases?kind=eq.${kind}&or=(source.eq.seed,added_at.gte.${since})` +
      `&select=id,kind,request,ours,theirs&order=id&limit=${MAX_CASES}`,
  ) as Row[];
}

/**
 * The `seed_id`s already present among `source = 'seed'` rows, so `--load-seed` never inserts the
 * same seed record twice (R-C2-E51 fix 1, finding 5 — `eval_cases_seed_id`'s partial unique index,
 * `20260911000800_eval_fix.sql`, is the database's own backstop; this is what makes the insert loop
 * skip them instead of relying on that index to reject a duplicate one row at a time).
 */
async function existingSeedIds(db: Db): Promise<Set<string>> {
  const rows = await db.select(`eval_cases?source=eq.seed&select=seed_id`) as Array<{ seed_id: string | null }>;
  const ids = new Set<string>();
  for (const row of rows) {
    if (typeof row.seed_id === "string" && row.seed_id !== "") ids.add(row.seed_id);
  }
  return ids;
}

/**
 * Ruling R-C2-E10: the dry run's scripted answer is a COMPLETE verdict for `kind`, built from
 * `theirs` so `validate()` accepts it and `score()` then compares only the labelled fields — on a
 * non-empty seed every metric reads 1.0 exactly. The literal defaults exist only so a case whose
 * `theirs` does not cover every field `validate()` requires still produces something valid;
 * `theirs` always wins where the two overlap, because it is spread last.
 */
export function dryRunAnswer(kind: Kind, theirs: Record<string, unknown>): Record<string, unknown> {
  if (kind === "task") {
    return {
      confidence: 1,
      importance_reason: "dry run",
      effort_hours: 1,
      importance: 3,
      course: null,
      ...theirs,
    };
  }
  if (kind === "event") {
    return { confidence: 1, why: "dry run", verdict: "drop", ...theirs };
  }
  return { confidence: 1, why: "dry run", tier: "information", ...theirs };
}

type EnvGet = (name: string) => string | undefined;
const realEnvGet: EnvGet = (name) => Deno.env.get(name);

/** Everything `main()` reaches outside its own arguments — factored so `run_eval_test.ts` can
 * drive it with a fake env, a fake `Db`, and a synthetic seed, without a key, a connection, or the
 * real (empty) `cloud/eval/seed/` directory always winning. */
export interface Deps {
  envGet: EnvGet;
  db: () => Db;
  loadSeed: () => Promise<SeedRecord[]>;
}
const REAL_DEPS: Deps = { envGet: realEnvGet, db: serviceDb, loadSeed };

/**
 * The entry point, factored so `run_eval_test.ts` can drive it directly (ruling R-C2-E50 (2)):
 * `envGet` is the only place this program reads a name that could be a secret, so a test can hand
 * it a function that throws and prove the zero-case path never calls it.
 */
export async function main(args: string[], deps: Partial<Deps> = {}): Promise<number> {
  const { envGet, db: dbFactory, loadSeed: loadSeedFn } = { ...REAL_DEPS, ...deps };
  const argSet = new Set(args);

  if (argSet.has("--load-seed")) {
    const db = dbFactory();
    const records = await loadSeedFn();
    const already = await existingSeedIds(db);
    let inserted = 0, present = 0;
    for (const record of records) {
      if (already.has(record.id)) {
        present += 1;
        continue;
      }
      await db.insert("eval_cases", {
        kind: record.kind, request: record.request, ours: null, theirs: record.theirs, source: "seed",
        seed_id: record.id,
      }, false);
      inserted += 1;
    }
    console.log(`${inserted} inserted, ${present} already present`);
    return 0;
  }

  // See the header (ruling R-C2-E50 (2)): this is the WHOLE check, and it runs before anything
  // else — no thresholds file, no env var, no connection.
  const seedRecords = await loadSeedFn();
  if (seedRecords.length === 0) {
    console.log("0 cases — nothing to score");
    return 0;
  }

  const thresholdIndex = args.indexOf("--thresholds");
  if (thresholdIndex < 0) {
    console.error("--thresholds <file> is required");
    return 2;
  }
  const thresholds = JSON.parse(await Deno.readTextFile(args[thresholdIndex + 1])) as
    Record<string, Record<string, number>>;

  const db = dbFactory();
  // Corrections carry no judgment id from the device; fill them before anything reads them.
  await db.rpc("backfill_correction_judgments", {});

  const dry = argSet.has("--dry-run");
  const apiKey = envGet("ANTHROPIC_API_KEY") ?? "";
  if (!dry && apiKey === "") {
    console.error("ANTHROPIC_API_KEY is required unless --dry-run");
    return 2;
  }

  // One id for the whole run (R-C2-E51 fix 1, finding 5): every `eval_runs` row this run writes,
  // across every kind and every metric, carries the same `run_id`, so a later reader can group a
  // run's rows back together without guessing from `ran_at` alone.
  const runId = crypto.randomUUID();

  let worst = 0;
  for (const kind of KINDS) {
    const rows = await load(db, kind);
    if (rows.length === 0) {
      const seeded = seedRecords.some((r) => r.kind === kind);
      if (seeded) {
        // R-C2-E51 fix 1, finding 2: the local seed HAS cases of this kind, but the database
        // corpus does not — the gate must not go green having scored nothing. `--load-seed` is
        // what closes this gap, so the message names it.
        console.error(`${kind}: the seed has cases of this kind but eval_cases holds none — run --load-seed first`);
        worst = Math.max(worst, 2);
      } else {
        console.log(`${kind}: no cases — skipped`);
      }
      continue;
    }
    // S-5: what this run is about to spend, before it spends it. Tier 2 is off for the eval and
    // `--dry-run` reaches no provider at all, so the model-call count is the case count exactly.
    console.log(`${kind}: ${rows.length} cases, <= ${rows.length} model calls (cap ${MAX_CASES})`);
    const row = await modelRow(db, kind);
    const cases: Case[] = rows.map((r) => ({ kind, theirs: r.theirs }));
    const answers: Array<Record<string, unknown> | null> = [];
    let inputTokens = 0, outputTokens = 0;
    for (const r of rows) {
      const raw: JudgeModel = dry
        ? new ScriptedModel([dryRunAnswer(kind, r.theirs)])
        : new AnthropicModel({ apiKey });
      // Wraps whichever model answers, so the run's own token spend is recorded (ruling
      // R-C2-E9) regardless of which branch produced the `ModelReply`.
      const metered: JudgeModel = {
        async complete(req) {
          const reply = await raw.complete(req);
          inputTokens += reply.inputTokens;
          outputTokens += reply.outputTokens;
          return reply;
        },
      };
      const reply = await judge(EVAL_ACCOUNT, { kind, item: r.request.item, heuristics_seed: r.request.heuristics_seed }, {
        row,
        model: metered,
        // Tier 2 is deliberately OFF for the eval: a promoted rule would score the rule, not the
        // model, and the gate exists to decide whether the MODEL still does the job.
        rules: { lookup: () => Promise.resolve(null) },
        caps: EVAL_CAPS,
        // The eval's own judgments are not the product's: they would poison rule promotion and
        // the correction rates with answers nobody ever saw.
        log: { write: () => Promise.resolve(null) },
        origin: "device",
        now: () => Date.now(),
      });
      answers.push(reply.verdict);
    }
    const hash = await promptHash(kind);
    for (const scored of score(kind, cases, answers)) {
      if (scored.value === null) {
        // R-C2-E51 fix 1, finding 1: nothing labelled this metric's field — neither passed nor
        // failed, and no `eval_runs` row (the `real not null` `value` column has nowhere to put a
        // "no data" result).
        console.log(`${kind}.${scored.metric} = n/a (0 of ${rows.length} cases label this field)`);
        continue;
      }
      const threshold = thresholds[kind]?.[`${scored.metric}${scored.higherIsBetter ? "_min" : "_max"}`];
      if (threshold === undefined) {
        console.error(`${kind}.${scored.metric}: no threshold in the file`);
        worst = Math.max(worst, 2);
        continue;
      }
      const bad = failed(scored, threshold);
      await db.insert("eval_runs", {
        model: row.model_id, prompt_version: row.prompt_version, grammar_version: row.grammar_version,
        prompt_hash: hash, kind, metric: scored.metric, value: scored.value,
        threshold, passed: !bad, cases: scored.n, input_tokens: inputTokens, output_tokens: outputTokens,
        run_id: runId, dry_run: dry,
      }, false);
      console.log(
        `${kind}.${scored.metric} = ${scored.value.toFixed(3)} (threshold ${threshold}, n=${scored.n} of ${rows.length} cases) ${
          bad ? "FAIL" : "ok"
        }`,
      );
      if (bad) worst = Math.max(worst, 1);
    }
  }
  return worst;
}

if (import.meta.main) {
  Deno.exit(await main(Deno.args));
}
