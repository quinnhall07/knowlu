import { assert, assertEquals } from "@std/assert";
import { Deps, handle } from "./handler.ts";

const NOW = new Date("2026-09-10T12:00:00.000Z");

function deps(over: Partial<Deps> = {}): Deps {
  return {
    verify: (t) => Promise.resolve(t === "good" ? { id: "acc-1", email: "a@example.invalid" } : null),
    requireEntitled: () => Promise.resolve({ account_id: "acc-1" }),
    getAccount: () => Promise.resolve({ email: "a@example.invalid", stripe_customer_id: "cus_1" }),
    getSubscriptionId: () => Promise.resolve("sub_1"),
    stripe: () => Promise.resolve({}),
    purge: () => Promise.resolve(),
    deleteAuthUser: () => Promise.resolve(),
    tombstone: () => Promise.resolve(),
    exportAll: () =>
      Promise.resolve({
        account: {},
        entitlement: null,
        consents: [],
        sources: [],
        telemetry_events: [],
        corrections: [],
        issues: [],
      }),
    hashEmail: (e) => Promise.resolve(`hash(${e})`),
    getSources: () => Promise.resolve([]),
    putSource: () => Promise.resolve(),
    now: () => NOW,
    ...over,
  };
}

const req = (method: string, path: string, body?: unknown, auth = "Bearer good") =>
  new Request(`http://127.0.0.1:1/functions/v1/account${path}`, {
    method,
    headers: { authorization: auth },
    body: body === undefined ? undefined : JSON.stringify(body),
  });

Deno.test("DELETE /account cancels at period end, purges, tombstones, and kills the login LAST", async () => {
  const order: string[] = [];
  const res = await handle(
    req("DELETE", ""),
    deps({
      stripe: (path, form) => {
        order.push(`stripe ${path} ${JSON.stringify(form)}`);
        return Promise.resolve({});
      },
      purge: () => {
        order.push("purge");
        return Promise.resolve();
      },
      tombstone: (h) => {
        order.push(`tombstone ${h}`);
        return Promise.resolve();
      },
      deleteAuthUser: () => {
        order.push("deleteAuthUser");
        return Promise.resolve();
      },
    }),
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { deleted: true });
  assertEquals(order, [
    'stripe /v1/subscriptions/sub_1 {"cancel_at_period_end":"true"}',
    "purge",
    "tombstone hash(a@example.invalid)",
    "deleteAuthUser",
  ]);
});

Deno.test("DELETE /account on an account that never subscribed still deletes everything", async () => {
  const order: string[] = [];
  const res = await handle(
    req("DELETE", ""),
    deps({
      getSubscriptionId: () => Promise.resolve(null),
      stripe: () => {
        throw new Error("Stripe must not be called when there is no subscription");
      },
      purge: () => {
        order.push("purge");
        return Promise.resolve();
      },
      tombstone: (h) => {
        order.push(`tombstone ${h}`);
        return Promise.resolve();
      },
      deleteAuthUser: () => {
        order.push("deleteAuthUser");
        return Promise.resolve();
      },
    }),
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { deleted: true });
  assertEquals(order, ["purge", "tombstone hash(a@example.invalid)", "deleteAuthUser"]);
});

Deno.test("DELETE /account when the accounts row is already gone still tombstones and finishes the login (a retry after a partial deletion)", async () => {
  const order: string[] = [];
  const res = await handle(
    req("DELETE", ""),
    deps({
      getAccount: () => Promise.resolve(null),
      stripe: () => {
        throw new Error("Stripe must not be called once the accounts row is gone");
      },
      purge: () => {
        throw new Error("purge must not run once the accounts row is gone — there is nothing left to purge");
      },
      tombstone: (h) => {
        order.push(`tombstone ${h}`);
        return Promise.resolve();
      },
      deleteAuthUser: () => {
        order.push("deleteAuthUser");
        return Promise.resolve();
      },
    }),
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { deleted: true });
  assertEquals(order, ["tombstone hash(a@example.invalid)", "deleteAuthUser"]);
});

Deno.test("GET /account/export hands back every table, keyed and complete", async () => {
  const res = await handle(req("GET", "/export"), deps());
  assertEquals(res.status, 200);
  const body = await res.json();
  assertEquals(Object.keys(body).sort(), [
    "account",
    "consents",
    "corrections",
    "entitlement",
    "exported_at",
    "issues",
    "sources",
    "telemetry_events",
  ]);
  assertEquals(body.exported_at, "2026-09-10T12:00:00.000Z");
});

Deno.test("an unknown path is 404 and an unknown method on a known path is 405", async () => {
  assertEquals((await handle(req("GET", "/nope"), deps())).status, 404);
  assertEquals((await handle(req("POST", "/export"), deps())).status, 405);
});

Deno.test("every route needs a bearer token", async () => {
  // PUT is not in this loop: the deps() stub's requireEntitled resolves regardless of the token, so
  // a PUT here would not exercise deps.verify the way DELETE/GET/GET do — its own 401 is not pinned
  // by this loop, and it is production's requireActiveEntitlement that actually calls requireUser.
  for (const [m, p] of [["DELETE", ""], ["GET", "/export"], ["GET", "/sources"]] as const) {
    const res = await handle(req(m, p, undefined, "Basic nope"), deps()).catch((e) => e as Response);
    assertEquals(res.status, 401, `${m} ${p}`);
  }
});

