#!/usr/bin/env -S deno run --allow-read --allow-write --allow-run=powershell.exe --allow-env=OPENROUTER_API_KEY,E2_CORPUS_ROOT,E2_WORKSPACE --allow-net=openrouter.ai
// Stream J experiment E2 — the offline review-triage replay. Orchestrates the whole procedure:
// extract, leak-check both arms, both baselines, the cost estimate, then either the paid run (one
// sequential pass over OpenRouter's decisions endpoint, only if a key resolves) or --dry-run (prints
// the exact request bodies and the cost estimate, sends nothing).
//
// The paid run writes two files to the workspace: `paid-run-results.jsonl` (one line per item: the
// request, the full answer with its distribution, latency, usage, or the error) and
// `paid-run-summary.json` (score.ts's scored summary beside the baselines). An HTTP or parse failure
// is recorded against its item and the pass goes on; five in a row stop it (replay.ts).
//
// Every run needs the corpus folder and a workspace to write into, as `--corpus-root <dir>
// --workspace <dir>` or the environment variables `E2_CORPUS_ROOT` / `E2_WORKSPACE` (see
// `extract.ts`'s `resolvePaths`; there is no default, and no machine path is written here).
//
// The one command for the paid run, once OPENROUTER_API_KEY is available through any of the three
// tiers credentials.ts tries:
//   deno run --allow-read --allow-write --allow-run=powershell.exe \
//     --allow-env=OPENROUTER_API_KEY,E2_CORPUS_ROOT,E2_WORKSPACE --allow-net=openrouter.ai run.ts \
//     --corpus-root <dir> --workspace <dir>
//
// Dry run (no network permission needed, nothing sent):
//   deno run --allow-read --allow-write --allow-run=powershell.exe \
//     --allow-env=OPENROUTER_API_KEY,E2_CORPUS_ROOT,E2_WORKSPACE run.ts --dry-run \
//     --corpus-root <dir> --workspace <dir>

import { extractCorpus, forDisk, resolvePaths } from "./extract.ts";
import { findLeaks } from "./severity.ts";
import { type LabeledItem, leaveOneOutLexicalKNN, leaveOneOutMajorityClass } from "./baselines.ts";
import { buildDecisionsBody, estimateCorpusCost, OPENROUTER_DECISIONS_URL } from "./jev_request.ts";
import { runPass } from "./replay.ts";
import { scoreResults } from "./score.ts";
import { noKeyMessage, resolveOpenRouterKey } from "./credentials.ts";
import type { CorpusFinding } from "./types.ts";

function toLabeledItems(findings: CorpusFinding[], label: (f: CorpusFinding) => string): LabeledItem[] {
  return findings.map((f) => ({ id: f.id, text: f.text, label: label(f) }));
}

