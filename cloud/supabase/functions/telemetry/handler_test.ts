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
