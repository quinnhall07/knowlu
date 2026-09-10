import { assert, assertEquals } from "@std/assert";
import { ACTIONS, handle, VALUED_FIELDS } from "./handler.ts";

const req = (body: unknown, auth = "Bearer good") =>
  new Request("http://127.0.0.1:1/telemetry", {
    method: "POST",
    headers: { authorization: auth },
    body: JSON.stringify(body),
  });

const base = {
  verify: (t: string) => Promise.resolve(t === "good" ? { id: "acc-1", email: null } : null),
  saveEvents: () => Promise.resolve(),
  saveCorrections: () => Promise.resolve(),
};

Deno.test("the action vocabulary is the engine's eleven, in the engine's order", () => {
  assertEquals(ACTIONS, [
    "view_opened",
    "object_seen",
    "edit_started",
    "edit_committed",
    "edit_cancelled",
    "decision_made",
    "decision_deferred",
    "issue_opened",
    "sync_run",
    "delta_expanded",
    "why_expanded",
  ]);
});

Deno.test("a good batch is accepted and counted", async () => {
  let events = 0, corrections = 0;
  const res = await handle(
    req({
      events: [{
        ts: "2026-09-10T12:00:00.000Z",
        session: "sess_1",
        view: "today",
        action: "object_seen",
        object_id: "task_0123456789",
        object_kind: "task",
        ms: 2400,
      }],
      corrections: [{
        ts: "2026-09-10T12:01:00.000Z",
        item_id: "task_0123456789",
        field: "effort_hours",
        ours: "2.0",
        theirs: "0.5",
        kind: "task",
      }],
    }),
    {
      ...base,
      saveEvents: (rows) => {
        events = rows.length;
        return Promise.resolve();
      },
      saveCorrections: (rows) => {
        corrections = rows.length;
        return Promise.resolve();
      },
    },
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { events: 1, corrections: 1 });
  assertEquals([events, corrections], [1, 1]);
});

Deno.test("an unknown action is refused by name, and the whole batch with it", async () => {
  const res = await handle(
    req({
      events: [{ ts: "2026-09-10T12:00:00.000Z", session: "s", view: "today", action: "typed_a_title" }],
      corrections: [],
    }),
    base,
  ).catch((e) => e as Response);
  assertEquals(res.status, 400);
  assert(String((await res.json()).error).includes("typed_a_title"));
});

Deno.test("free text is refused wherever it could hide", async () => {
  const bad = [
    {
      ts: "2026-09-10T12:00:00.000Z",
      session: "s",
      view: "today",
      action: "object_seen",
      object_id: "Read chapter 3",
    },
    { ts: "2026-09-10T12:00:00.000Z", session: "s", view: "Calculus II — Today", action: "view_opened" },
    {
      ts: "2026-09-10T12:00:00.000Z",
      session: "s",
      view: "today",
      action: "object_seen",
      object_kind: "a task about the midterm",
    },
  ];
  for (const e of bad) {
    const res = await handle(req({ events: [e], corrections: [] }), base).catch((x) => x as Response);
    assertEquals(res.status, 400, JSON.stringify(e));
  }
});

Deno.test("a correction on a content field carries no value — spec §6's rule, enforced here too", async () => {
  let saved: Record<string, unknown>[] = [];
  const res = await handle(
    req({
      events: [],
      corrections: [
        {
          ts: "2026-09-10T12:00:00.000Z",
          item_id: "task_0123456789",
          field: "course",
          ours: "MATH 125",
          theirs: "SPAN 101",
          kind: "task",
        },
      ],
    }),
    {
      ...base,
      saveCorrections: (rows) => {
        saved = rows as Record<string, unknown>[];
        return Promise.resolve();
      },
    },
  );
  assertEquals(res.status, 200);
  // The row is kept — that a course was corrected is the signal — but the two names are not.
  assertEquals(saved[0].ours, null);
  assertEquals(saved[0].theirs, null);
  assert(!VALUED_FIELDS.includes("course"));
  assert(!VALUED_FIELDS.includes("title"));
});

// Fix round 1 (C1, ruling R-C1-39): `domain`, `effort_confidence` and `status` are VALUED_FIELDS
// but free-text inputs in the console with no vocabulary check anywhere in the write path — so
// `ours`/`theirs` can be a sentence even though the device is supposed to drop it first. The server
// mirrors, never 400s: the row survives, the sentence is nulled.
Deno.test("a free-text value on a VALUED field is nulled, not refused — the row still saves", async () => {
  let saved: Record<string, unknown>[] = [];
  const res = await handle(
    req({
      events: [],
      corrections: [
        {
          ts: "2026-09-10T12:00:00.000Z",
          item_id: "task_0123456789",
          field: "effort_confidence",
          ours: "low",
          theirs: "I honestly have no idea how long this will take",
          kind: "task",
        },
      ],
    }),
    {
      ...base,
      saveCorrections: (rows) => {
        saved = rows as Record<string, unknown>[];
        return Promise.resolve();
      },
    },
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { events: 0, corrections: 1 });
  // A closed-vocabulary value still travels…
  assertEquals(saved[0].ours, "low");
  // …a sentence never does, even on a field the client-side EventIn/CorrectionIn contract calls
  // VALUED rather than FLAGGED.
  assertEquals(saved[0].theirs, null);
});

Deno.test("a correction on a field that is neither valued nor flagged is refused", async () => {
  const res = await handle(
    req({
      events: [],
      corrections: [
        {
          ts: "2026-09-10T12:00:00.000Z",
          item_id: "task_1",
          field: "title",
          ours: "a",
          theirs: "b",
          kind: "task",
        },
      ],
    }),
    base,
  ).catch((e) => e as Response);
  assertEquals(res.status, 400);
});

