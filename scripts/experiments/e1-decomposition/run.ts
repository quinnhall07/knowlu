// Stream J experiment E1 — the B1 gate for T3's decomposition. Scores the event-3 prompt (frozen in
// `event3_frozen.ts`, as committed at 7c127e2) against the live decomposed event-4 prompt over T2's
// 26 seed cases (`cloud/eval/seed/events.jsonl`), with the pinned event model called through
// OpenRouter directly, one sequential request per case per arm (52 in all), and reports per-arm
// weighted_exact with a 95% interval, the paired difference, and the B1 verdict (`stats.ts`).
//
// Dry run (sends nothing, resolves no key, needs only --allow-read), from the worktree root:
//   deno run --allow-read --config scripts/experiments/e1-decomposition/deno.json \
//     scripts/experiments/e1-decomposition/run.ts --dry-run
//
// The paid run (the controller runs it; the key comes from the first of: process env
// OPENROUTER_API_KEY, the Windows USER-scope variable of that name, Credential Manager target
// knowlu/dev/openrouter — it is never printed, logged or written):
//   deno run --allow-read --allow-env=OPENROUTER_API_KEY --allow-run=powershell.exe --allow-net=openrouter.ai \
//     --config scripts/experiments/e1-decomposition/deno.json scripts/experiments/e1-decomposition/run.ts
import { loadSeed } from "../../../cloud/eval/loader.ts";
import type { Case } from "../../../cloud/eval/score.ts";
import { score } from "../../../cloud/eval/score.ts";
import type { ModelRow } from "../../../cloud/supabase/functions/_shared/judge_models.ts";
import { OpenRouterModel } from "../../../cloud/supabase/functions/_shared/judge_openrouter.ts";
import {
  type Arm,
  ARMS,
  type ArmName,
  deviceRecorded,
  estimateCost,
  type Interpreted,
  modelRequest,
  requestBody,
} from "./arms.ts";
import { noKeyMessage, resolveOpenRouterKey } from "./credentials.ts";
import { eventRowFromMigrations } from "./model_row.ts";
import { b1Verdict, bootstrapMeanCI, minimumShippableWins, perCaseCredit } from "./stats.ts";

export interface EventCase {
  id: string;
  item: Record<string, unknown>;
  seed: Record<string, unknown>;
  label: string;
}

export const COST_NOTE = "weighted_exact is cloud/eval/score.ts's, unmodified. Its COST map is UNRULED " +
  "(docs/notes/2026-09-22-cost-matrices.md is a proposal): only obligation->drop (3), obligation->opportunity, " +
  "opportunity->drop and drop->opportunity (1 each) are named for events; every other mismatch — every cell " +
  "involving unsure among them — falls through to the default cost 1, and a missing or refused answer costs 3.";

export function b1RuleLines(n: number): string[] {
  const table = [0, 1, 2, 3].map((l) => `${minimumShippableWins(l)} wins with ${l} losses`).join("; ");
  return [
    "B1 verdict rule: ship event-4 only if BOTH (1) the paired bootstrap 95% CI on (event-4 - event-3) " +
    "weighted_exact lies wholly above 0 AND (2) the exact two-sided sign test over the cases where the arms " +
    "disagree gives p < 0.05 in event-4's favour. A tie or a loss closes the idea: record the number.",
    `How large a gap that takes with ${n} cases: the sign test binds — event-4 must score higher on at least ` +
    `${table}. Six clean wins out of ${n} is a weighted_exact gap of ${(6 / n / 3).toFixed(3)} if every win is ` +
    `the smallest one-cost step and ${(6 / n).toFixed(3)} if every win is a full miss-to-hit. Fewer net ` +
    "wins than that is inside what this seed can explain by noise (its README: roughly twenty points of " +
    "error at 95%). This seed can rule the decomposition out; it cannot rule it in for real events.",
  ];
}

export function dryRunLines(cases: EventCase[], row: ModelRow): string[] {
  const est = estimateCost(cases, row);
  const lines = [
    `cost estimate: ${est.calls} calls, ~${est.inputTokens} input tokens (over-estimated: the schema and ` +
    `route count as input), ~${est.outputTokens} output tokens, at $${row.usd_per_m_in}/M in and ` +
    `$${row.usd_per_m_out}/M out: ~$${est.usd.toFixed(4)} total.`,
  ];
  for (const arm of Object.values(ARMS)) {
    lines.push(`--- request body, arm ${arm.name}, case ${cases[0].id} (dry run — nothing sent) ---`);
    lines.push(JSON.stringify(requestBody(arm, cases[0].item, cases[0].seed, row), null, 2));
  }
  return lines;
}

async function loadCases(): Promise<EventCase[]> {
  const records = await loadSeed();
  return records.filter((r) => r.kind === "event").map((r) => ({
    id: r.id,
    item: r.request.item as Record<string, unknown>,
    seed: (r.request.heuristics_seed ?? {}) as Record<string, unknown>,
    label: String(r.theirs.verdict),
  }));
}

async function runArm(
  arm: Arm,
  cases: EventCase[],
  row: ModelRow,
  model: OpenRouterModel,
): Promise<{ results: Interpreted[]; inTok: number; outTok: number; failed: number }> {
  const results: Interpreted[] = [];
  let inTok = 0, outTok = 0, failed = 0;
  for (const c of cases) {
    try {
      const reply = await model.complete(modelRequest(arm, c.item, c.seed, row));
      inTok += reply.inputTokens;
      outTok += reply.outputTokens;
      results.push(arm.interpret(reply.json));
    } catch (e) {
      // Only the error's class reaches the console: a provider string can carry the prompt back
      // out, and the key must never appear anywhere.
      failed++;
      results.push({ verdict: null, cause: "model failed" });
      console.log(`  ${c.id} [${arm.name}]: call failed (${e instanceof Error ? e.constructor.name : "unknown"})`);
    }
  }
  return { results, inTok, outTok, failed };
}

