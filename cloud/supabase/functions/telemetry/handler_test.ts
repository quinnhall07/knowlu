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
  // F7: a batch with no id-bearing row never asks; a test that expects the lookup overrides this.
  ownedJudgments: (): Promise<Map<string, string>> =>
    Promise.reject(new Error("unexpected ownership lookup")),
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

// ---- F7: id-bearing label rows (engine follow-ups plan, item (b)) ----

const J1 = "11111111-1111-4111-8111-111111111111";
const J2 = "22222222-2222-4222-8222-222222222222";

/** The kind each test judgment really has on the server (`judgments.kind`). */
const KINDS: Record<string, string> = { [J1]: "event", [J2]: "task" };

/** `ids`, each owned with its real kind from `KINDS`: what `index.ts`'s lookup returns. */
const ownedAs = (ids: string[], kinds: Record<string, string> = KINDS) =>
  Promise.resolve(new Map(ids.filter((id) => id in kinds).map((id) => [id, kinds[id]])));

/** `base`, plus an `ownedJudgments` that owns every test judgment it is asked about. */
const owning = {
  ...base,
  ownedJudgments: (_account: string, ids: string[]) => ownedAs(ids),
};

const verdictRow = (overrides: Record<string, unknown> = {}) => ({
  ts: "2026-09-23T12:00:00.000Z",
  item_id: "evt_0123456789",
  field: "verdict",
  ours: "unsure",
  theirs: "obligation",
  kind: "event",
  judgment_id: J1,
  judgment_kind: "event",
  ...overrides,
});

const decisionRow = (overrides: Record<string, unknown> = {}) => ({
  ts: "2026-09-23T12:05:00.000Z",
  item_id: "appr_0123456789",
  field: "decision",
  ours: "proposed",
  theirs: "rejected",
  kind: "task",
  judgment_id: J2,
  judgment_kind: "task",
  ...overrides,
});

const plainRow = {
  ts: "2026-09-10T12:01:00.000Z",
  item_id: "task_0123456789",
  field: "effort_hours",
  ours: "2.0",
  theirs: "0.5",
  kind: "task",
};

async function refused(corrections: unknown[], deps: typeof owning = owning): Promise<number> {
  const res = await handle(req({ events: [], corrections }), deps).catch((e) => e as Response);
  return res.status;
}

Deno.test("a verdict label with a judgment_id is saved with it", async () => {
  let saved: Record<string, unknown>[] = [];
  const res = await handle(req({ events: [], corrections: [verdictRow()] }), {
    ...owning,
    saveCorrections: (rows) => {
      saved = rows as Record<string, unknown>[];
      return Promise.resolve();
    },
  });
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { events: 0, corrections: 1, unowned: 0, refused: 0 });
  assertEquals(saved, [{
    account_id: "acc-1",
    ts: "2026-09-23T12:00:00.000Z",
    item_id: "evt_0123456789",
    field: "verdict",
    ours: "unsure",
    theirs: "obligation",
    kind: "event",
    request: null,
    judgment_id: J1,
    judgment_kind: "event",
  }]);
});

Deno.test("a label field without a judgment_id is refused", async () => {
  assertEquals(await refused([verdictRow({ judgment_id: undefined, judgment_kind: undefined })]), 400);
  assertEquals(await refused([decisionRow({ judgment_id: null })]), 400);
});

Deno.test("a verdict outside the event vocabulary is refused", async () => {
  assertEquals(await refused([verdictRow({ theirs: "it is my dentist appointment" })]), 400);
  assertEquals(await refused([verdictRow({ ours: "maybe" })]), 400);
  // A verdict on a task judgment is not a verdict.
  assertEquals(await refused([verdictRow({ judgment_kind: "task" })]), 400);
  // A decision is proposed → approved/rejected, and on a task or event judgment only.
  assertEquals(await refused([decisionRow({ theirs: "later" })]), 400);
  assertEquals(await refused([decisionRow({ ours: "approved" })]), 400);
  assertEquals(await refused([decisionRow({ judgment_kind: "course" })]), 400);
});

