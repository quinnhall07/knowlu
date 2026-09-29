// Pure parsing of the three finding shapes the C1b corpus uses (e2 procedure §1, §Step 1):
//   - a task review: numbered items, the severity word inside the item's leading bold span
//     ("1. **should-fix — path.** body", "1. **Nit** — body", "1. **[blocking]** body", or
//     "**1. `path` — title. (should-fix)** body" — task-3's own variant).
//   - the final review: "**F<n> — title.** body **Severity: <word>.**"
//   - the plan review: a "### Critical" / "### Important" / "### Minor" heading positions the
//     severity for every finding under it; Critical/Important are bold-headed paragraphs
//     ("**C1. title.** body"), Minor is a dash-bulleted list ("- **M1.** body").
//
// Each parser returns the item's full text with every leak term already stripped
// (severity.ts's stripLeakTerms), so nothing downstream has to remember to call it.

import { stripLeakTerms } from "./severity.ts";

export interface ParsedFinding {
  /** The bare id within its source, e.g. "1", "F7", "C3", "M9". */
  tag: string;
  severityRaw: string;
  /** The label-leak-stripped text — what the model/baselines see. */
  text: string;
  /** The untouched original span, severity words and all — only for the leak-check's "both arms" run. */
  rawText: string;
}

function extractSection(markdown: string, startHeading: string, endHeadings: string[]): string {
  const startIdx = markdown.indexOf(startHeading);
  if (startIdx === -1) {
    throw new Error(`extractSection: heading ${JSON.stringify(startHeading)} not found`);
  }
  const bodyStart = startIdx + startHeading.length;
  let endIdx = markdown.length;
  for (const h of endHeadings) {
    const i = markdown.indexOf(h, bodyStart);
    if (i !== -1 && i < endIdx) endIdx = i;
  }
  return markdown.slice(bodyStart, endIdx);
}

/**
 * Reads `startHeading` up to the NEXT "## " heading (any wording), rather than a hardcoded list of
 * expected next-heading strings. The nine source files spell the following section differently —
 * "## What I verified clean", "## Verified clean (with file:line)", "## What was verified clean" —
 * and a hardcoded list silently swallows the whole rest of the file the day a new spelling shows up
 * (task-4-review.md's "## What was verified clean" did exactly this until this function replaced
 * the list). Falls back to end-of-file if there is no next "## " heading at all.
 */
