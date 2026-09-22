import { assertEquals } from "@std/assert";
import { noKeyMessage, OPENROUTER_KEY_TARGET, resolveKeyWith } from "./credentials.ts";

Deno.test("resolveKeyWith returns the first tier that supplies a non-empty value", async () => {
  const result = await resolveKeyWith([
    { source: "tier1", probe: () => null },
    { source: "tier2", probe: () => "  " }, // blank counts as absent
    { source: "tier3", probe: () => "the-key" },
  ]);
  assertEquals(result, { key: "the-key", source: "tier3" });
});

Deno.test("resolveKeyWith prefers an earlier tier over a later one that would also succeed", async () => {
  const result = await resolveKeyWith([
    { source: "tier1", probe: () => "first" },
    { source: "tier2", probe: () => "second" },
  ]);
  assertEquals(result, { key: "first", source: "tier1" });
});

Deno.test("resolveKeyWith never inspects tiers past the first success (order is the contract)", async () => {
  let tier2Called = false;
  await resolveKeyWith([
    { source: "tier1", probe: () => "first" },
    {
      source: "tier2",
      probe: () => {
        tier2Called = true;
        return "second";
      },
    },
  ]);
  assertEquals(tier2Called, false);
});

Deno.test("resolveKeyWith returns source 'none' and a null key when every tier is empty", async () => {
  const result = await resolveKeyWith([
    { source: "tier1", probe: () => null },
    { source: "tier2", probe: () => "" },
  ]);
  assertEquals(result, { key: null, source: "none" });
});

Deno.test("resolveKeyWith supports async probes", async () => {
  const result = await resolveKeyWith([
    { source: "tier1", probe: async () => await Promise.resolve(null) },
    { source: "tier2", probe: async () => await Promise.resolve("async-key") },
  ]);
  assertEquals(result, { key: "async-key", source: "tier2" });
});

Deno.test("noKeyMessage names the three places and never a value", () => {
  const msg = noKeyMessage();
  assertEquals(msg.includes("process environment"), true);
  assertEquals(msg.includes("USER-scope"), true);
  assertEquals(msg.includes(OPENROUTER_KEY_TARGET), true);
  assertEquals(msg.includes("--dry-run"), true);
});
