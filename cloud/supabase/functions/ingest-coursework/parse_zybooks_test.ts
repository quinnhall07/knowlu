// The frozen Python-written reference is this port's acceptance test, exactly as it is the Rust
// port's. It is READ from engine/tests/fixtures/ by relative path — never copied into cloud/,
// never written, never regenerated (CLAUDE.md). If this fails, the port is wrong.
//
// The comparison is STRUCTURAL, not byte-for-byte, and that is deliberate: JavaScript has no
// `1.0` — `JSON.stringify(1.0)` is `"1"` — so a byte comparison would fail a correct port on the
// first whole-number effort_hours. The rows are arrays, so this still pins every field and its
// order, which is the property the reference was frozen for. The Rust byte-for-byte test
// (`zybooks::tests::the_parsed_payload_matches_python_byte_for_byte`) stays where it is.
import { assert, assertEquals } from "@std/assert";
import { type Assignment, categoryOf, NotLoggedIn, parseAssignments, routeZybook } from "./parse_zybooks.ts";

const FIXTURES = new URL("../../../../engine/tests/fixtures/", import.meta.url);

const CFG = {
  categories: { HW: "hw", Lab: "lab", Project: "project" },
  effort: { minutes_per_section: 6, floors: { hw: 0.25, lab: 0.5, project: 1.0 } },
  importance: { hw: 2, lab: 2, project: 2 },
};

async function payload(): Promise<unknown> {
  // The capture is a real one and carries a UTF-8 BOM; the device's `decode_json` strips it there,
  // so no BOM ever travels on the wire. Here the file is read directly, so strip it here.
  const text = await Deno.readTextFile(new URL("zybooks-assignments.json", FIXTURES));
  return JSON.parse(text.replace(/^﻿/, ""));
}

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

Deno.test("every field of all 24 parsed assignments matches the frozen Python reference", async () => {
  const warnings: string[] = [];
  const items = parseAssignments(await payload(), "cs-100", "CS 100", CFG, "America/Chicago", warnings);
  assertEquals(items.length, 24);
  assertEquals(warnings, []);
  const reference = JSON.parse(await Deno.readTextFile(new URL("zybooks-parsed-reference.json", FIXTURES)));
  assertEquals(items.map(row), reference);
});

Deno.test("a UTC stamp becomes 23:59 the day before, in Central", async () => {
  // 2026-08-27T04:59:00Z is 2026-08-26 23:59 CDT. Getting this wrong shifts every deadline by a
  // day, which is the whole point of the system.
  const items = parseAssignments(await payload(), "cs-100", "CS 100", CFG, "America/Chicago", []);
  const hw01 = items.find((a) => a.title.includes("HW 01"));
  assert(hw01 !== undefined);
  assertEquals(hw01.due, "2026-08-26T23:59");
});

Deno.test("a 200 with success false is a dead session, never an empty semester", () => {
  let threw: unknown = null;
  try {
    parseAssignments(
      { success: false, error: "invalid token" },
      "cs-100",
      "CS 100",
      CFG,
      "America/Chicago",
      [],
    );
  } catch (e) {
    threw = e;
  }
  assert(threw instanceof NotLoggedIn, "a falsy success must be NotLoggedIn, not an empty list");
});

Deno.test("an assignment with no due date is skipped with a warning, not dropped silently", () => {
  const warnings: string[] = [];
  const items = parseAssignments(
    { assignments: [{ assignment_id: 1, title: "HW 99", visible: true, due_dates: [], sections: [] }] },
    "cs-100",
    "CS 100",
    CFG,
    "America/Chicago",
    warnings,
  );
  assertEquals(items.length, 0);
  assertEquals(warnings, ["HW 99: no due date; skipped"]);
});

Deno.test("more than one due date warns and uses the earliest", () => {
  const warnings: string[] = [];
  const items = parseAssignments(
    {
      assignments: [{
        assignment_id: 7,
        title: "HW 07",
        visible: true,
        sections: [],
        due_dates: [{ date: "2026-09-11T04:59:00Z" }, { date: "2026-09-05T04:59:00Z" }],
      }],
    },
    "cs-100",
    "CS 100",
    CFG,
    "America/Chicago",
    warnings,
  );
  assertEquals(items[0].due, "2026-09-04T23:59");
  assertEquals(warnings, ["HW 07: 2 due dates; using the earliest (2026-09-04T23:59)"]);
});

Deno.test("visible false is skipped, and a missing visible is not", () => {
  const hidden = parseAssignments(
    {
      assignments: [{
        assignment_id: 1,
        title: "HW 01",
        visible: false,
        due_dates: [{ date: "2026-09-05T04:59:00Z" }],
        sections: [],
      }],
    },
    "cs-100",
    "CS 100",
    CFG,
    "America/Chicago",
    [],
  );
  assertEquals(hidden.length, 0);
  const shown = parseAssignments(
    {
      assignments: [{
        assignment_id: 1,
        title: "HW 01",
        due_dates: [{ date: "2026-09-05T04:59:00Z" }],
        sections: [],
      }],
    },
    "cs-100",
    "CS 100",
    CFG,
    "America/Chicago",
    [],
  );
  assertEquals(shown.length, 1);
});

Deno.test("routing: mapped, ignored silently, or a warning nobody should miss", () => {
  const courses = { "cs-100-2026": { course: "cs-100", label: "CS 100" }, empty: {} };
  assertEquals(routeZybook("cs-100-2026", courses, []).kind, "mapped");
  // An empty mapping is falsy in Python and is not a course.
  assertEquals(routeZybook("empty", courses, []).kind, "unmapped");
  assertEquals(routeZybook("HowToUseZyBooks2", courses, ["HowToUseZyBooks2"]).kind, "ignored");
  assertEquals(routeZybook("new-course", courses, []).kind, "unmapped");
});

Deno.test("the first configured prefix wins, in the config's own order", () => {
  assertEquals(categoryOf("Lab 02 math_lib", CFG.categories), "lab");
  assertEquals(categoryOf("Something else", CFG.categories), null);
});