Deno.test("an oversized batch is refused rather than truncated", async () => {
  const one = { ts: "2026-09-10T12:00:00.000Z", session: "s", view: "today", action: "view_opened" };
  const res = await handle(req({ events: new Array(501).fill(one), corrections: [] }), base).catch((e) =>
    e as Response
  );
  assertEquals(res.status, 400);
});

// Fix round 1, item 1: the class-(c) opt-in cannot come from the request body. C1 builds no
// opt-in, so `request` is always written null — `opt_in_raw: true` and a populated `request`
// object must not change that, and the extra fields must never reach `saveCorrections`.
Deno.test("a correction's request is never stored from the client, even with opt_in_raw: true", async () => {
  let saved: Record<string, unknown>[] = [];
  const res = await handle(
    req({
      events: [],
      corrections: [
        {
          ts: "2026-09-10T12:00:00.000Z",
          item_id: "task_1",
          field: "effort_hours",
          ours: "2.0",
          theirs: "3.0",
          kind: "task",
          request: { title: "Read chapter 3", body: "do the homework" },
        },
      ],
      opt_in_raw: true,
    }),
    {
      ...base,
      saveCorrections: (rows) => {
        saved = rows as Record<string, unknown>[];
        return Promise.resolve();
      },
    },
  );
  assertEquals(res.status, 200);
  assertEquals(saved[0].request, null);
  assert(!("title" in saved[0]));
});

// Fix round 1, item 2: a timestamp must look like ISO-8601 before Date.parse is trusted — V8
// parses both of these loosely today, which is exactly the free-text hiding spot this endpoint
// exists to close.
Deno.test("a timestamp must look like ISO-8601, not just something Date.parse happens to accept", async () => {
  const bad = [
    { ts: "Read chapter 3", session: "s", view: "today", action: "view_opened" },
    { ts: "2026-09-10 (Read chapter 3)", session: "s", view: "today", action: "view_opened" },
  ];
  for (const e of bad) {
    const res = await handle(req({ events: [e], corrections: [] }), base).catch((x) => x as Response);
    assertEquals(res.status, 400, JSON.stringify(e));
  }
});

// Fix round 1, item 3: `events` and `corrections` must each be an array when present.
Deno.test("events must be an array when present", async () => {
  const res = await handle(req({ events: { a: 1 } }), base).catch((e) => e as Response);
  assertEquals(res.status, 400);
});

Deno.test("corrections must be an array when present", async () => {
  const res = await handle(req({ corrections: { a: 1 } }), base).catch((e) => e as Response);
  assertEquals(res.status, 400);
});

// Fix round 1, item 4: `ms` must be a sane, non-negative duration — at most one day.
Deno.test("ms outside a sane range is refused", async () => {
  const res = await handle(
    req({
      events: [{
        ts: "2026-09-10T12:00:00.000Z",
        session: "s",
        view: "today",
        action: "view_opened",
        ms: 1e18,
      }],
      corrections: [],
    }),
    base,
  ).catch((e) => e as Response);
  assertEquals(res.status, 400);
});

// Fix round 1, item 5: within-batch dedup on the migration's own unique-constraint tuples,
// last occurrence wins.
Deno.test("two identical events in one batch reach saveEvents as one row", async () => {
  let saved: unknown[] = [];
  const one = { ts: "2026-09-10T12:00:00.000Z", session: "s", view: "today", action: "view_opened" };
  const res = await handle(
    req({ events: [one, one], corrections: [] }),
    {
      ...base,
      saveEvents: (rows) => {
        saved = rows;
        return Promise.resolve();
      },
    },
  );
  assertEquals(res.status, 200);
  assertEquals(saved.length, 1);
  assertEquals((await res.json()).events, 1);
});

Deno.test("two identical corrections in one batch reach saveCorrections as one row", async () => {
  let saved: unknown[] = [];
  const one = {
    ts: "2026-09-10T12:00:00.000Z",
    item_id: "task_1",
    field: "effort_hours",
    ours: "2.0",
    theirs: "3.0",
    kind: "task",
  };
  const res = await handle(
    req({ events: [], corrections: [one, one] }),
    {
      ...base,
      saveCorrections: (rows) => {
        saved = rows;
        return Promise.resolve();
      },
    },
  );
  assertEquals(res.status, 200);
  assertEquals(saved.length, 1);
  assertEquals((await res.json()).corrections, 1);
});

// Fix round 1, item 7: three pins.
Deno.test("no bearer token is 401", async () => {
  const res = await handle(req({ events: [], corrections: [] }, ""), base).catch((e) => e as Response);
  assertEquals(res.status, 401);
  assertEquals(await res.json(), { error: "no bearer token" });
});

Deno.test("GET is 405 with an exact allow header", async () => {
  const res = await handle(new Request("http://127.0.0.1:1/telemetry", { method: "GET" }), base);
  assertEquals(res.status, 405);
  assertEquals(res.headers.get("allow"), "POST");
});

Deno.test("an event batch carrying extra fields reaches saveEvents with exactly the schema's columns", async () => {
  let saved: Record<string, unknown>[] = [];
  const res = await handle(
    req({
      events: [{
        ts: "2026-09-10T12:00:00.000Z",
        session: "s",
        view: "today",
        action: "view_opened",
        title: "Read chapter 3",
        note_body: "do the homework",
      }],
      corrections: [],
    }),
    {
      ...base,
      saveEvents: (rows) => {
        saved = rows as Record<string, unknown>[];
        return Promise.resolve();
      },
    },
  );
  assertEquals(res.status, 200);
  assertEquals(
    Object.keys(saved[0]).sort(),
    ["account_id", "action", "ms", "object_id", "object_kind", "session", "ts", "view"].sort(),
  );
});