Deno.test("PUT /account/sources stores an https .ics link and answers with the kind only", async () => {
  let stored: [string, string, string] | null = null;
  const res = await handle(
    req("PUT", "/sources", { kind: "lms_ics", url: "https://lms.example.invalid/feed/abc.ics" }),
    deps({
      putSource: (a, k, u) => {
        stored = [a, k, u];
        return Promise.resolve();
      },
    }),
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { kind: "lms_ics" });
  assertEquals(stored, ["acc-1", "lms_ics", "https://lms.example.invalid/feed/abc.ics"]);
});

Deno.test("PUT /account/sources refuses google_calendar from a client — that row is C2's to write", async () => {
  let stored = false;
  const res = await handle(
    req("PUT", "/sources", { kind: "google_calendar", url: "https://calendar.google.com/x.ics" }),
    deps({
      putSource: () => {
        stored = true;
        return Promise.resolve();
      },
    }),
  ).catch((e) => e as Response);
  assertEquals(res.status, 403);
  assert(!stored, "a client must not be able to forge the kind C2's callback writes");
});

Deno.test("PUT /account/sources needs an ACTIVE subscription — the 402 C2 imports", async () => {
  let stored = false;
  const res = await handle(
    req("PUT", "/sources", { kind: "lms_ics", url: "https://lms.example.invalid/feed/abc.ics" }),
    deps({
      requireEntitled: () =>
        Promise.reject(
          new Response(JSON.stringify({ error: "this account has no active subscription" }), {
            status: 402,
          }),
        ),
      putSource: () => {
        stored = true;
        return Promise.resolve();
      },
    }),
  ).catch((e) => e as Response);
  assertEquals(res.status, 402);
  assert(!stored);
});

Deno.test("PUT /account/sources takes a personal calendar too, under its own kind", async () => {
  let stored: [string, string, string] | null = null;
  const res = await handle(
    req("PUT", "/sources", {
      kind: "calendar_ics",
      url: "https://calendar.google.com/calendar/ical/abc%40group.calendar.google.com/private-def/basic.ics",
    }),
    deps({
      putSource: (a, k, u) => {
        stored = [a, k, u];
        return Promise.resolve();
      },
    }),
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { kind: "calendar_ics" });
  assertEquals(stored![1], "calendar_ics");
  // The secret address is a capability URL like the school one, and the reply does not echo it.
  assertEquals(
    Object.keys(
      await (await handle(
        req("PUT", "/sources", {
          kind: "calendar_ics",
          url: "https://calendar.google.com/calendar/ical/x/private-def/basic.ics",
        }),
        deps(),
      )).json(),
    ),
    ["kind"],
  );
});

Deno.test("PUT /account/sources refuses a URL that is not https, and an unknown kind", async () => {
  for (
    const body of [
      { kind: "lms_ics", url: "http://lms.example.invalid/feed/abc.ics" },
      { kind: "lms_ics", url: "file:///c:/x.ics" },
      { kind: "gradebook", url: "https://lms.example.invalid/x.ics" },
      { kind: "calendar_ics", url: "webcal://calendar.google.com/x.ics" },
      { kind: "lms_ics", url: "https://" + "a".repeat(3000) },
    ]
  ) {
    const res = await handle(req("PUT", "/sources", body), deps()).catch((e) => e as Response);
    assertEquals(res.status, 400, JSON.stringify(body));
  }
});

Deno.test("GET /account/sources returns kinds and dates, and never the URL", async () => {
  const res = await handle(
    req("GET", "/sources"),
    deps({ getSources: () => Promise.resolve([{ kind: "lms_ics", added_at: "2026-09-10T00:00:00+00:00" }]) }),
  );
  assertEquals(res.status, 200);
  const text = await res.text();
  assertEquals(JSON.parse(text), { sources: [{ kind: "lms_ics", added_at: "2026-09-10T00:00:00+00:00" }] });
  assert(!text.includes("http"), "a URL reached the response body");
});

Deno.test("PUT /account/sources checks entitlement before it reads the body — an unparseable body still answers the gate's 402", async () => {
  const res = await handle(
    new Request("http://127.0.0.1:1/functions/v1/account/sources", {
      method: "PUT",
      headers: { authorization: "Bearer good" },
      body: "not json",
    }),
    deps({
      requireEntitled: () =>
        Promise.reject(
          new Response(JSON.stringify({ error: "this account has no active subscription" }), {
            status: 402,
          }),
        ),
    }),
  ).catch((e) => e as Response);
  assertEquals(res.status, 402);
});

Deno.test("POST /account/sources is 405 with the exact methods the route allows", async () => {
  const res = await handle(
    req("POST", "/sources", { kind: "lms_ics", url: "https://lms.example.invalid/feed/abc.ics" }),
    deps(),
  );
  assertEquals(res.status, 405);
  assertEquals(res.headers.get("allow"), "GET, PUT");
});