async function main() {
  const args = new Set(Deno.args);
  const forceDryRun = args.has("--dry-run");
  const paths = resolvePaths(Deno.args);
  const workspace = paths.workspace;

  console.log("== Step 1: extract ==");
  const { kept, dropped, summary } = await extractCorpus(paths);
  console.log(`raw findings: ${summary.rawFound} (expected ${summary.expected})`);
  console.log(`dropped (secret/token/session/credential): ${dropped.length}`);
  console.log(`corpus size: ${kept.length}`);

  await Deno.mkdir(workspace, { recursive: true });
  await Deno.writeTextFile(
    `${workspace}/corpus.jsonl`,
    kept.map((f) => JSON.stringify(forDisk(f))).join("\n") + "\n",
  );
  await Deno.writeTextFile(`${workspace}/summary.json`, JSON.stringify(summary, null, 2));
  await Deno.writeTextFile(`${workspace}/dropped.json`, JSON.stringify(dropped, null, 2));

  console.log("\n== Step 2: leak check, both arms (procedure §Step 3) ==");
  for (const f of kept) {
    const leaks = findLeaks(f.text);
    if (leaks.length > 0) {
      console.error(`LEAK CHECK FAILED for ${f.id}: ${JSON.stringify(leaks)} — stopping.`);
      Deno.exit(1);
    }
  }
  console.log("stripped arm: 0 leaks over the whole corpus (findLeaks empty on every kept item).");
  const strippedArm = leaveOneOutLexicalKNN(toLabeledItems(kept, (f) => f.severity), 3);
  const rawArm = leaveOneOutLexicalKNN(
    kept.map((f) => ({ id: f.id, text: f.rawText, label: f.severity })),
    3,
  );
  console.log(
    `lexical baseline on severity, whole corpus — stripped: ${(strippedArm.accuracy * 100).toFixed(1)}% ` +
      `(${strippedArm.correct}/${strippedArm.n}); raw (labels left in): ` +
      `${(rawArm.accuracy * 100).toFixed(1)}% (${rawArm.correct}/${rawArm.n}).`,
  );

  // Set A's severity is positional (a "### Critical" heading above the item, never a word inside
  // it), so its rawText carries no extra signal at all — only Set B's rawText literally contains
  // "should-fix"/"nit"/"blocking". Diluting the raw arm with Set A's 19 items understates the leak
  // a naive extraction would cause; the Set-B-only comparison is the one that actually tests it.
  const keptB = kept.filter((f) => f.set === "B");
  const strippedArmB = leaveOneOutLexicalKNN(toLabeledItems(keptB, (f) => f.severity), 3);
  const rawArmB = leaveOneOutLexicalKNN(
    keptB.map((f) => ({ id: f.id, text: f.rawText, label: f.severity })),
    3,
  );
  console.log(
    `lexical baseline on severity, Set B only (${keptB.length} items, where the raw span literally ` +
      `contains the label word) — stripped: ${(strippedArmB.accuracy * 100).toFixed(1)}% ` +
      `(${strippedArmB.correct}/${strippedArmB.n}); raw: ${(rawArmB.accuracy * 100).toFixed(1)}% ` +
      `(${rawArmB.correct}/${rawArmB.n}).`,
  );
  if (rawArmB.accuracy <= strippedArmB.accuracy) {
    console.warn(
      "WARNING: on Set B, where a leak would actually show up as a token, the raw arm did not " +
        "score higher than the stripped arm. See the report before trusting any other number here.",
    );
  }

  console.log("\n== Step 3: baselines (procedure §Step 3) ==");
  const severityItems = toLabeledItems(kept, (f) => f.severity);
  const dispositionItems = toLabeledItems(kept, (f) => f.disposition);

  const majoritySeverity = leaveOneOutMajorityClass(severityItems);
  const majorityDisposition = leaveOneOutMajorityClass(dispositionItems);
  const lexicalSeverity = leaveOneOutLexicalKNN(severityItems, 3);
  const lexicalDisposition = leaveOneOutLexicalKNN(dispositionItems, 3);

  const fmt = (r: { accuracy: number; correct: number; n: number; ci95: { lower: number; upper: number } }) =>
    `${(r.accuracy * 100).toFixed(1)}% (${r.correct}/${r.n}), 95% CI [${(r.ci95.lower * 100).toFixed(1)}%, ` +
    `${(r.ci95.upper * 100).toFixed(1)}%]`;

  console.log(`severity — majority class (leave-one-out): ${fmt(majoritySeverity)}`);
  console.log(`severity — lexical TF-IDF 3-NN (leave-one-out): ${fmt(lexicalSeverity)}`);
  console.log(`disposition — majority class (leave-one-out): ${fmt(majorityDisposition)}`);
  console.log(`disposition — lexical TF-IDF 3-NN (leave-one-out): ${fmt(lexicalDisposition)}`);

  const baselineResults = {
    severity: { majorityClass: majoritySeverity, lexicalKnn: lexicalSeverity },
    disposition: { majorityClass: majorityDisposition, lexicalKnn: lexicalDisposition },
    leakCheck: { strippedArm, rawArm, strippedArmSetBOnly: strippedArmB, rawArmSetBOnly: rawArmB },
  };
  await Deno.writeTextFile(`${workspace}/baselines.json`, JSON.stringify(baselineResults, null, 2));

  console.log("\n== Step 4: cost estimate ==");
  const cost = estimateCorpusCost(kept);
  console.log(
    `one pass over ${cost.items} items: ~${cost.estimatedInputTokens} input tokens, ` +
      `~$${cost.estimatedCostUsd.toFixed(4)} (output free).`,
  );

  console.log("\n== Step 5: the paid run ==");
  const { key, source, diagnostics } = await resolveOpenRouterKey();
  if (!key || forceDryRun) {
    if (!key) console.log(noKeyMessage(diagnostics));
    else console.log("--dry-run passed: not sending, even though a key resolved.");
    console.log(
      `transport: POST ${OPENROUTER_DECISIONS_URL}, model typesafe/jev-1.13, pinned zero-retention.`,
    );
    console.log("first three request bodies (dry run — nothing sent):");
    for (const f of kept.slice(0, 3)) {
      console.log(JSON.stringify(buildDecisionsBody(f), null, 2));
    }
    console.log(
      `\n(dry run) would send ${kept.length} sequential requests, estimated ` +
        `~$${cost.estimatedCostUsd.toFixed(4)} total.`,
    );
    return;
  }

  console.log(`OPENROUTER_API_KEY resolved from: ${source}. Sending ${kept.length} sequential requests.`);
  const pass = await runPass(kept, async (body) => {
    const resp = await fetch(OPENROUTER_DECISIONS_URL, {
      method: "POST",
      headers: {
        "Authorization": `Bearer ${key}`,
        "Content-Type": "application/json",
        "HTTP-Referer": "https://knowlu.com",
        "X-Title": "Knowlu E2 review-triage replay",
      },
      body: JSON.stringify(body),
    });
    return { status: resp.status, text: await resp.text() };
  });
  await Deno.writeTextFile(
    `${workspace}/paid-run-results.jsonl`,
    pass.results.map((r) => JSON.stringify(r)).join("\n") + "\n",
  );
  const scored = scoreResults(pass.results);
  const paidSummary = {
    ranAt: new Date().toISOString(),
    stoppedEarly: pass.stoppedEarly,
    jev: scored,
    baselines: {
      severity: { majorityClass: majoritySeverity, lexicalKnn: lexicalSeverity },
      disposition: { majorityClass: majorityDisposition, lexicalKnn: lexicalDisposition },
    },
  };
  await Deno.writeTextFile(`${workspace}/paid-run-summary.json`, JSON.stringify(paidSummary, null, 2));

  console.log("\n== Step 6: scored ==");
  console.log(
    `answered ${scored.answered}/${scored.items}; failed: ${JSON.stringify(scored.failed)}` +
      (pass.stoppedEarly ? " — STOPPED EARLY after consecutive failures" : ""),
  );
  console.log(
    `severity — Jev: ${fmt(scored.severity.accuracy)}; demotions (critical graded minor): ` +
      JSON.stringify(scored.severity.demotions),
  );
  console.log(`severity confusion (truth → predicted): ${JSON.stringify(scored.severity.confusion)}`);
  console.log(`disposition — Jev: ${fmt(scored.disposition.accuracy)}`);
  console.log(`disposition confusion (truth → predicted): ${JSON.stringify(scored.disposition.confusion)}`);
  console.log(
    `blocks vs critical|important: AUROC ${scored.blocks.auroc}, Brier ${scored.blocks.brier} ` +
      `(${scored.blocks.positives}/${scored.blocks.n} positive)`,
  );
  console.log(
    `confidence separation — grade: ${JSON.stringify(scored.severity.confidence)}; ` +
      `disposition: ${JSON.stringify(scored.disposition.confidence)}`,
  );
  console.log(
    `spend $${scored.costUsd.toFixed(6)} (${scored.inputTokens} in / ${scored.outputTokens} out); ` +
      `latency p50 ${scored.latencyMs.p50?.toFixed(0)}ms p99 ${scored.latencyMs.p99?.toFixed(0)}ms; ` +
      `models ${JSON.stringify(scored.models)}`,
  );
  console.log(`wrote ${workspace}/paid-run-results.jsonl and paid-run-summary.json`);
}

if (import.meta.main) {
  await main();
}
