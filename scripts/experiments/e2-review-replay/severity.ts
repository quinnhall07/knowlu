// The label-leak hazard (e2-brief.md, procedure §1): the severity word sits in four
// placements — the first token of a task-review finding, a trailing "**Severity: X.**" in the
// final review, a "### Critical"-style heading positioned above a finding in the plan review, and
// (checked empirically against the actual corpus, 2026-09-22) nowhere else as an incidental English
// word — "critical", "important", "minor", "blocking", "nit" and "should-fix" do not otherwise occur
// in this corpus's finding bodies. `stripLeakTerms` is still a global scrub, not just a positional
// one, so it is not relying on that being true forever.

import type { Severity } from "./types.ts";

/** Every raw spelling the corpus uses, mapped to the one ordinal scale (documented in the report). */
const SEVERITY_WORD_MAP: Record<string, Severity> = {
  critical: "critical",
  blocking: "critical",
  "[blocking]": "critical",
  important: "important",
  "should-fix": "important",
  "should fix": "important",
  minor: "minor",
  nit: "minor",
};

/** Normalises a raw severity token/heading (already lowercased by the caller is not required). */
export function normalizeSeverity(raw: string): Severity {
  const key = raw
    .trim()
    .toLowerCase()
    .replace(/^severity:\s*/, "")
    .replace(/[.:]+$/, "")
    .trim();
  const mapped = SEVERITY_WORD_MAP[key];
  if (!mapped) {
    throw new Error(`normalizeSeverity: unrecognised severity token ${JSON.stringify(raw)}`);
  }
  return mapped;
}

/**
 * Every term whose presence in a finding's text would leak its severity class, checked as a
 * whole word (case-insensitive). "should-fix" and "should fix" are checked as phrases because the
 * hyphen is not a word boundary in all four placements.
 */
export const LEAK_PATTERNS: RegExp[] = [
  /\bcritical\b/gi,
  /\bimportant\b/gi,
  /\bminor\b/gi,
  /\bblocking\b/gi,
  /\bshould-fix\b/gi,
  /\bshould\s+fix\b/gi,
  /\bnit\b/gi,
  /\bseverity\b/gi,
];

/** Returns every leak-term match still present in `text`, for the leak-check test and report. */
export function findLeaks(text: string): string[] {
  const hits: string[] = [];
  for (const pattern of LEAK_PATTERNS) {
    const re = new RegExp(pattern.source, pattern.flags);
    let m: RegExpExecArray | null;
    while ((m = re.exec(text)) !== null) {
      hits.push(m[0]);
    }
  }
  return hits;
}

/**
 * Strips every leak term from `text`, replacing each with a neutral placeholder and collapsing
 * the whitespace/punctuation debris a mid-sentence removal leaves behind. Not a redaction of
 * secrets — a redaction of the label the experiment is not allowed to see.
 */
export function stripLeakTerms(text: string): string {
  let out = text;
  for (const pattern of LEAK_PATTERNS) {
    const re = new RegExp(pattern.source, pattern.flags);
    out = out.replace(re, "[sev]");
  }
  // Collapse a run of the placeholder and surrounding markdown/punctuation debris left by a
  // positional strip (e.g. "**[sev] —**" or "([sev])") down to nothing — the placeholder itself
  // still proves the check ran (see severity_test.ts), but a run of them reads as one.
  out = out.replace(/(\[sev\][\s,;:—-]*){2,}/gi, "[sev] ");
  out = out.replace(/[ \t]{2,}/g, " ");
  out = out.replace(/\*\*\s*\[sev\]\s*\*\*/g, "[sev]");
  out = out.replace(/\(\s*\[sev\]\s*\)/g, "");
  return out.trim();
}