Deno.test("PUT /account/sources — a null JSON body is 400, not 500", async () => {
  const res = await handle(req("PUT", "/sources", null), deps()).catch((e) => e as Response);
  assertEquals(res.status, 400);
});

Deno.test("PUT /account/sources — the unknown-kind message lists only the kinds a client may send", async () => {
  const res = await handle(
    req("PUT", "/sources", { kind: "gradebook", url: "https://lms.example.invalid/x.ics" }),
    deps(),
  ).catch((e) => e as Response);
  assertEquals(res.status, 400);
  const body = await res.json();
  assertEquals(body.error, "unknown source kind; use lms_ics, calendar_ics");
  assert(!String(body.error).includes("google_calendar"), "the message must not advertise google_calendar");
});

// `index.ts` is never loaded by `deno test`, so nothing today would fail if `putSource` wrote
// `url_plaintext: url` or dropped the encryption call entirely — this pins both.
Deno.test("putSource encrypts the URL before the row it writes ever reaches restUpsert", async () => {
  const text = await Deno.readTextFile(new URL("./index.ts", import.meta.url));
  const match = text.match(/putSource: async[\s\S]*?\}\], "account_id,kind"\);/);
  assert(match, "could not find putSource's body, from its declaration to the closing of restUpsert");
  const body = match[0];
  assert(body.includes("encryptString("), "putSource must call encryptString before it writes");
  assert(body.includes("importAesKey("), "putSource must call importAesKey to get a key to encrypt with");
  // No key in the upserted row literal may hold the bare `url` parameter as its value — every URL
  // that leaves this function must go through `encryptString` first.
  assert(!/:\s*url\b/.test(body), "the upserted row must not store the bare url in any column");
});

// `index.ts` is never loaded by `deno test` (it reads `Deno.env` and calls `Deno.serve`), so its
// reads of `sources` have no runtime test to pin them. Reading the file as text is the guard,
// narrowed to what it actually protects (R-C1-21): `putSource` now legitimately writes
// `url_ciphertext`/`url_iv`, so the rule is not "the file never says these names" but "no *read* of
// `sources` — `restSelect`/`restSelectAll`, called directly or through `exportAll`'s `one`/`all`
// wrappers — ever selects them or `*`, in this function or any future one".
Deno.test("every read of the sources table selects kind and added_at, and never a capability URL column", async () => {
  const text = await Deno.readTextFile(new URL("./index.ts", import.meta.url));
  // Pinned by name: exportAll's own line, so an edit that drops or narrows it is caught here too.
  assert(
    text.includes('sources: await all("sources", `${eq(id)}&select=kind,added_at&order=kind`)'),
    "exportAll must read sources with select=kind,added_at",
  );
  // Every call in the file naming "sources" as a table argument immediately followed by its query
  // template literal — the direct `restSelect(rest, "sources", ...)` in getSources and the indirect
  // `all("sources", ...)` in exportAll alike. A write (`restUpsert(rest, "sources", [...])`) is not
  // followed by a template literal here and so is not matched — this guard is about reads.
  const reads = [...text.matchAll(/"sources",\s*`([^`]*)`/g)];
  assert(reads.length >= 2, `expected at least 2 reads of sources in index.ts, found ${reads.length}`);
  for (const [, query] of reads) {
    assert(query.includes("select=kind,added_at"), `a sources read did not select kind,added_at: ${query}`);
    assert(!query.includes("select=*"), `a sources read selected everything: ${query}`);
    assert(!query.includes("url_ciphertext"), `a sources read selected the ciphertext column: ${query}`);
    assert(!query.includes("url_iv"), `a sources read selected the IV column: ${query}`);
  }
});

// The same guard, for the one row a deletion deliberately leaves behind (R-C1-56, C2). The purge
// lives in `index.ts`, which `deno test` never loads, so nothing would have failed when the PATCH
// nulled `account_id` alone and left `ip` — the address recorded at checkout — attached to a
// surviving consent row for at least three years. The privacy policy says what is left is the hash,
// the price, the terms version and the date; this is what makes that sentence true.
Deno.test("the purge strips both the account id and the IP from every consent row it leaves behind", async () => {
  const text = await Deno.readTextFile(new URL("./index.ts", import.meta.url));
  const match = text.match(/restPatch\(rest,\s*"consents",[\s\S]*?\);/);
  assert(match, "could not find the PATCH that strips the surviving consent rows");
  const call = match[0];
  assert(call.includes("account_id: null"), "a surviving consent row must lose its account id");
  assert(call.includes("ip: null"), "a surviving consent row must lose its IP address");
  // `consents` is never deleted from — that is the point of the PATCH — so a `restDelete` naming it
  // would be a different bug than this test is about, and a PATCH that set anything else on the row
  // would be rewriting evidence California requires us to keep.
  assert(!/restDelete\(rest,\s*"consents"/.test(text), "consent rows are stripped, never deleted");
  assert(
    !/subject_hash|price_cents|version|accepted_at/.test(call),
    "the PATCH must leave the hash, the price, the terms version and the date exactly as they were",
  );
});
