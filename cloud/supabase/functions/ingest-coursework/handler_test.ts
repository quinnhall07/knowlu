import { assertEquals } from "@std/assert";
import { ingestHandler } from "./handler.ts";

const FIXTURES = new URL("../../../../engine/tests/fixtures/", import.meta.url);
const OK = () => Promise.resolve({ account_id: "acct-1" });

Deno.test("both sources parse and reconcile in one round trip, warnings prefixed by source", async () => {
  const payload = JSON.parse(
    (await Deno.readTextFile(new URL("zybooks-assignments.json", FIXTURES))).replace(/^\uFEFF/, ""),
  );
  const html = await Deno.readTextFile(new URL("vhl-dashboard.html", FIXTURES));
  const body = JSON.stringify({
    timezone: "America/Chicago",
    sources: [
      {
        name: "zybooks",
        config: {
          courses: { "cs-100-2026": { course: "cs-100", label: "CS 100" } },
          ignore: ["HowToUseZyBooks2"],
          categories: { HW: "hw", Lab: "lab", Project: "project" },
          effort: { minutes_per_section: 6, floors: { hw: 0.25, lab: 0.5, project: 1.0 } },
          importance: { hw: 2, lab: 2, project: 2 },
        },
        books: [
          { code: "cs-100-2026", payload },
          { code: "HowToUseZyBooks2", payload: { assignments: [] } },
          { code: "surprise-101", payload: { assignments: [] } },
        ],
      },
      {
        name: "vhl",
        config: {
          sections: { "2102121": { course: "gn-103", label: "GN 103 Hausaufgaben" } },
          importance: 3,
          importance_reason: "GN 103 accepts no late Hausaufgaben at all",
        },
        html,
      },
    ],
  });
  const response = await ingestHandler(OK)(
    new Request("http://127.0.0.1/ingest-coursework", { method: "POST", body }),
  );
  assertEquals(response.status, 200);
  const reply = await response.json();
  assertEquals(reply.assignments.length, 24 + 13);
  assertEquals(reply.warnings, ["zybooks: zybook surprise-101 not in config; skipped"]);
});

Deno.test("a dead session is a named warning and zero items, never an empty semester", async () => {
  const body = JSON.stringify({
    timezone: "America/Chicago",
    sources: [{
      name: "zybooks",
      config: {},
      books: [{ code: "x", payload: { success: false, error: "invalid token" } }],
    }],
  });
  const response = await ingestHandler(OK)(
    new Request("http://127.0.0.1/ingest-coursework", { method: "POST", body }),
  );
  const reply = await response.json();
  assertEquals(reply.assignments, []);
  assertEquals(reply.warnings.length, 1);
  assertEquals(reply.warnings[0].startsWith("zybooks: session invalid"), true);
});

Deno.test("one source failing never stops the other", async () => {
  const html = await Deno.readTextFile(new URL("vhl-dashboard.html", FIXTURES));
  const body = JSON.stringify({
    timezone: "America/Chicago",
    sources: [
      { name: "zybooks", config: {}, books: [{ code: "x", payload: { success: false } }] },
      {
        name: "vhl",
        config: {
          sections: { "2102121": { course: "gn-103", label: "GN 103 Hausaufgaben" } },
          importance: 3,
          importance_reason: "GN 103 accepts no late Hausaufgaben at all",
        },
        html,
      },
    ],
  });
  const reply = await (await ingestHandler(OK)(
    new Request("http://127.0.0.1/ingest-coursework", { method: "POST", body }),
  )).json();
  assertEquals(reply.assignments.length, 13);
});
