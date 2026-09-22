import { assert, assertEquals } from "@std/assert";
import { handle } from "./handler.ts";

const DEVICE = "0123456789abcdef";
const REC = (n: number) =>
  `{"actor":"quinn","device":"LAPTOP","id":"task_000000000${n}","op":"set","path":"tasks/x.md","ts":"2026-09-17T10:0${n}:00.000Z","via":"dashboard"}`;

import { sha256Hex } from "../_shared/sync_rows.ts";   // the one hash, shared with the handler

const OK = () => Promise.resolve({ account_id: "acct-1" });

function deps(over: Partial<Parameters<typeof handle>[1]> = {}) {
  return {
    requireEntitled: OK,
    bytesUsed: () => Promise.resolve(0),
    ceiling: () => Promise.resolve(1_000_000),
    saveRecords: () => Promise.resolve(),
    saveNotes: () => Promise.resolve(),
    ...over,
  };
}

function push(body: Record<string, unknown>): Request {
  return new Request("http://127.0.0.1/sync-push", {
    method: "POST",
    headers: { authorization: "Bearer t", "content-type": "application/json" },
    body: JSON.stringify(body),
  });
}

/** **Every refusal in this codebase is a THROWN `Response`** — `_shared/http.ts`'s `fail` throws and
 * `requireActiveEntitlement` rejects, and `index.ts`'s `asResponse` is what turns either into the
 * reply. A handler test therefore awaits the rejection, exactly as C1's
 * `telemetry/handler_test.ts:79` does; a bare `await handle(…)` would reject the test instead of
 * failing an assertion. */
function refusal(req: Request, d: Parameters<typeof handle>[1]): Promise<Response> {
  return handle(req, d).catch((e) => e as Response);
}

Deno.test("a batch of records and notes is stored and counted", async () => {
  let records: unknown[] = [];
  let notes: unknown[] = [];
  const res = await handle(
    push({
      device: DEVICE,
      records: [{ hash: await sha256Hex(REC(1)), body: REC(1) }, { hash: await sha256Hex(REC(2)), body: REC(2) }],
      notes: [{ path: "tasks/x.md", body: "---\nid: task_0000000001\n---\n" }, { path: "tasks/y.md", deleted: true }],
    }),
    deps({
      saveRecords: (r) => { records = r; return Promise.resolve(); },
      saveNotes: (n) => { notes = n; return Promise.resolve(); },
    }),
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { records: 2, notes: 2, bytes_used: 0, bytes_ceiling: 1_000_000 });
  assertEquals(records.length, 2);
  assertEquals((records[0] as Record<string, unknown>).account_id, "acct-1");
  assertEquals((notes[1] as Record<string, unknown>).deleted, true);
});

Deno.test("a record whose hash is not its body's is a 400, and nothing is stored", async () => {
  // **The check the sealed design could not make.** A hash the client asserts is an idempotence key
  // a client can aim: two different records under one hash, and the second silently never lands.
  // The server can read the body now, so it computes the key itself.
  let stored = 0;
  const res = await refusal(
    push({ device: DEVICE, records: [{ hash: await sha256Hex(REC(1)), body: REC(2) }] }),
    deps({ saveRecords: () => { stored += 1; return Promise.resolve(); } }),
  );
  assertEquals(res.status, 400);
  assert((await res.json()).error.includes("hash"), "the message names what is wrong");
  assertEquals(stored, 0);
});

Deno.test("a duplicate row inside one batch becomes one write", async () => {
  // `sync_records_once` catches it across retries; this catches it within a batch, before the
  // upsert — PostgREST refuses a batch that names one conflict target twice, so this is not an
  // optimisation, it is what keeps a legitimate double-send from becoming a 400.
  let records: unknown[] = [];
  const h = await sha256Hex(REC(1));
  const res = await handle(
    push({ device: DEVICE, records: [{ hash: h, body: REC(1) }, { hash: h, body: REC(1) }] }),
    deps({ saveRecords: (r) => { records = r; return Promise.resolve(); } }),
  );
  assertEquals(res.status, 200);
  assertEquals((await res.json()).records, 1);
  assertEquals(records.length, 1);
});

Deno.test("a row with a plaintext field beside the body is a 400 that names the field", async () => {
  const res = await refusal(
    push({ device: DEVICE, notes: [{ path: "tasks/x.md", body: "x", title: "CS 100 HW 1" }] }),
    deps(),
  );
  assertEquals(res.status, 400);
  assert((await res.json()).error.includes("title"));
});

Deno.test("a device token that is not a device token is a 400", async () => {
  assertEquals((await refusal(push({ device: "nope", records: [] }), deps())).status, 400);
  assertEquals((await refusal(push({ records: [] }), deps())).status, 400);
});

Deno.test("a batch that would pass the ceiling is a 413 and stores nothing", async () => {
  let stored = 0;
  const res = await refusal(
    push({ device: DEVICE, records: [{ hash: await sha256Hex(REC(1)), body: REC(1) }] }),
    deps({
      bytesUsed: () => Promise.resolve(999_999),
      ceiling: () => Promise.resolve(1_000_000),
      saveRecords: () => { stored += 1; return Promise.resolve(); },
    }),
  );
  assertEquals(res.status, 413);
  assertEquals(stored, 0);
});

Deno.test("more than MAX_ROWS of either kind is a 400, not a silent truncation", async () => {
  const many = Array.from({ length: 501 }, (_, i) => ({ path: `tasks/x${i}.md`, deleted: true }));
  assertEquals((await refusal(push({ device: DEVICE, notes: many }), deps())).status, 400);
});

Deno.test("no entitlement and no session are the gate's, thrown, and nothing is stored", async () => {
  for (const status of [401, 402]) {
    let stored = 0;
    const res = await refusal(
      push({ device: DEVICE, records: [] }),
      deps({
        requireEntitled: () => Promise.reject(new Response(JSON.stringify({ error: "x" }), { status })),
        saveRecords: () => { stored += 1; return Promise.resolve(); },
      }),
    );
    assertEquals(res.status, status);
    assertEquals(stored, 0);
  }
});

Deno.test("the wrong method is a 405", async () => {
  const res = await handle(new Request("http://127.0.0.1/sync-push", { method: "GET" }), deps());
  assertEquals(res.status, 405);
});
