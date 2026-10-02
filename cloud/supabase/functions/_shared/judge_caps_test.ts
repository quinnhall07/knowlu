// The caps' two pinned numbers, so a future edit to either one is a deliberate diff, not a drift
// nobody notices (whole-branch review M1).
import { assertEquals } from "@std/assert";
import { DAILY_CAP, MAIL_DAILY_CAP, MONTHLY_CEILING_USD } from "./judge_caps.ts";

Deno.test("MONTHLY_CEILING_USD is pinned at $2.00 (the provider swap plan's runaway-guard constraint)", () => {
  assertEquals(MONTHLY_CEILING_USD, 2.0);
});

Deno.test("DAILY_CAP is pinned per kind", () => {
  assertEquals(DAILY_CAP, { task: 60, event: 80, email: 120 });
});

// Email plan T3c (spec §5.1, plan P6): the mail kinds' daily caps sit in their own record, keyed by
// `MailKind`, so `DAILY_CAP` and every `Record<Kind, …>` stay as they were.
Deno.test("mail_daily_caps_are_120_for_claims_and_20_for_resolve", () => {
  assertEquals(MAIL_DAILY_CAP.mail_claim, 120);
  assertEquals(MAIL_DAILY_CAP.mail_resolve, 20);
  // The digest (T16, optional) is not built: no call is allowed until its own task sets a cap.
  assertEquals(MAIL_DAILY_CAP, { mail_claim: 120, mail_resolve: 20, mail_digest: 0 });
});

Deno.test("the_existing_kinds_caps_are_unchanged", () => {
  assertEquals(DAILY_CAP, { task: 60, event: 80, email: 120 });
  assertEquals(Object.keys(DAILY_CAP).sort(), ["email", "event", "task"]);
});
