#!/usr/bin/env -S deno run --allow-read --allow-write
// Stream J experiment E2 — corpus extraction (procedure §Step 1).
//
// Reads the nine C1b review documents, parses their findings, normalises severity to one ordinal
// scale, attaches the curated disposition, drops anything naming a secret/token/session/credential,
// and writes the surviving corpus to the git-ignored e2/ workspace directory named in the brief —
// never into the repository, because the corpus contains our own review prose.
//
// Run from this directory:
//   deno run --allow-read --allow-write extract.ts
//
// Corpus paths are absolute and point at a specific worktree (c1b-sign-in) that must exist on this
// machine — this is a one-time replay over a fixed, already-written corpus, not a general tool.

import { parseFinalReviewFindings, parsePlanReviewFindings, parseTaskReviewFindings } from "./parsers.ts";
import { findLeaks, normalizeSeverity } from "./severity.ts";
import { mentionsSecret, secretMentionTerms } from "./secretfilter.ts";
import { dispositionFor } from "./dispositions.ts";
import type { CorpusFinding, DroppedFinding, ExtractionSummary, RawFinding } from "./types.ts";

const C1B_WORKTREE_ROOT =
  "C:\\Users\\danie\\GitHub\\knowlu\\.claude\\worktrees\\c1b-sign-in\\.superpowers\\sdd\\2026-09-17-c1b-sign-in-plan";
const PLAN_REVIEW_PATH =
  "C:\\Users\\danie\\GitHub\\knowlu\\.claude\\worktrees\\j-e2\\docs\\reports\\2026-09-17-c1b-sign-in-plan-review.md";
export const WORKSPACE_DIR =
  "C:\\Users\\danie\\GitHub\\knowlu\\.superpowers\\sdd\\2026-09-22-judgment-quality-plan\\e2";

const TASK_FILES = [1, 2, 3, 4, 5, 6, 7].map((n) => `task-${n}-review.md`);

interface ExtractionOutcome {
  kept: CorpusFinding[];
  dropped: DroppedFinding[];
  summary: ExtractionSummary;
}

