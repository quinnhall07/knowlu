// The eval suite (§5.4 measure 2). Loads Task 13's seed, replays each case's `request` through the
// LIVE pipeline with the currently pinned model, scores the replies, writes one `eval_runs` row
// per metric, and exits 1 if any metric is worse than its threshold.
//
// Three switches:
//   --load-seed   insert Task 13's seed (`loadSeed`, `./loader.ts`) into `eval_cases` as
//                 `source = 'seed'` rows, and exit.
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
// `EVAL_ACCOUNT_ID`, before `SUPABASE_URL`/`SUPABASE_SERVICE_ROLE_KEY` (both read inside
// `serviceDb()`), and before any connection opens or any `eval_runs` row is written. That is what
// lets `eval-gate` (hand-off H8, part 2) pass green on a PR that sets none of its three secrets.
//
// Ruling R-C2-E9: the eval suite is not a user, so it never spends an account's own daily cap or
// monthly budget — it passes the UNMETERED `EVAL_CAPS` below and records its OWN spend, summed
// from every `ModelReply`, as `input_tokens`/`output_tokens` on each `eval_runs` row instead.
import type { CapStore } from "../supabase/functions/_shared/judge_caps.ts";
import type { Db } from "../supabase/functions/_shared/judge_db.ts";
import { serviceDb } from "../supabase/functions/_shared/judge_db.ts";
import { AnthropicModel, type JudgeModel, ScriptedModel } from "../supabase/functions/_shared/judge_anthropic.ts";
import { modelRow } from "../supabase/functions/_shared/judge_models.ts";
import { judge, type Kind } from "../supabase/functions/_shared/judge_pipeline.ts";
import { promptHash } from "../supabase/functions/_shared/judge_prompts.ts";
import { loadSeed } from "./loader.ts";
import { type Case, failed, score } from "./score.ts";

const KINDS: Kind[] = ["task", "event", "email"];

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
    `eval_cases?kind=eq.${kind}&or=(source.eq.seed,added_at.gte.${since})&select=id,kind,request,ours,theirs`,
  ) as Row[];
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

/**
 * The entry point, factored so `run_eval_test.ts` can drive it directly (ruling R-C2-E50 (2)):
 * `envGet` is the only place this program reads a name that could be a secret, so a test can hand
 * it a function that throws and prove the zero-case path never calls it.
 */
export async function main(args: string[], envGet: EnvGet = realEnvGet): Promise<number> {
  const argSet = new Set(args);

  if (argSet.has("--load-seed")) {
    const db = serviceDb();
    const records = await loadSeed();
    let loaded = 0;
    for (const record of records) {
      await db.insert("eval_cases", {
        kind: record.kind, request: record.request, ours: null, theirs: record.theirs, source: "seed",
      }, false);
      loaded += 1;
    }
    console.log(`loaded ${loaded} seed cases`);
    return 0;
  }

  // See the header (ruling R-C2-E50 (2)): this is the WHOLE check, and it runs before anything
  // else — no thresholds file, no env var, no connection.
  const seedCount = (await loadSeed()).length;
  if (seedCount === 0) {
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

  const db = serviceDb();
  // Corrections carry no judgment id from the device; fill them before anything reads them.
  await db.rpc("backfill_correction_judgments", {});

  const dry = argSet.has("--dry-run");
  const apiKey = envGet("ANTHROPIC_API_KEY") ?? "";
  if (!dry && apiKey === "") {
    console.error("ANTHROPIC_API_KEY is required unless --dry-run");
    return 2;
  }
  const evalAccount = envGet("EVAL_ACCOUNT_ID") ?? "";

  let worst = 0;
  for (const kind of KINDS) {
    const rows = await load(db, kind);
    if (rows.length === 0) {
      console.log(`${kind}: no cases — skipped`);
      continue;
    }
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
      const reply = await judge(evalAccount, { kind, item: r.request.item, heuristics_seed: r.request.heuristics_seed }, {
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
      const threshold = thresholds[kind]?.[`${scored.metric}${scored.higherIsBetter ? "_min" : "_max"}`];
      if (threshold === undefined) {
        console.error(`${kind}.${scored.metric}: no threshold in the file`);
        worst = 2;
        continue;
      }
      const bad = failed(scored, threshold);
      await db.insert("eval_runs", {
        model: row.model_id, prompt_version: row.prompt_version, grammar_version: row.grammar_version,
        prompt_hash: hash, kind, metric: scored.metric, value: scored.value,
        threshold, passed: !bad, cases: rows.length, input_tokens: inputTokens, output_tokens: outputTokens,
      }, false);
      console.log(
        `${kind}.${scored.metric} = ${scored.value.toFixed(3)} (threshold ${threshold}, ${rows.length} cases) ${bad ? "FAIL" : "ok"}`,
      );
      if (bad) worst = 1;
    }
  }
  return worst;
}

if (import.meta.main) {
  Deno.exit(await main(Deno.args));
}
