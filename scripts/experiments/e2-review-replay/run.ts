#!/usr/bin/env -S deno run --allow-read --allow-write --allow-run=powershell.exe --allow-env=OPENROUTER_API_KEY,E2_CORPUS_ROOT,E2_WORKSPACE --allow-net=openrouter.ai
// Stream J experiment E2 — the offline review-triage replay. Orchestrates the whole procedure:
// extract, leak-check both arms, both baselines, the cost estimate, then either the paid run (one
// sequential pass over OpenRouter, only if a key resolves) or --dry-run (always the default; prints
// the exact request bodies and the cost estimate, sends nothing).
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
import { buildOpenRouterChatBody, estimateCorpusCost, OPENROUTER_CHAT_URL } from "./jev_request.ts";
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
  const { key, source } = await resolveOpenRouterKey();
  if (!key || forceDryRun) {
    if (!key) console.log(noKeyMessage());
    else console.log("--dry-run passed: not sending, even though a key resolved.");
    console.log(`transport: POST ${OPENROUTER_CHAT_URL}, model typesafe/jev-1.13, pinned zero-retention.`);
    console.log("first three request bodies (dry run — nothing sent):");
    for (const f of kept.slice(0, 3)) {
      console.log(JSON.stringify(buildOpenRouterChatBody(f), null, 2));
    }
    console.log(
      `\n(dry run) would send ${kept.length} sequential requests, estimated ` +
        `~$${cost.estimatedCostUsd.toFixed(4)} total.`,
    );
    return;
  }

  console.log(`OPENROUTER_API_KEY resolved from: ${source}. Sending ${kept.length} sequential requests.`);
  const results: unknown[] = [];
  for (const f of kept) {
    const body = buildOpenRouterChatBody(f);
    const started = performance.now();
    const resp = await fetch(OPENROUTER_CHAT_URL, {
      method: "POST",
      headers: {
        "Authorization": `Bearer ${key}`,
        "Content-Type": "application/json",
        "HTTP-Referer": "https://knowlu.com",
        "X-Title": "Knowlu E2 review-triage replay",
      },
      body: JSON.stringify(body),
    });
    const elapsedMs = performance.now() - started;
    const text = await resp.text();
    results.push({ id: f.id, status: resp.status, elapsedMs, body: text });
    console.log(`${f.id}: HTTP ${resp.status} in ${elapsedMs.toFixed(0)}ms`);
  }
  await Deno.writeTextFile(`${workspace}/paid-run-results.json`, JSON.stringify(results, null, 2));
  console.log(`wrote ${workspace}/paid-run-results.json`);
}

if (import.meta.main) {
  await main();
}
