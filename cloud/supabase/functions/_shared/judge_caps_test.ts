// The caps' two pinned numbers, so a future edit to either one is a deliberate diff, not a drift
// nobody notices (whole-branch review M1).
import { assertEquals } from "@std/assert";
import { DAILY_CAP, MONTHLY_CEILING_USD } from "./judge_caps.ts";

Deno.test("MONTHLY_CEILING_USD is pinned at $2.00 (the provider swap plan's runaway-guard constraint)", () => {
  assertEquals(MONTHLY_CEILING_USD, 2.0);
});

Deno.test("DAILY_CAP is pinned per kind", () => {
  assertEquals(DAILY_CAP, { task: 60, event: 80, email: 120 });
});
