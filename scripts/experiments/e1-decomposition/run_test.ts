import { assert, assertEquals } from "@std/assert";
import { loadSeed } from "../../../cloud/eval/loader.ts";
import { eventRowFromMigrations } from "./model_row.ts";
import { b1RuleLines, COST_NOTE, dryRunLines, type EventCase } from "./run.ts";

async function seedCases(): Promise<EventCase[]> {
  return (await loadSeed()).filter((r) => r.kind === "event").map((r) => ({
    id: r.id,
    item: r.request.item as Record<string, unknown>,
    seed: (r.request.heuristics_seed ?? {}) as Record<string, unknown>,
    label: String(r.theirs.verdict),
  }));
}

Deno.test("E1 runs over T2's 26 event cases", async () => {
  assertEquals((await seedCases()).length, 26);
});

Deno.test("the dry run prints one request body per arm and an estimate, and carries no credential", async () => {
  const lines = dryRunLines(await seedCases(), await eventRowFromMigrations());
  const text = lines.join("\n");
  assertEquals(lines.filter((l) => l.startsWith("--- request body, arm ")).length, 2);
  assert(text.includes("cost estimate: 52 calls"), text);
  assert(text.includes("audience_excludes_student"), "the event-4 body must carry the rule fields");
  assert(!/authorization|bearer/i.test(text), "a dry run must never print a header or a key");
});

Deno.test("the output states the COST map's unruled fall-through, unsure included", () => {
  assert(COST_NOTE.includes("UNRULED"));
  assert(COST_NOTE.includes("unsure"));
  assert(COST_NOTE.includes("default cost 1"));
});

Deno.test("the B1 rule is stated with the gap 26 cases require", () => {
  const text = b1RuleLines(26).join("\n");
  assert(text.includes("6 wins with 0 losses"), text);
  assert(text.includes("close"), text);
});
