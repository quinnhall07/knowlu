import { assert, assertEquals, assertThrows } from "@std/assert";
import type { Assignment } from "./parse_zybooks.ts";
import { NotLoggedIn } from "./parse_zybooks.ts";
import { parseDashboard, parseDurationHours } from "./parse_vhl.ts";

const FIXTURES = new URL("../../../../engine/tests/fixtures/", import.meta.url);

const CFG = {
  sections: { "2102121": { course: "gn-103", label: "GN 103 Hausaufgaben" } },
  importance: 3,
  importance_reason: "GN 103 accepts no late Hausaufgaben at all",
};

function row(a: Assignment): unknown[] {
  return [
    a.uid,
    a.slug,
    a.title,
    a.due,
    a.course,
    a.effort_hours,
    a.effort_confidence,
    a.effort_source,
    a.importance,
    a.importance_reason,
    a.progress,
    a.created_by,
    a.body,
  ];
}

Deno.test("every field of all 13 buckets matches the frozen Python reference", async () => {
  const html = await Deno.readTextFile(new URL("vhl-dashboard.html", FIXTURES));
  const warnings: string[] = [];
  const items = parseDashboard(html, CFG, warnings);
  assertEquals(items.length, 13);
  const reference = JSON.parse(await Deno.readTextFile(new URL("vhl-parsed-reference.json", FIXTURES)));
  assertEquals(items.map(row), reference);
  // The reference is the only one in the port carrying a non-ASCII character — the em dash in
  // every title — which makes it the test that catches a writer slipping into ASCII escaping.
  assert(items[0].title.includes("—"));
});

Deno.test("an unauthenticated page is a dead session, never an empty semester", () => {
  assertThrows(
    () => parseDashboard("<html><body>Please sign in</body></html>", CFG, []),
    NotLoggedIn,
    "dashboard mount element absent",
  );
});

Deno.test("a section the config does not name is skipped with a warning", () => {
  const html =
    `<div class="js-student-dashboard-app" data-assignment-summaries="[{&quot;due_date&quot;:&quot;2026-09-01&quot;,&quot;detail_url&quot;:&quot;/sections/999/x&quot;,&quot;estimated_time&quot;:&quot;10m&quot;,&quot;assignment_count&quot;:1,&quot;activities_remaining&quot;:1,&quot;percentage_complete&quot;:0}]"></div>`;
  const warnings: string[] = [];
  assertEquals(parseDashboard(html, CFG, warnings).length, 0);
  assertEquals(warnings, ["section 999 not in config; skipped"]);
});

Deno.test("durations read the way Python read them, and an unreadable one is an error", () => {
  assertEquals(parseDurationHours("1h 22m"), 1.37);
  assertEquals(parseDurationHours("27m"), 0.45);
  assertEquals(parseDurationHours("3h 4m"), 3.07);
  // The empty string matches the pattern but fires neither group: returning 0.0 would be worse
  // than failing, because a zero-effort task fits any capacity gap and never needs a slot.
  assertThrows(() => parseDurationHours(""), Error, "unparseable duration");
  assertThrows(() => parseDurationHours("about an hour"), Error, "unparseable duration");
});

Deno.test("effort_hours is the WHOLE bucket, not what is left", () => {
  // estimated_time covers what remains; `Task.remaining_hours` is effort_hours * (1 - progress/100),
  // so a bucket 10% done with 1h 22m left is a 1.52-hour bucket.
  const html =
    `<div class="js-student-dashboard-app" data-assignment-summaries="[{&quot;due_date&quot;:&quot;2026-08-26&quot;,&quot;detail_url&quot;:&quot;/sections/2102121/x&quot;,&quot;estimated_time&quot;:&quot;1h 22m&quot;,&quot;assignment_count&quot;:10,&quot;activities_remaining&quot;:9,&quot;percentage_complete&quot;:10}]"></div>`;
  const items = parseDashboard(html, CFG, []);
  assertEquals(items[0].effort_hours, 1.52);
  assertEquals(items[0].progress, 10);
  assertEquals(items[0].effort_confidence, "high");
  assertEquals(items[0].effort_source, "vendor");
});
