import { assertEquals } from "@std/assert";
import { findLeaks } from "./severity.ts";
import { parseFinalReviewFindings, parsePlanReviewFindings, parseTaskReviewFindings } from "./parsers.ts";

// --- Small, structurally faithful fixtures for each of the shapes the real files use ---

const TASK1_STYLE = `## Findings

1. **should-fix — cloud/supabase/functions/account/handler.ts:148-156.** A body of literal \`null\`
   is valid JSON, so \`readJson\` does not throw.
   *Fix:* do the thing.
2. **should-fix — handler_test.ts:154-163.** POST /consent is missing from the loop.
7. **nit — handler.ts:154-156.** tos_version and privacy_version go into the log verbatim.

## What I verified clean

- clean stuff
`;

const TASK2_STYLE = `## Findings

None blocking. Two nits, both already disclosed and immaterial:

1. **Nit** — \`handler_test.ts:44-56\` (the null-attestation stub). Confirmed whitespace-only.
2. **Nit** — report's "Deviations" item 2 is a no-op observation.

## Verified clean

- fine
`;

const TASK3_STYLE = `## Findings

**1. \`app/src/account.rs:379-406\` — the deadline is not checked. (should-fix)** \`serve_one_callback\`'s
loop only tests the deadline inside one arm.

**2. \`app/src/account.rs:1074\` — the bind address is the boundary and no test pins it. (should-fix)**
\`TcpListener::bind\` lives only in the command.

**3. \`app/src/account.rs:367-371\` — stronger than the code supports. (nit, doc accuracy)** The challenge
is not a shared value.

## Verified clean

- fine
`;

const TASK7_STYLE = `## Findings

1. **[blocking]** \`site/privacy.html:32\`. A splice produced a broken sentence.
2. **[nit]** \`app/tests/static_assets.rs\`'s new pin only catches a reverted bullet.

## Verified clean

- fine
`;

const FINAL_STYLE = `## Findings

**F1 — the browser is told the wrong thing.** \`app/src/account.rs:419\` serves the page before deciding.
**Severity: should-fix.**

**F8 — the rate limit table declares only one key.** \`config.toml:57-58\`. Completeness, not a hole.
**Severity: nit.**

## Cross-task assessment

- fine
`;

const PLAN_STYLE = `## Findings

### Critical

**C1. \`plan.md:141-158\` (Task 1 step 1) — the migration bumps the count and nothing bumps it.**
\`migrations_test.ts:303\` asserts the old number.

**C2. \`plan.md:911\` (Task 4 step 1) — an assertion can never pass.**
Some other body text about the assertion.

### Important

**I1. \`plan.md:166-192\` (Task 1 step 2) — the trigger drops the raise outright.**
Body text here.

### Minor

- **M1.** Line cites drift by one or two.
- **M2.** H1's count is off by one, the arithmetic still holds.

## Fidelity

- fine
`;

Deno.test("parseTaskReviewFindings extracts the numbered items with their severity and leak-free text", () => {
  const items = parseTaskReviewFindings(TASK1_STYLE);
  assertEquals(items.length, 3);
  assertEquals(items.map((i) => i.tag), ["1", "2", "7"]);
  assertEquals(items.map((i) => i.severityRaw.toLowerCase()), ["should-fix", "should-fix", "nit"]);
  for (const item of items) assertEquals(findLeaks(item.text), []);
  assertEquals(items[0].text.includes("readJson"), true);
});

Deno.test("parseTaskReviewFindings handles the whole-bold-span 'Nit' shape (tasks 2/4/6)", () => {
  const items = parseTaskReviewFindings(TASK2_STYLE);
  assertEquals(items.length, 2);
  assertEquals(items.map((i) => i.severityRaw.toLowerCase()), ["nit", "nit"]);
  for (const item of items) assertEquals(findLeaks(item.text), []);
});

Deno.test("parseTaskReviewFindings handles the trailing-parenthetical shape (task 3)", () => {
  const items = parseTaskReviewFindings(TASK3_STYLE);
  assertEquals(items.length, 3);
  assertEquals(items.map((i) => i.severityRaw.toLowerCase()), ["should-fix", "should-fix", "nit"]);
  for (const item of items) assertEquals(findLeaks(item.text), []);
});

Deno.test("parseTaskReviewFindings handles the bracketed shape (task 7)", () => {
  const items = parseTaskReviewFindings(TASK7_STYLE);
  assertEquals(items.length, 2);
  assertEquals(items.map((i) => i.severityRaw.toLowerCase()), ["blocking", "nit"]);
  for (const item of items) assertEquals(findLeaks(item.text), []);
});

