import { assertEquals, assertThrows } from "@std/assert";
import { allCuratedIds, dispositionFor } from "./dispositions.ts";

Deno.test("dispositionFor resolves a known Set A and Set B id", () => {
  assertEquals(dispositionFor("A-C1").disposition, "fixed");
  assertEquals(dispositionFor("B-task3-6").disposition, "handed_off");
  assertEquals(dispositionFor("B-final-F5").disposition, "handed_off");
  assertEquals(dispositionFor("B-task5-1").disposition, "deferred");
});

Deno.test("dispositionFor throws on an id the table does not cover", () => {
  assertThrows(() => dispositionFor("A-C99"), Error, "no curated disposition");
});

Deno.test("the curated table has exactly 60 entries — 24 Set A + 36 Set B", () => {
  const ids = allCuratedIds();
  assertEquals(ids.filter((i) => i.startsWith("A-")).length, 24);
  assertEquals(ids.filter((i) => i.startsWith("B-")).length, 36);
  assertEquals(ids.length, 60);
});

Deno.test("every disposition value used is one of the four the procedure names", () => {
  const allowed = new Set(["fixed", "ruled_against", "handed_off", "deferred"]);
  for (const id of allCuratedIds()) {
    const { disposition } = dispositionFor(id);
    assertEquals(allowed.has(disposition), true, `${id} has an unrecognised disposition`);
  }
});