Deno.test("a malformed judgment_id is refused", async () => {
  for (const id of ["not-a-uuid", "Read chapter 3", J1 + "0", 42, ""]) {
    assertEquals(await refused([verdictRow({ judgment_id: id })]), 400, String(id));
  }
});

Deno.test("plain rows and id-bearing rows reach saveCorrections in separate calls, and plain rows carry no judgment_id key", async () => {
  const calls: Record<string, unknown>[][] = [];
  const res = await handle(req({ events: [], corrections: [plainRow, verdictRow(), decisionRow()] }), {
    ...owning,
    saveCorrections: (rows) => {
      calls.push(rows as Record<string, unknown>[]);
      return Promise.resolve();
    },
  });
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { events: 0, corrections: 3, unowned: 0, refused: 0 });
  assertEquals(calls.length, 2);
  const plain = calls.find((c) => c.some((r) => r.field === "effort_hours"))!;
  const labels = calls.find((c) => c.some((r) => r.field === "verdict"))!;
  assertEquals(plain.length, 1);
  // A merge-duplicates upsert sets every column the payload names: a plain row that named
  // `judgment_id: null` would erase the nightly backfill's id on a re-send.
  assert(!("judgment_id" in plain[0]));
  assert(!("judgment_kind" in plain[0]));
  assertEquals(labels.map((r) => r.judgment_id).sort(), [J1, J2]);
  assert(labels.every((r) => "judgment_kind" in r));
});

Deno.test("today's correction rows are saved exactly as before", async () => {
  let saved: unknown[] = [];
  let lookups = 0;
  const res = await handle(req({ events: [], corrections: [plainRow] }), {
    ...base,
    ownedJudgments: () => {
      lookups++;
      return Promise.resolve(new Map<string, string>());
    },
    saveCorrections: (rows) => {
      saved = rows;
      return Promise.resolve();
    },
  });
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { events: 0, corrections: 1 });
  assertEquals(saved, [{
    account_id: "acc-1",
    ts: "2026-09-10T12:01:00.000Z",
    item_id: "task_0123456789",
    field: "effort_hours",
    ours: "2.0",
    theirs: "0.5",
    kind: "task",
    request: null,
  }]);
  assertEquals(lookups, 0);
});

Deno.test("an email label row is refused", async () => {
  // Gmail Limited Use: an email-derived decision never reaches the shared calibration/rules path.
  assertEquals(await refused([decisionRow({ judgment_kind: "email", kind: "email" })]), 400);
  assertEquals(await refused([verdictRow({ judgment_kind: "email" })]), 400);
});

Deno.test("a judgment_id on a valued or flagged field is refused", async () => {
  assertEquals(await refused([{ ...plainRow, judgment_id: J1, judgment_kind: "task" }]), 400);
  assertEquals(
    await refused([{
      ...plainRow,
      field: "course",
      ours: null,
      theirs: null,
      judgment_id: J1,
      judgment_kind: "email",
    }]),
    400,
  );
  // judgment_kind alone is the same shape.
  assertEquals(await refused([{ ...plainRow, judgment_kind: "task" }]), 400);
});

Deno.test("a judgment_id the caller does not own is dropped and counted", async () => {
  let saved: Record<string, unknown>[] = [];
  let asked: [string, string[]] | null = null;
  const res = await handle(req({ events: [], corrections: [verdictRow(), decisionRow()] }), {
    ...base,
    ownedJudgments: (account: string, ids: string[]) => {
      asked = [account, [...ids].sort()];
      return ownedAs(ids, { [J1]: "event" });
    },
    saveCorrections: (rows) => {
      saved = rows as Record<string, unknown>[];
      return Promise.resolve();
    },
  });
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { events: 0, corrections: 1, unowned: 1, refused: 0 });
  assertEquals(asked, ["acc-1", [J1, J2]]);
  assertEquals(saved.map((r) => r.judgment_id), [J1]);
});

