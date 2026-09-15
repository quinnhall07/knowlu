// TDD, per CLAUDE.md: this file is written before `schema.ts` or `loader.ts` exist, and fails to
// even load (RED — the two imports below resolve to nothing) until they do.
//
// `cloud/eval/` never reads an archive (ruling R-C2-E12, 2026-09-14: P4 was declined — "nothing is
// ever read from the quinn-ops archive"). Every fixture in this file is written by hand and is
// obviously synthetic. Nothing here proves a real record is SAFE, only that a record shaped like
// the ones below is caught or accepted the way its rule says it should be — the eval suite's real
// cases arrive later, one consented correction at a time (a later stream's opt-in), and go through
// exactly these same two checks before they are trusted.
import { assert, assertEquals } from "@std/assert";
import { loadSeed } from "./loader.ts";
import { scrubViolations, type SeedRecord, validateSeedRecord } from "./schema.ts";

// ---------------------------------------------------------------------------------------------
// Three valid records, one per kind — each is `request` shaped exactly as the device sends it
// today (`engine/src/cloudmodel.rs`'s `task_request` / `event_request` / `email_request`), paired
// with a `theirs` that names only fields the kind labels.
// ---------------------------------------------------------------------------------------------

const VALID_TASK: SeedRecord = {
  id: "seed-0001",
  kind: "task",
  request: {
    kind: "task",
    item: {
      id: "seed-item-0001",
      title: "Problem set 3",
      source_uid: "zybooks:cs-101:ps3",
      created_by: "zybooks",
      course: "cs-101",
      due: "2026-10-01",
    },
    heuristics_seed: {
      course: "cs-101",
      effort_hours: null,
      slice_hours: 1.5,
      weights: "Homework 20%\nExams 50%",
      preferences: "",
      known_courses: ["cs-101"],
    },
  },
  theirs: { effort_hours: 2.5, importance: 4, course: "cs-101" },
};

const VALID_EVENT: SeedRecord = {
  id: "seed-0002",
  kind: "event",
  request: {
    kind: "event",
    item: {
      uid: "seed-event-0002",
      title: "Fall involvement fair",
      start: "2026-10-05T10:00",
      end: "2026-10-05T14:00",
      source: "campus-calendar",
      location: "Student union",
      categories: ["involvement"],
      audiences: ["undergraduate"],
      series_uid: null,
    },
    heuristics_seed: { interests: "" },
  },
  theirs: { verdict: "opportunity" },
};

const VALID_EMAIL: SeedRecord = {
  id: "seed-0003",
  kind: "email",
  request: {
    kind: "email",
    item: {
      message_id: "seed-msg-0003",
      subject: "Assignment reminder",
      date: "2026-10-02",
    },
    heuristics_seed: { known_courses: ["cs-101"] },
  },
  theirs: { tier: "task", title: "Problem set 3", course: "cs-101" },
};

Deno.test("a valid record of each kind passes validateSeedRecord and scrubViolations", () => {
  for (const record of [VALID_TASK, VALID_EVENT, VALID_EMAIL]) {
    assertEquals(validateSeedRecord(record), [], `${record.kind} should validate clean`);
    assertEquals(scrubViolations(record), [], `${record.kind} should scrub clean`);
  }
});

// ---------------------------------------------------------------------------------------------
// Three records that each break one structural rule.
// ---------------------------------------------------------------------------------------------

Deno.test("a fifth top-level key fails validateSeedRecord, naming the extra key", () => {
  const withExtra = { ...VALID_TASK, note: "not one of the four keys" } as unknown;
  const violations = validateSeedRecord(withExtra);
  assert(violations.length > 0, "expected a violation");
  assert(violations.some((v) => v.includes("top-level keys")), `no top-level-key violation in ${violations}`);
});

Deno.test("an 'email' key in request.item fails validateSeedRecord, naming the key", () => {
  const withEmailKey: SeedRecord = {
    ...VALID_TASK,
    request: { ...VALID_TASK.request, item: { ...VALID_TASK.request.item, email: "not-a-real-one" } },
  };
  const violations = validateSeedRecord(withEmailKey);
  assert(violations.length > 0, "expected a violation");
  assert(violations.some((v) => v.includes("'email'")), `no 'email' violation in ${violations}`);
});

Deno.test("a 'theirs' field outside the kind's labelled set fails validateSeedRecord, naming it", () => {
  const withUnlabelled: SeedRecord = {
    ...VALID_EVENT,
    theirs: { verdict: "opportunity", why: "free text a label was never supposed to carry" },
  };
  const violations = validateSeedRecord(withUnlabelled);
  assert(violations.length > 0, "expected a violation");
  assert(violations.some((v) => v.includes("'why'")), `no 'why' violation in ${violations}`);
});

// ---------------------------------------------------------------------------------------------
// scrubViolations: an otherwise schema-valid record that leaks something the scrub rules exist to
// catch. Each is checked in isolation so a single fixture failing two rules at once can't hide
// which rule this test is actually proving.
// ---------------------------------------------------------------------------------------------

Deno.test("scrubViolations catches an '@' token anywhere in request or theirs", () => {
  const leaky: SeedRecord = {
    ...VALID_TASK,
    request: {
      ...VALID_TASK.request,
      item: { ...VALID_TASK.request.item, title: "Reach student@example.edu about ps3" },
    },
  };
  assertEquals(validateSeedRecord(leaky), [], "this record is still schema-valid");
  const violations = scrubViolations(leaky);
  assert(violations.some((v) => v.includes("@")), `no '@' violation in ${violations}`);
});