function sectionUntilNextH2(markdown: string, startHeading: string): string {
  const startIdx = markdown.indexOf(startHeading);
  if (startIdx === -1) {
    throw new Error(`sectionUntilNextH2: heading ${JSON.stringify(startHeading)} not found`);
  }
  const bodyStart = startIdx + startHeading.length;
  const nextH2 = markdown.slice(bodyStart).search(/\n## /);
  const endIdx = nextH2 === -1 ? markdown.length : bodyStart + nextH2;
  return markdown.slice(bodyStart, endIdx);
}

const TASK_SEVERITY = /\[?\b(blocking|should-fix|should\s+fix|nit)\b\]?/i;

/**
 * task-3-review.md is the one file with a trailing "**Report accuracy ...**" aside inside the
 * numbered list that is not itself a finding; it is dropped rather than folded into finding 6's
 * text. Documented here rather than silently: the aside carries no severity claim of its own.
 */
const NON_FINDING_ASIDE = /\n\n\*\*Report accuracy/;

/** Parses a task review's "## Findings" section into its numbered items. */
export function parseTaskReviewFindings(markdown: string): ParsedFinding[] {
  const section = sectionUntilNextH2(markdown, "## Findings");
  const asideAt = section.search(NON_FINDING_ASIDE);
  const bounded = asideAt === -1 ? section : section.slice(0, asideAt);

  // The number is sometimes outside the bold span ("1. **should-fix — ...**", tasks 1/2/4/5/6/7)
  // and sometimes inside it ("**1. `path` — ... (should-fix)**", task 3) — both are handled by
  // allowing an optional leading "**" before the digit.
  const starts = [...bounded.matchAll(/^\*{0,2}(\d+)\.\s+/gm)];
  if (starts.length === 0) {
    throw new Error("parseTaskReviewFindings: no numbered items found in Findings section");
  }
  const out: ParsedFinding[] = [];
  for (let i = 0; i < starts.length; i++) {
    const start = starts[i].index!;
    const end = i + 1 < starts.length ? starts[i + 1].index! : bounded.length;
    const raw = bounded.slice(start, end).trim();
    if (raw.length === 0) continue;
    const tag = starts[i][1];
    const headMatch = raw.match(/\*\*([\s\S]*?)\*\*/);
    const head = headMatch ? headMatch[1] : raw;
    const sevMatch = head.match(TASK_SEVERITY);
    if (!sevMatch) {
      throw new Error(`parseTaskReviewFindings: no severity token in item ${tag}: ${head.slice(0, 80)}`);
    }
    out.push({ tag, severityRaw: sevMatch[1], text: stripLeakTerms(raw), rawText: raw });
  }
  return out;
}

/** Parses the final review's "## Findings" section into its F1..F12 items. */
export function parseFinalReviewFindings(markdown: string): ParsedFinding[] {
  const section = sectionUntilNextH2(markdown, "## Findings");
  const starts = [...section.matchAll(/\*\*F(\d+)\s+—/g)];
  if (starts.length === 0) {
    throw new Error("parseFinalReviewFindings: no F<n> items found");
  }
  const out: ParsedFinding[] = [];
  for (let i = 0; i < starts.length; i++) {
    const start = starts[i].index!;
    const end = i + 1 < starts.length ? starts[i + 1].index! : section.length;
    const raw = section.slice(start, end).trim();
    const tag = `F${starts[i][1]}`;
    const sevMatch = raw.match(/Severity:\s*([a-zA-Z][a-zA-Z\s-]*?)\.?\*\*/i);
    if (!sevMatch) {
      throw new Error(`parseFinalReviewFindings: no "Severity: ..." trailer in ${tag}`);
    }
    out.push({ tag, severityRaw: sevMatch[1].trim(), text: stripLeakTerms(raw), rawText: raw });
  }
  return out;
}

interface PlanSection {
  heading: string;
  severityRaw: "critical" | "important" | "minor";
  tagPrefix: "C" | "I" | "M";
}

const PLAN_SECTIONS: PlanSection[] = [
  { heading: "### Critical", severityRaw: "critical", tagPrefix: "C" },
  { heading: "### Important", severityRaw: "important", tagPrefix: "I" },
  { heading: "### Minor", severityRaw: "minor", tagPrefix: "M" },
];

/**
 * Parses the plan review's "## Findings" section, which positions severity by heading rather
 * than by word: everything under "### Critical" is Critical, and so on, until the next "###" or
 * "## Fidelity". Critical/Important items are bold-headed paragraphs ("**C1. ...**"); Minor items
 * are a dash-bulleted list ("- **M1.** ...").
 */
export function parsePlanReviewFindings(markdown: string): ParsedFinding[] {
  const section = extractSection(markdown, "## Findings", ["\n## Fidelity"]);
  const out: ParsedFinding[] = [];

  for (let s = 0; s < PLAN_SECTIONS.length; s++) {
    const { heading, severityRaw, tagPrefix } = PLAN_SECTIONS[s];
    const nextHeadings = PLAN_SECTIONS.slice(s + 1).map((p) => p.heading);
    const idx = section.indexOf(heading);
    if (idx === -1) throw new Error(`parsePlanReviewFindings: heading ${heading} not found`);
    let end = section.length;
    for (const h of nextHeadings) {
      const i = section.indexOf(h, idx + heading.length);
      if (i !== -1 && i < end) end = i;
    }
    const body = section.slice(idx + heading.length, end);

    if (tagPrefix === "M") {
      const starts = [...body.matchAll(/^-\s+\*\*(M\d+)\.\*\*/gm)];
      for (let i = 0; i < starts.length; i++) {
        const start = starts[i].index!;
        const itemEnd = i + 1 < starts.length ? starts[i + 1].index! : body.length;
        const raw = body.slice(start, itemEnd).trim();
        out.push({ tag: starts[i][1], severityRaw, text: stripLeakTerms(raw), rawText: raw });
      }
    } else {
      const pattern = new RegExp(`\\*\\*(${tagPrefix}\\d+)\\.\\s`, "g");
      const starts = [...body.matchAll(pattern)];
      for (let i = 0; i < starts.length; i++) {
        const start = starts[i].index!;
        const itemEnd = i + 1 < starts.length ? starts[i + 1].index! : body.length;
        const raw = body.slice(start, itemEnd).trim();
        out.push({ tag: starts[i][1], severityRaw, text: stripLeakTerms(raw), rawText: raw });
      }
    }
  }
  return out;
}