async function main() {
  const dryRun = Deno.args.includes("--dry-run");
  const cases = await loadCases();
  const row = await eventRowFromMigrations();
  const labels = cases.reduce<Record<string, number>>((m, c) => ({ ...m, [c.label]: (m[c.label] ?? 0) + 1 }), {});

  console.log("== E1: event-3 (frozen at 7c127e2) vs event-4 (decomposed drop rules) ==");
  console.log(`seed: ${cases.length} event cases — ${JSON.stringify(labels)}`);
  console.log(
    `pinned event row (from the migrations): ${row.model_id} via ${row.provider}, route ${JSON.stringify(row.route)}, ` +
      `precision ${row.precision}, max_tokens ${row.max_tokens}, sampling ${JSON.stringify(row.sampling)}`,
  );
  for (const arm of Object.values(ARMS)) console.log(`arm ${arm.name}: prompt hash ${await arm.promptHash()}`);
  console.log(COST_NOTE);
  for (const line of b1RuleLines(cases.length)) console.log(line);

  if (dryRun) {
    console.log("--dry-run: no key is resolved and nothing is sent.");
    for (const line of dryRunLines(cases, row)) console.log(line);
    return;
  }
  const { key, source } = await resolveOpenRouterKey();
  if (!key) {
    console.log(noKeyMessage());
    for (const line of dryRunLines(cases, row)) console.log(line);
    return;
  }
  console.log(`OPENROUTER_API_KEY resolved from: ${source}. Sending ${cases.length * 2} sequential requests.`);
  const model = new OpenRouterModel({ apiKey: key });

  const scoreCases: Case[] = cases.map((c) => ({ kind: "event", theirs: { verdict: c.label } }));
  const out: Record<
    string,
    { answers: Array<Record<string, unknown> | null>; credit: number[]; recordedCredit: number[] }
  > = {};
  let usd = 0;
  for (const name of ["event-3", "event-4"] as ArmName[]) {
    const arm = ARMS[name];
    const run = await runArm(arm, cases, row, model);
    usd += (run.inTok * row.usd_per_m_in + run.outTok * row.usd_per_m_out) / 1e6;
    const answers = run.results.map((r) => r.verdict);
    const credit = perCaseCredit(scoreCases, answers);
    // The device's view: a refused answer is recorded as `unsure` (cloudmodel.rs judge_event).
    const recorded = run.results.map(deviceRecorded);
    const recordedCredit = perCaseCredit(scoreCases, recorded);
    out[name] = { answers, credit, recordedCredit };
    const we = score("event", scoreCases, answers)[0].value ?? 0;
    const ci = bootstrapMeanCI(credit);
    const weRec = score("event", scoreCases, recorded)[0].value ?? 0;
    const ciRec = bootstrapMeanCI(recordedCredit);
    const causes = run.results.reduce<Record<string, number>>(
      (m, r) => (r.cause ? { ...m, [r.cause]: (m[r.cause] ?? 0) + 1 } : m),
      {},
    );
    const rules = run.results.reduce<Record<string, number>>(
      (m, r) => (r.rule ? { ...m, [r.rule]: (m[r.rule] ?? 0) + 1 } : m),
      {},
    );
    console.log(
      `${name}: weighted_exact ${we.toFixed(3)}, 95% CI [${ci.lower.toFixed(3)}, ${ci.upper.toFixed(3)}] ` +
        `(score.ts: a refused answer costs 3); as the device records it (refused -> unsure): ` +
        `${weRec.toFixed(3)}, 95% CI [${ciRec.lower.toFixed(3)}, ${ciRec.upper.toFixed(3)}]; ` +
        `refused ${JSON.stringify(causes)}; ${name === "event-4" ? `rules fired ${JSON.stringify(rules)}; ` : ""}` +
        `tokens ${run.inTok} in / ${run.outTok} out`,
    );
  }

  console.log("--- per case: id | label | event-3 | event-4 ---");
  cases.forEach((c, i) => {
    const v = (name: ArmName) => String(out[name].answers[i]?.verdict ?? "(none)");
    console.log(`${c.id} | ${c.label} | ${v("event-3")} | ${v("event-4")}`);
  });

  const b1 = b1Verdict(out["event-3"].credit, out["event-4"].credit);
  console.log(
    `paired (event-4 - event-3): ${b1.paired.meanDiff.toFixed(3)}, 95% CI [${b1.paired.ci.lower.toFixed(3)}, ` +
      `${b1.paired.ci.upper.toFixed(3)}]; wins ${b1.paired.wins}, losses ${b1.paired.losses}, ties ${b1.paired.ties}; ` +
      `sign test p=${b1.paired.signP.toFixed(4)}`,
  );
  console.log(`B1 (the gate, on score.ts's figure): ${b1.reason}`);
  const b1Rec = b1Verdict(out["event-3"].recordedCredit, out["event-4"].recordedCredit);
  console.log(`beside it, as the device records (refused -> unsure; not the gate): ${b1Rec.reason}`);
  console.log(`actual spend at the row's rates: ~$${usd.toFixed(4)}`);
}

if (import.meta.main) await main();
