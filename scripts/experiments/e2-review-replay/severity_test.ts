import { assertEquals, assertThrows } from "@std/assert";
import { findLeaks, normalizeSeverity, stripLeakTerms } from "./severity.ts";

Deno.test("normalizeSeverity maps every raw spelling the corpus uses", () => {
  assertEquals(normalizeSeverity("Critical"), "critical");
  assertEquals(normalizeSeverity("blocking"), "critical");
  assertEquals(normalizeSeverity("[blocking]"), "critical");
  assertEquals(normalizeSeverity("Important"), "important");
  assertEquals(normalizeSeverity("should-fix"), "important");
  assertEquals(normalizeSeverity("should fix"), "important");
  assertEquals(normalizeSeverity("Minor"), "minor");
  assertEquals(normalizeSeverity("nit"), "minor");
  assertEquals(normalizeSeverity("Severity: should-fix."), "important");
});

Deno.test("normalizeSeverity rejects an unrecognised token rather than guessing", () => {
  assertThrows(() => normalizeSeverity("urgent"));
});

Deno.test("findLeaks catches the four real placements from the corpus", () => {
  // task review, first token
  assertEquals(findLeaks("1. **should-fix — cloud/supabase/functions/account/handler.ts:148.**"), [
    "should-fix",
  ]);
  assertEquals(findLeaks("1. **[blocking]** the loopback listener never checks the path."), [
    "blocking",
  ]);
  // final review, trailing marker
  assertEquals(findLeaks("A student reading the page hits a fragment. **Severity: should-fix.**"), [
    "should-fix",
    "Severity",
  ]);
  // plan review, heading
  assertEquals(findLeaks("### Critical\n\n**C1. plan.md:141 — the migration bumps the count.**"), [
    "Critical",
  ]);
});

Deno.test("findLeaks is empty over clean prose that happens to share no vocabulary", () => {
  assertEquals(
    findLeaks(
      "the migration replaces one function and adds no table, policy, column, grant or revoke",
    ),
    [],
  );
});

Deno.test("stripLeakTerms removes every leak term the four placements introduce", () => {
  const taskReview =
    "1. **should-fix — cloud/supabase/functions/account/handler.ts:148-156.** A body of literal null is valid JSON.";
  const stripped = stripLeakTerms(taskReview);
  assertEquals(findLeaks(stripped), []);

  const finalReview = "The result is a standalone fragment. **Severity: should-fix.**";
  assertEquals(findLeaks(stripLeakTerms(finalReview)), []);

  const heading = "### Critical\n\n**C1. the migration bumps the count and nothing bumps it.**";
  assertEquals(findLeaks(stripLeakTerms(heading)), []);

  const blocking = "1. **[blocking]** the loopback listener never checks the path.";
  assertEquals(findLeaks(stripLeakTerms(blocking)), []);
});

Deno.test("stripLeakTerms leaves substantive text intact, not just the label", () => {
  const text = "5. should-fix — the listener accepts exactly one connection and never checks the path.";
  const stripped = stripLeakTerms(text);
  assertEquals(stripped.includes("the listener accepts exactly one connection"), true);
  assertEquals(stripped.includes("never checks the path"), true);
});
