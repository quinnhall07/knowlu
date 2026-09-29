// Stream J experiment E2 — the offline review-triage replay.
//
// Shared types for the corpus extracted from the C1b review documents. See
// docs/notes/2026-09-22-jev-review-replay-experiment.md for the full procedure this
// implements, and docs/reports/2026-09-22-e2-review-replay-prep.md for what running it here found.

/** Which of the two labelled sets a finding came from (procedure §1). */
export type CorpusSet = "A" | "B";

/**
 * One ordinal severity scale, normalising the two vocabularies the corpus actually uses.
 * blocking/Critical > should-fix/Important > nit/Minor — see severity.ts for the mapping
 * and the judgment calls it documents.
 */
export type Severity = "critical" | "important" | "minor";

/**
 * The controller's ruling on a finding, in the four values the procedure names (§1):
 * fixed in this round, ruled against, handed off, deferred.
 */
export type Disposition = "fixed" | "ruled_against" | "handed_off" | "deferred";

/** A finding as extracted from its source document, before the leak check runs. */
export interface RawFinding {
  /** Stable id, e.g. "A-C1", "B-task1-3", "B-final-F7". */
  id: string;
  set: CorpusSet;
  /** Repo- or worktree-relative path the finding was read from. */
  source: string;
  /** The severity word/heading as the source document spelled it, lowercased. */
  severityRaw: string;
  severity: Severity;
  disposition: Disposition;
  /** Where the disposition claim comes from (a re-review file, a citation) — for the report, not the model. */
  dispositionSource: string;
  /** The finding's body text, with every known severity-label placement already removed. */
  text: string;
  /**
   * The untouched original span, severity words and all. In-memory only — never written to the
   * persisted corpus.jsonl (extract.ts strips it before serialising) — kept so run.ts's leak-check
   * "both arms" comparison (procedure §Step 3) can show the score falling when text is stripped,
   * without a second, separately-shaped read of the source files.
   */
  rawText: string;
}

/** A finding after the secret/token/session/credential drop filter has run. */
export interface CorpusFinding extends RawFinding {
  /** True if this finding was kept. Dropped findings are excluded from the written corpus entirely. */
  kept: true;
}

export interface DroppedFinding {
  id: string;
  set: CorpusSet;
  source: string;
  /** Which keyword(s) triggered the drop — never the finding text itself. */
  matchedTerms: string[];
}

export interface ExtractionSummary {
  extractedAt: string;
  expected: number;
  rawFound: number;
  perSourceCounts: Record<string, number>;
  reconciliation: string[];
  droppedForSecrets: DroppedFinding[];
  keptCount: number;
}
