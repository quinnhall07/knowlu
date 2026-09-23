import { assertEquals } from "@std/assert";
import { ingestHandler, suggestCourse } from "./handler.ts";

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
  // R-OB-1: `surprise-101` is unmapped, so it is a proposal now, not a warning.
  assertEquals(reply.warnings, []);
  assertEquals(reply.proposals, [{
    source: "zybooks",
    key: "surprise-101",
    label: "surprise-101",
    suggested_course: "surprise-101",
  }]);
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

Deno.test("an unknown zybook is one proposal and zero warnings", async () => {
  // R-OB-1: the old behaviour was `warnings: ["zybooks: zybook X not in config; skipped"]` and
  // nothing else — a line in a log nobody reads, on the very first run of a vault whose whole
  // point was to show the student their coursework.
  const body = JSON.stringify({
    timezone: "America/Chicago",
    sources: [{
      name: "zybooks",
      config: { courses: {}, ignore: ["HowToUseZyBooks2"], categories: {}, effort: {}, importance: {} },
      books: [{ code: "UACS100Fall2026", payload: { assignments: [] } }],
    }],
  });
  const reply = await (await ingestHandler(OK)(
    new Request("http://127.0.0.1/ingest-coursework", { method: "POST", body }),
  )).json();
  assertEquals(reply.assignments, []);
  // R-C1c-10 (supersedes R-C2 pre-flight correction F3): the one book in this source is
  // unmapped, so it produced a proposal instead of an item. That proposal is the honest channel
  // for "the portal returned work" — a second, `0 assignments parsed` complaint layered under it
  // would be the very bug this rule exists to stop.
  assertEquals(reply.warnings, []);
  assertEquals(reply.proposals, [{
    source: "zybooks",
    key: "UACS100Fall2026",
    label: "UACS100Fall2026",
    suggested_course: "cs-100",
  }]);
});

Deno.test("an ignored book is still silent, and a known one still yields its items", async () => {
  // `HowToUseZyBooks2` is zyBooks' own onboarding book: a proposal there would fire on every
  // healthy run for every student, which is exactly the noise R-OB-1 is trying to stop.
  const payload = JSON.parse(
    (await Deno.readTextFile(new URL("zybooks-assignments.json", FIXTURES))).replace(/^\uFEFF/, ""),
  );
  const body = JSON.stringify({
    timezone: "America/Chicago",
    sources: [{
      name: "zybooks",
      config: {
        courses: { "cs-100-2026": { course: "cs-100", label: "CS 100" } },
        ignore: ["HowToUseZyBooks2"],
        categories: { HW: "hw", Lab: "lab", Project: "project" },
        effort: { minutes_per_section: 6, floors: { hw: 0.25, lab: 0.5, project: 1.0 } },
        importance: { hw: 2, lab: 2, project: 2 },
      },
      books: [{ code: "cs-100-2026", payload }, { code: "HowToUseZyBooks2", payload: { assignments: [] } }],
    }],
  });
  const reply = await (await ingestHandler(OK)(
    new Request("http://127.0.0.1/ingest-coursework", { method: "POST", body }),
  )).json();
  assertEquals(reply.assignments.length, 24);
  assertEquals(reply.proposals, []);
  assertEquals(reply.warnings, []);
});

Deno.test("an unmapped VHL section becomes a proposal with no course to suggest", async () => {
  // The dashboard carries a section id and a due date and no course name at all, so there is
  // nothing to guess from — and a guess would be worse than a question. The card asks.
  const html = await Deno.readTextFile(new URL("vhl-dashboard.html", FIXTURES));
  const body = JSON.stringify({
    timezone: "America/Chicago",
    sources: [{ name: "vhl", config: { sections: {}, importance: 3, importance_reason: "" }, html }],
  });
  const reply = await (await ingestHandler(OK)(
    new Request("http://127.0.0.1/ingest-coursework", { method: "POST", body }),
  )).json();
  assertEquals(reply.assignments, []);
  assertEquals(reply.proposals, [{
    source: "vhl",
    key: "2102121",
    label: "VHL section 2102121",
    suggested_course: null,
  }]);
  // R-C1c-10: every row in this dashboard is in that one unmapped section, so the source's items
  // are empty — but it produced a proposal, and that proposal is the honest channel for "the
  // portal returned work", not the empty-parse failure line on top of it.
  assertEquals(reply.warnings, []);
});

Deno.test("a VHL source with no rows at all still gets the failure warning", async () => {
  // R-C1c-10's other half: a genuinely empty dashboard is not a question — it never produced a
  // proposal, so the empty-parse rule must still fire, exactly as strongly as before.
  const html = '<div class="js-student-dashboard-app" data-assignment-summaries="[]"></div>';
  const body = JSON.stringify({
    timezone: "America/Chicago",
    sources: [{ name: "vhl", config: { sections: {}, importance: 3, importance_reason: "" }, html }],
  });
  const reply = await (await ingestHandler(OK)(
    new Request("http://127.0.0.1/ingest-coursework", { method: "POST", body }),
  )).json();
  assertEquals(reply.assignments, []);
  assertEquals(reply.proposals, []);
  assertEquals(reply.warnings, ["vhl: 0 assignments parsed; treating as failure"]);
});

Deno.test("an unmapped source's proposal never excuses a truly empty sibling in the same request", async () => {
  // R-C1c-10: `proposals` is shared across sources in one request — a VHL proposal must not
  // quiet an empty zyBooks parse in the same request, or the reverse. Counted per source.
  const html = await Deno.readTextFile(new URL("vhl-dashboard.html", FIXTURES));
  const body = JSON.stringify({
    timezone: "America/Chicago",
    sources: [
      {
        name: "zybooks",
        config: { courses: {}, ignore: [], categories: {}, effort: {}, importance: {} },
        books: [],
      },
      {
        name: "vhl",
        config: { sections: {}, importance: 3, importance_reason: "" },
        html,
      },
    ],
  });
  const reply = await (await ingestHandler(OK)(
    new Request("http://127.0.0.1/ingest-coursework", { method: "POST", body }),
  )).json();
  assertEquals(reply.assignments, []);
  assertEquals(reply.warnings, ["zybooks: 0 assignments parsed; treating as failure"]);
  assertEquals(reply.proposals, [{
    source: "vhl",
    key: "2102121",
    label: "VHL section 2102121",
    suggested_course: null,
  }]);
});

Deno.test("a course is suggested from a zybook code, or not at all", () => {
  // The institution prefix comes off only when the term suffix says the code carries one, so a
  // department that happens to be four letters keeps all four.
  assertEquals(suggestCourse("UACS100Fall2026"), "cs-100");
  assertEquals(suggestCourse("UAMATH120Fall2026"), "math-120");
  assertEquals(suggestCourse("PH106Spring2027"), "ph-106");
  assertEquals(suggestCourse("MATH125"), "math-125");
  assertEquals(suggestCourse("cs-100-2026"), "cs-100");
  // zyBooks' own onboarding book, and a VHL section id: neither is a course code.
  assertEquals(suggestCourse("HowToUseZyBooks2"), null);
  assertEquals(suggestCourse("2102121"), null);
  assertEquals(suggestCourse("SomeBookWithNoCode"), null);
  assertEquals(suggestCourse(""), null);
});