Deno.test("parseFinalReviewFindings reads the trailing Severity marker per F-item", () => {
  const items = parseFinalReviewFindings(FINAL_STYLE);
  assertEquals(items.length, 2);
  assertEquals(items.map((i) => i.tag), ["F1", "F8"]);
  assertEquals(items.map((i) => i.severityRaw.toLowerCase()), ["should-fix", "nit"]);
  for (const item of items) assertEquals(findLeaks(item.text), []);
});

Deno.test("parsePlanReviewFindings assigns severity by heading, both paragraph and dash-bullet shapes", () => {
  const items = parsePlanReviewFindings(PLAN_STYLE);
  assertEquals(items.length, 5);
  assertEquals(
    items.map((i) => `${i.tag}:${i.severityRaw}`),
    ["C1:critical", "C2:critical", "I1:important", "M1:minor", "M2:minor"],
  );
  for (const item of items) assertEquals(findLeaks(item.text), []);
});

// --- Integration: parse the real corpus and reconcile the count (procedure §Step 1) ---

const CORPUS_ROOT =
  "C:\\Users\\danie\\GitHub\\knowlu\\.claude\\worktrees\\c1b-sign-in\\.superpowers\\sdd\\2026-09-17-c1b-sign-in-plan";
const PLAN_REVIEW_PATH =
  "C:\\Users\\danie\\GitHub\\knowlu\\.claude\\worktrees\\j-e2\\docs\\reports\\2026-09-17-c1b-sign-in-plan-review.md";

// Reconciled by hand against every source file, 2026-09-22 — see
// docs/reports/2026-09-22-e2-review-replay-prep.md for the discrepancy this test pins: the
// experiment note's own table (procedure §1) undercounts task 3 by one nit (item 4,
// `app/src/account.rs:395-398`, "(nit)"), which its own table's total (35) does not include.
// This test's expectation is the real per-file count, re-derived from the documents themselves,
// which CLAUDE.md's "if the engine disagrees with a fixture, the engine is wrong" spirit says is
// the one to trust over a hand-kept table.
const EXPECTED_TASK_COUNTS: Record<string, number> = {
  "task-1-review.md": 10,
  "task-2-review.md": 2,
  "task-3-review.md": 6,
  "task-4-review.md": 1,
  "task-5-review.md": 1,
  "task-6-review.md": 2,
  "task-7-review.md": 2,
};

Deno.test({
  name: "reconciliation: each task review's real finding count, against the corpus on disk",
  ignore: !pathExists(CORPUS_ROOT),
  fn: async () => {
    for (const [file, expected] of Object.entries(EXPECTED_TASK_COUNTS)) {
      const text = await Deno.readTextFile(`${CORPUS_ROOT}\\${file}`);
      const items = parseTaskReviewFindings(text);
      assertEquals(items.length, expected, `${file}: expected ${expected} findings`);
    }
  },
});

Deno.test({
  name: "reconciliation: the final review has 12 findings (F1-F12), 7 should-fix + 5 nit",
  ignore: !pathExists(CORPUS_ROOT),
  fn: async () => {
    const text = await Deno.readTextFile(`${CORPUS_ROOT}\\final-review.md`);
    const items = parseFinalReviewFindings(text);
    assertEquals(items.length, 12);
    const bySeverity = items.map((i) => i.severityRaw.toLowerCase());
    assertEquals(bySeverity.filter((s) => s === "should-fix").length, 7);
    assertEquals(bySeverity.filter((s) => s === "nit").length, 5);
  },
});

Deno.test({
  name: "reconciliation: the plan review has 24 findings — 6 Critical, 8 Important, 10 Minor",
  ignore: !pathExists(PLAN_REVIEW_PATH),
  fn: async () => {
    const text = await Deno.readTextFile(PLAN_REVIEW_PATH);
    const items = parsePlanReviewFindings(text);
    assertEquals(items.length, 24);
    assertEquals(items.filter((i) => i.severityRaw === "critical").length, 6);
    assertEquals(items.filter((i) => i.severityRaw === "important").length, 8);
    assertEquals(items.filter((i) => i.severityRaw === "minor").length, 10);
    for (const item of items) assertEquals(findLeaks(item.text), [], `leak in ${item.tag}`);
  },
});

function pathExists(path: string): boolean {
  try {
    Deno.statSync(path);
    return true;
  } catch {
    return false;
  }
}