export async function extractCorpus(): Promise<ExtractionOutcome> {
  const raw: RawFinding[] = [];
  const perSourceCounts: Record<string, number> = {};

  for (const file of TASK_FILES) {
    const taskNum = file.match(/^task-(\d+)-review\.md$/)![1];
    const text = await Deno.readTextFile(`${C1B_WORKTREE_ROOT}\\${file}`);
    const items = parseTaskReviewFindings(text);
    perSourceCounts[file] = items.length;
    for (const item of items) {
      const id = `B-task${taskNum}-${item.tag}`;
      const { disposition, citedFrom } = dispositionFor(id);
      raw.push({
        id,
        set: "B",
        source: file,
        severityRaw: item.severityRaw,
        severity: normalizeSeverity(item.severityRaw),
        disposition,
        dispositionSource: citedFrom,
        text: item.text,
        rawText: item.rawText,
      });
    }
  }

  {
    const file = "final-review.md";
    const text = await Deno.readTextFile(`${C1B_WORKTREE_ROOT}\\${file}`);
    const items = parseFinalReviewFindings(text);
    perSourceCounts[file] = items.length;
    for (const item of items) {
      const id = `B-final-${item.tag}`;
      const { disposition, citedFrom } = dispositionFor(id);
      raw.push({
        id,
        set: "B",
        source: file,
        severityRaw: item.severityRaw,
        severity: normalizeSeverity(item.severityRaw),
        disposition,
        dispositionSource: citedFrom,
        text: item.text,
        rawText: item.rawText,
      });
    }
  }

  {
    const file = "2026-09-17-c1b-sign-in-plan-review.md";
    const text = await Deno.readTextFile(PLAN_REVIEW_PATH);
    const items = parsePlanReviewFindings(text);
    perSourceCounts[file] = items.length;
    for (const item of items) {
      const id = `A-${item.tag}`;
      const { disposition, citedFrom } = dispositionFor(id);
      raw.push({
        id,
        set: "A",
        source: file,
        severityRaw: item.severityRaw,
        severity: normalizeSeverity(item.severityRaw),
        disposition,
        dispositionSource: citedFrom,
        text: item.text,
        rawText: item.rawText,
      });
    }
  }

  // Leak check first (procedure §4 stop rule 1): nothing proceeds past this if a severity marker
  // survived extraction, in any of the four placements.
  for (const f of raw) {
    const leaks = findLeaks(f.text);
    if (leaks.length > 0) {
      throw new Error(`LEAK CHECK FAILED for ${f.id}: found ${JSON.stringify(leaks)} still in the text`);
    }
  }

  // The secret/token/session/credential drop filter (decision context item 3): mechanical, over
  // every kept finding's text, never a per-item judgment call.
  const kept: CorpusFinding[] = [];
  const dropped: DroppedFinding[] = [];
  for (const f of raw) {
    if (mentionsSecret(f.text)) {
      dropped.push({
        id: f.id,
        set: f.set,
        source: f.source,
        matchedTerms: secretMentionTerms(f.text),
      });
    } else {
      kept.push({ ...f, kept: true });
    }
  }

  const reconciliation = [
    "Set A (plan review): 24 findings — 6 Critical, 8 Important, 10 Minor. Matches the note's own " +
    "table (procedure §1) exactly.",
    "Set B task reviews + final review: 36 findings, not the note's table total of 35. task-3-review.md " +
    "has 6 findings (2 should-fix + 4 nit: items 1,2 should-fix; items 3,4,5,6 nit), not 5 " +
    "(0 blocking + 2 should-fix + 3 nit) as the note's table states — item 4 " +
    "(`app/src/account.rs:395-398`, a `(nit)`) is present in the source document and is not an " +
    "extraction artefact; re-derived directly from task-3-review.md on disk, 2026-09-22.",
    "Total raw corpus: 60 findings (24 + 36), against the brief's expected 59.",
    "Secret/token/session/credential drop filter: 11 of the 60 raw findings, not the 3-8 a first " +
    "manual read suggested — see docs/reports/2026-09-22-e2-review-replay-prep.md for the full " +
    "audit. Two read as false positives at first glance and are not: A-C6 matches on Stripe's own " +
    'proper noun "Checkout Session" (still, literally, a mention of a session), and A-M4 matches ' +
    'on the literal env-var name KNOWLU_ANON_KEY. Fix round 1 added B-final-F2 ("the port and the ' +
    'tokens", plural) once SECRET_TERMS matched plural/inflected forms too. Kept: 49.',
  ];

  const summary: ExtractionSummary = {
    extractedAt: new Date().toISOString(),
    expected: 59,
    rawFound: raw.length,
    perSourceCounts,
    reconciliation,
    droppedForSecrets: dropped,
    keptCount: kept.length,
  };

  return { kept, dropped, summary };
}

/** Never persists `rawText` — the corpus.jsonl on disk is leak-free, full stop. */
export function forDisk(f: CorpusFinding): Omit<CorpusFinding, "rawText"> {
  const { rawText: _rawText, ...rest } = f;
  return rest;
}

function stableStringify(value: unknown): string {
  return JSON.stringify(value);
}

async function main() {
  const { kept, dropped, summary } = await extractCorpus();

  await Deno.mkdir(WORKSPACE_DIR, { recursive: true });

  const corpusLines = kept.map((f) => stableStringify(forDisk(f))).join("\n") + "\n";
  await Deno.writeTextFile(`${WORKSPACE_DIR}\\corpus.jsonl`, corpusLines);
  await Deno.writeTextFile(`${WORKSPACE_DIR}\\summary.json`, JSON.stringify(summary, null, 2));
  await Deno.writeTextFile(`${WORKSPACE_DIR}\\dropped.json`, JSON.stringify(dropped, null, 2));

  console.log(`raw findings parsed: ${summary.rawFound} (expected ${summary.expected})`);
  console.log(`dropped for naming a secret/token/session/credential: ${dropped.length}`);
  console.log(`kept in the corpus: ${summary.keptCount}`);
  console.log(`wrote ${WORKSPACE_DIR}\\corpus.jsonl, summary.json, dropped.json`);
}

if (import.meta.main) {
  await main();
}