Deno.test("scrubViolations catches a URL anywhere in request or theirs", () => {
  const leaky: SeedRecord = {
    ...VALID_EVENT,
    request: {
      ...VALID_EVENT.request,
      item: { ...VALID_EVENT.request.item, title: "Details at https://example.edu/fair" },
    },
  };
  assertEquals(validateSeedRecord(leaky), [], "this record is still schema-valid");
  const violations = scrubViolations(leaky);
  assert(violations.some((v) => v.includes("URL")), `no URL violation in ${violations}`);
});

Deno.test("scrubViolations catches a string field over 200 characters", () => {
  const leaky: SeedRecord = {
    ...VALID_TASK,
    theirs: { ...VALID_TASK.theirs, course: "c".repeat(201) },
  };
  const violations = scrubViolations(leaky);
  assert(violations.some((v) => v.includes("200 characters")), `no length violation in ${violations}`);
});

Deno.test("scrubViolations catches a banned key anywhere in request.item", () => {
  const leaky: SeedRecord = {
    ...VALID_EMAIL,
    request: {
      ...VALID_EMAIL.request,
      item: { ...VALID_EMAIL.request.item, from: "registrar@example.edu" },
    },
  };
  const violations = scrubViolations(leaky);
  assert(violations.some((v) => v.includes("'from'")), `no banned-key violation in ${violations}`);
});

Deno.test("scrubViolations catches a Windows path and a POSIX home path", () => {
  const winPath: SeedRecord = {
    ...VALID_TASK,
    request: {
      ...VALID_TASK.request,
      item: { ...VALID_TASK.request.item, title: "See C:\\Users\\student\\notes.txt" },
    },
  };
  assert(scrubViolations(winPath).some((v) => v.includes("Windows path")));

  const posixPath: SeedRecord = {
    ...VALID_TASK,
    request: {
      ...VALID_TASK.request,
      item: { ...VALID_TASK.request.item, title: "See /Users/student/notes.txt" },
    },
  };
  assert(scrubViolations(posixPath).some((v) => v.includes("POSIX home path")));
});

Deno.test("scrubViolations catches a long number", () => {
  const leaky: SeedRecord = {
    ...VALID_TASK,
    request: {
      ...VALID_TASK.request,
      item: { ...VALID_TASK.request.item, title: "Confirmation 12345678" },
    },
  };
  assert(scrubViolations(leaky).some((v) => v.includes("long number")));
});

// ---------------------------------------------------------------------------------------------
// loadSeed: an absent or empty directory is the merge-time state and never an error; an invalid
// line throws naming the file and the line number.
// ---------------------------------------------------------------------------------------------

// `Deno.makeTempDir`'s `dir` option and return value are plain path strings, not `file:` URLs, so
// these two convert between the two — Windows drive letters (`C:\...`) included — without pulling
// in `@std/path` for a job this small.
function fromFileUrl(url: URL): string {
  const decoded = decodeURIComponent(url.pathname);
  return /^\/[A-Za-z]:\//.test(decoded) ? decoded.slice(1) : decoded;
}

function toFileUrl(path: string): URL {
  const posix = path.replace(/\\/g, "/");
  return new URL(/^[A-Za-z]:\//.test(posix) ? `file:///${posix}` : `file://${posix}`);
}

async function withTempDir(fn: (dir: URL) => Promise<void>): Promise<void> {
  // Deno.makeTempDir, per the task's test list — landed under `cloud/eval/` itself so the CI and
  // focused-test `--allow-write` grant can be scoped to this directory rather than an unportable
  // OS temp root (`cloud/eval/.gitignore` keeps the resulting `.tmp-*` dirs out of git).
  const here = fromFileUrl(new URL("./", import.meta.url));
  const path = await Deno.makeTempDir({ dir: here, prefix: ".tmp-" });
  try {
    await fn(toFileUrl(path.endsWith("/") || path.endsWith("\\") ? path : path + "/"));
  } finally {
    await Deno.remove(path, { recursive: true });
  }
}

Deno.test("loadSeed on a temp directory with no *.jsonl files returns []", async () => {
  await withTempDir(async (dir) => {
    assertEquals(await loadSeed(dir), []);
  });
});

Deno.test("loadSeed on a missing directory returns []", async () => {
  const missing = new URL("./.tmp-does-not-exist-9f2c1a/", import.meta.url);
  assertEquals(await loadSeed(missing), []);
});

Deno.test("loadSeed on a temp directory with one valid line returns one record", async () => {
  await withTempDir(async (dir) => {
    await Deno.writeTextFile(new URL("tasks.jsonl", dir), JSON.stringify(VALID_TASK) + "\n");
    const records = await loadSeed(dir);
    assertEquals(records.length, 1);
    assertEquals(records[0].id, "seed-0001");
  });
});

Deno.test("loadSeed on a file with one invalid line throws naming the file and line 1", async () => {
  await withTempDir(async (dir) => {
    await Deno.writeTextFile(new URL("bad.jsonl", dir), JSON.stringify({ not: "a seed record" }) + "\n");
    let threw: unknown;
    try {
      await loadSeed(dir);
    } catch (e) {
      threw = e;
    }
    assert(threw instanceof Error, "loadSeed should throw on an invalid line");
    assert((threw as Error).message.includes("bad.jsonl"), `message should name the file: ${threw}`);
    assert(
      (threw as Error).message.includes(":1:") || (threw as Error).message.includes("line 1"),
      `message should name line 1: ${threw}`,
    );
  });
});
