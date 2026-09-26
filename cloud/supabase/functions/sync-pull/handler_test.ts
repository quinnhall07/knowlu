import { assertEquals } from "@std/assert";
import { handle, MAX_PAGE } from "./handler.ts";

const OK = () => Promise.resolve({ account_id: "acct-1" });
const rec = (seq: number) => ({ seq, device: "0123456789abcdef", record_hash: "a".repeat(64), body: '{"actor":"student","op":"set"}' });
const note = (rev: number) => ({ rev, device: "0123456789abcdef", path: `tasks/x${rev}.md`, deleted: false, body: "---\n---\n" });

/** A fixed clock, so the ten-second read lag is a value a test can see rather than a race. */
const NOW = new Date("2026-09-17T12:00:00.000Z");

function pull(query: string) {
  return new Request(`http://127.0.0.1/sync-pull${query}`, { headers: { authorization: "Bearer t" } });
}

function refusal(req: Request, d: Parameters<typeof handle>[1]): Promise<Response> {
  return handle(req, d).catch((e) => e as Response);
}

Deno.test("a page comes back in cursor order with the cursor it ends on", async () => {
  const res = await handle(pull("?records_after=0&notes_after=0"), {
    requireEntitled: OK,
    readRecords: () => Promise.resolve([rec(7), rec(9)]),
    readNotes: () => Promise.resolve([note(3)]),
    now: () => NOW,
  });
  assertEquals(res.status, 200);
  const body = await res.json();
  assertEquals(body.records.map((r: { seq: number }) => r.seq), [7, 9]);
  assertEquals(body.record_cursor, 9);
  assertEquals(body.note_cursor, 3);
  assertEquals(body.more, false);
});

Deno.test("an empty page holds the cursors where they were", async () => {
  // The device must not rewind. A page with nothing in it answers with the cursor it was given, so a
  // slot that finds nothing new advances nothing and asks the same question next time.
  const res = await handle(pull("?records_after=41&notes_after=17"), {
    requireEntitled: OK, readRecords: () => Promise.resolve([]), readNotes: () => Promise.resolve([]), now: () => NOW,
  });
  const body = await res.json();
  assertEquals([body.record_cursor, body.note_cursor, body.more], [41, 17, false]);
});

Deno.test("a full page says there is more", async () => {
  const many = Array.from({ length: MAX_PAGE }, (_, i) => rec(i + 1));
  const res = await handle(pull(`?records_after=0&notes_after=0&limit=${MAX_PAGE}`), {
    requireEntitled: OK, readRecords: () => Promise.resolve(many), readNotes: () => Promise.resolve([]), now: () => NOW,
  });
  const body = await res.json();
  assertEquals(body.more, true);
  assertEquals(body.record_cursor, MAX_PAGE);
});

Deno.test("the caller's limit is clamped, never trusted", async () => {
  // PostgREST caps a select at `max_rows` (1000) whether or not the caller asked, so a limit above
  // the cap would silently produce a short page that reads as "no more" and strand every row past it.
  for (const q of ["?limit=99999", "?limit=0", "?limit=-3", "?limit=lots"]) {
    let asked = -1;
    await handle(pull(q), {
      requireEntitled: OK,
      readRecords: (_a, _b, limit) => { asked = limit; return Promise.resolve([]); },
      readNotes: () => Promise.resolve([]),
      now: () => NOW,
    });
    assertEquals(asked, MAX_PAGE, q);
  }
});

Deno.test("a cursor that is not a number starts from the beginning rather than erroring", async () => {
  let asked = -1;
  await handle(pull("?records_after=yesterday"), {
    requireEntitled: OK,
    readRecords: (_a, after) => { asked = after; return Promise.resolve([]); },
    readNotes: () => Promise.resolve([]),
    now: () => NOW,
  });
  assertEquals(asked, 0);
});

Deno.test("a restore is this endpoint from zero, and nothing else", async () => {
  // Exit gate item 7: there is no third function. A desktop with an empty vault asks the same
  // question with `records_after=0` and pages to the end.
  let asked = -1;
  await handle(pull("?records_after=0&notes_after=0"), {
    requireEntitled: OK,
    readRecords: (_a, after) => { asked = after; return Promise.resolve([]); },
    readNotes: () => Promise.resolve([]),
    now: () => NOW,
  });
  assertEquals(asked, 0);
});

Deno.test("the clock the read lag uses is the handler's, and it reaches both readers", async () => {
  // I3, inherited: both reads must share one window, or a note can be returned from behind a lag a
  // record was already read past.
  const seen: Date[] = [];
  await handle(pull("?records_after=0"), {
    requireEntitled: OK,
    readRecords: (_a, _b, _c, now) => { seen.push(now); return Promise.resolve([]); },
    readNotes: (_a, _b, _c, now) => { seen.push(now); return Promise.resolve([]); },
    now: () => NOW,
  });
  assertEquals(seen.length, 2);
  assertEquals(seen[0].getTime(), NOW.getTime());
  assertEquals(seen[1].getTime(), NOW.getTime());
});

Deno.test("no entitlement and no session are the gate's, thrown, and nothing is read", async () => {
  for (const status of [401, 402]) {
    let read = 0;
    const res = await refusal(pull("?records_after=0"), {
      requireEntitled: () => Promise.reject(new Response(JSON.stringify({ error: "x" }), { status })),
      readRecords: () => { read += 1; return Promise.resolve([]); },
      readNotes: () => { read += 1; return Promise.resolve([]); },
      now: () => NOW,
    });
    assertEquals(res.status, status);
    assertEquals(read, 0);
  }
});

Deno.test("the wrong method is a 405", async () => {
  const res = await handle(new Request("http://127.0.0.1/sync-pull", { method: "POST" }), {
    requireEntitled: OK, readRecords: () => Promise.resolve([]), readNotes: () => Promise.resolve([]), now: () => NOW,
  });
  assertEquals(res.status, 405);
});