Deno.test("an ownership lookup failure saves nothing", async () => {
  let saves = 0;
  const res = await handle(req({ events: [plainEvent()], corrections: [plainRow, verdictRow()] }), {
    ...base,
    ownedJudgments: () => Promise.reject(new Error("select judgments: 503")),
    saveEvents: () => {
      saves++;
      return Promise.resolve();
    },
    saveCorrections: () => {
      saves++;
      return Promise.resolve();
    },
  }).catch((e) => e as Error);
  assert(res instanceof Error);
  assertEquals(saves, 0);
});

function plainEvent() {
  return { ts: "2026-09-23T12:00:00.000Z", session: "s", view: "today", action: "view_opened" };
}

// ---- F7 fix round 1 (review I-1, M-1, M-3) ----

/** Runs one batch through `handle` with `ownedJudgments` answering from `kinds`, and returns the
 * response body and every row that reached `saveCorrections`. */
async function run(corrections: unknown[], kinds: Record<string, string>) {
  const saved: Record<string, unknown>[] = [];
  const res = await handle(req({ events: [], corrections }), {
    ...base,
    ownedJudgments: (_account: string, ids: string[]) => ownedAs(ids, kinds),
    saveCorrections: (rows) => {
      saved.push(...rows as Record<string, unknown>[]);
      return Promise.resolve();
    },
  });
  assertEquals(res.status, 200);
  return { body: await res.json(), saved };
}

Deno.test("a label whose judgment is really an email judgment is not saved, whatever kind it claims", async () => {
  // Review I-1: the caller owns J2, but it is an email judgment on the server. Claiming "task"
  // must not carry an email-derived decision into promote_rules (Gmail Limited Use).
  const { body, saved } = await run([decisionRow({ judgment_kind: "task" })], { [J2]: "email" });
  assertEquals(body, { events: 0, corrections: 0, unowned: 1, refused: 0 });
  assertEquals(saved, []);
});

Deno.test("a label whose claimed kind differs from the judgment's real kind is dropped and counted", async () => {
  // J2 is really an event judgment; the row claims "task".
  const { body, saved } = await run([decisionRow({ judgment_kind: "task" })], { [J2]: "event" });
  assertEquals(body, { events: 0, corrections: 0, unowned: 1, refused: 0 });
  assertEquals(saved, []);
});

Deno.test("a saved label carries the server's kind for its judgment", async () => {
  const { saved } = await run([decisionRow({ judgment_kind: "event", kind: "event" })], { [J2]: "event" });
  assertEquals(saved.map((r) => [r.judgment_id, r.judgment_kind]), [[J2, "event"]]);
});

Deno.test("a verdict row whose ours is not unsure is dropped and counted as refused", async () => {
  // Review M-1: the event card exists only for an `unsure` judgment. A verdict row with any other
  // `ours` (or an answer of `unsure` again) would mark a judgment wrong that nobody corrected.
  const { body, saved } = await run(
    [
      verdictRow({ ours: "obligation", theirs: "obligation" }),
      verdictRow({ ts: "2026-09-23T12:01:00.000Z", ours: "drop", theirs: "obligation" }),
      verdictRow({ ts: "2026-09-23T12:02:00.000Z", ours: "unsure", theirs: "unsure" }),
      verdictRow({ ts: "2026-09-23T12:03:00.000Z", ours: "unsure", theirs: "drop" }),
    ],
    KINDS,
  );
  assertEquals(body, { events: 0, corrections: 1, unowned: 0, refused: 3 });
  assertEquals(saved.map((r) => [r.ours, r.theirs]), [["unsure", "drop"]]);
});

Deno.test("an uppercase judgment_id still matches its judgment and is saved lowercased", async () => {
  // Review M-3: Postgres prints uuids in lowercase; a device sending uppercase must not be unowned.
  const J3 = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";
  const { body, saved } = await run([verdictRow({ judgment_id: J3.toUpperCase() })], { [J3]: "event" });
  assertEquals(body, { events: 0, corrections: 1, unowned: 0, refused: 0 });
  assertEquals(saved.map((r) => r.judgment_id), [J3]);
});
