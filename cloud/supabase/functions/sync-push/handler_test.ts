import { assert, assertEquals } from "@std/assert";
import { handle } from "./handler.ts";

const DEVICE = "0123456789abcdef";
const REC = (n: number) =>
  `{"actor":"student","device":"LAPTOP","id":"task_000000000${n}","op":"set","path":"tasks/x.md","ts":"2026-09-17T10:0${n}:00.000Z","via":"dashboard"}`;

import { MAX_PUSH_BYTES, sha256Hex } from "../_shared/sync_rows.ts";   // the one hash, shared with the handler

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

Deno.test("a batch that would pass the ceiling is a 403 and stores nothing", async () => {
  // 403, not 413 (R-C3′-exec-7): 413 is `readJson`'s own refusal for a request body over
  // `MAX_PUSH_BYTES`, thrown before a row is ever parsed. This is the account's ceiling instead —
  // a reason the device must tell apart from a request that was simply too big to read.
  let stored = 0;
  const res = await refusal(
    push({ device: DEVICE, records: [{ hash: await sha256Hex(REC(1)), body: REC(1) }] }),
    deps({
      bytesUsed: () => Promise.resolve(999_999),
      ceiling: () => Promise.resolve(1_000_000),
      saveRecords: () => { stored += 1; return Promise.resolve(); },
    }),
  );
  assertEquals(res.status, 403);
  assertEquals(stored, 0);
});

Deno.test("an over-ceiling account can still push a batch that adds no net bytes", async () => {
  // R-C3′-exec-9 m1: `used` already over `cap` — a concurrent push from a second desktop, or a
  // lowered ceiling — must not trap the account with no push that can ever succeed. A tombstone
  // carries no bytes, so a tombstone-only batch adds 0 and must go through.
  const res = await handle(
    push({ device: DEVICE, notes: [{ path: "tasks/x.md", deleted: true }] }),
    deps({ bytesUsed: () => Promise.resolve(2_000_000), ceiling: () => Promise.resolve(1_000_000) }),
  );
  assertEquals(res.status, 200);
  assertEquals((await res.json()).notes, 1);
});

Deno.test("a valid batch whose JSON is over 1 MiB and under MAX_PUSH_BYTES is accepted", async () => {
  // The brief-mandated default `readJson` cap is 1 MiB of characters; five hundred notes at a
  // realistic size clear it easily and must not be refused as if the account were full.
  const notes = Array.from({ length: 500 }, (_, i) => ({ path: `tasks/x${i}.md`, body: "x".repeat(4000) }));
  const text = JSON.stringify({ device: DEVICE, notes });
  assert(text.length > 1 << 20, "the test vector must exceed the old 1 MiB default to be a test");
  assert(text.length < MAX_PUSH_BYTES, "the test vector must stay under the new transport cap");
  let saved = 0;
  const res = await handle(
    new Request("http://127.0.0.1/sync-push", {
      method: "POST",
      headers: { authorization: "Bearer t", "content-type": "application/json" },
      body: text,
    }),
    deps({
      ceiling: () => Promise.resolve(100_000_000),
      saveNotes: (n) => { saved = n.length; return Promise.resolve(); },
    }),
  );
  assertEquals(res.status, 200);
  assertEquals(saved, 500);
});

Deno.test("a request body over MAX_PUSH_BYTES is a 413 and stores nothing", async () => {
  const text = JSON.stringify({ device: DEVICE, pad: "x".repeat(MAX_PUSH_BYTES + 8) });
  let stored = 0;
  const res = await refusal(
    new Request("http://127.0.0.1/sync-push", {
      method: "POST",
      headers: { authorization: "Bearer t", "content-type": "application/json" },
      body: text,
    }),
    deps({ saveRecords: () => { stored += 1; return Promise.resolve(); } }),
  );
  assertEquals(res.status, 413);
  assertEquals(stored, 0);
});

Deno.test("a push body that is not a JSON object is a 400, not a bare 500", async () => {
  for (const text of ["null", "[]", '"x"', "42"]) {
    const res = await refusal(
      new Request("http://127.0.0.1/sync-push", {
        method: "POST",
        headers: { authorization: "Bearer t", "content-type": "application/json" },
        body: text,
      }),
      deps(),
    );
    assertEquals(res.status, 400, `${text} was accepted`);
  }
});

Deno.test("valid records beside one bad note is all-or-nothing: saveRecords is never called", async () => {
  let recordsCalled = 0;
  const res = await refusal(
    push({
      device: DEVICE,
      records: [{ hash: await sha256Hex(REC(1)), body: REC(1) }],
      notes: [{ path: "tasks/x.md", body: "x", title: "nope" }],
    }),
    deps({ saveRecords: () => { recordsCalled += 1; return Promise.resolve(); } }),
  );
  assertEquals(res.status, 400);
  assertEquals(recordsCalled, 0);
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
