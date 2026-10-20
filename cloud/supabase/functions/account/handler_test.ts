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
    // Task 1: the two consent fields. Default to "this account has not consented and the write is a
    // no-op", so every DELETE/export/sources test in this file is untouched by the new route.
    hasConsent: () => Promise.resolve(false),
    recordAccountConsent: () => Promise.resolve(),
    // L4 T2: the recorded pair defaults to the very versions the consent tests below send, so a
    // second call with them is the silent 200 it always was; and the later-version write refuses
    // to run unless a test asks for it, so no existing test can reach it unnoticed.
    getConsentVersions: () => Promise.resolve({ tos_version: "2026-09-10", privacy_version: "2026-09-17" }),
    recordLaterPrivacy: () => Promise.reject(new Error("recordLaterPrivacy must not be called here")),
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
  // a PUT here would not exercise deps.verify the way DELETE/GET/GET/POST do — its own 401 is not
  // pinned by this loop, and it is production's requireActiveEntitlement that actually calls
  // requireUser. POST /consent calls requireUser directly, same as the other three, so it belongs
  // here: `config.toml`'s `verify_jwt = false` is a platform-level setting, not this handler's own.
  for (
    const [m, p] of [["DELETE", ""], ["GET", "/export"], ["GET", "/sources"], ["POST", "/consent"]] as const
  ) {
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

function consentDeps(recorded: unknown[], already: boolean): Deps {
  return deps({
    hasConsent: () => Promise.resolve(already),
    recordAccountConsent: (c) => {
      recorded.push(c);
      return Promise.resolve();
    },
    now: () => new Date("2026-09-17T12:00:00Z"),
  });
}

Deno.test("POST /account/consent records the attestation and both versions", async () => {
  const recorded: unknown[] = [];
  const res = await handle(
    new Request("http://127.0.0.1:1/account/consent", {
      method: "POST",
      headers: { authorization: "Bearer good" },
      body: JSON.stringify({ tos_version: "2026-09-10", privacy_version: "2026-09-17", age_attested: true }),
    }),
    consentDeps(recorded, false),
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { ok: true });
  assertEquals(recorded, [{
    account_id: "acc-1",
    email: "a@example.invalid",
    tos_version: "2026-09-10",
    privacy_version: "2026-09-17",
    at: "2026-09-17T12:00:00.000Z",
  }]);
});

Deno.test("a second call is a no-op — the app calls it after every sign-in", async () => {
  const recorded: unknown[] = [];
  const res = await handle(
    new Request("http://127.0.0.1:1/account/consent", {
      method: "POST",
      headers: { authorization: "Bearer good" },
      body: JSON.stringify({ tos_version: "2026-09-10", privacy_version: "2026-09-17", age_attested: true }),
    }),
    consentDeps(recorded, true),
  );
  assertEquals(res.status, 200);
  assertEquals(recorded.length, 0, "an account that already consented is not written again");
});

Deno.test("age_attested false, or a missing version, is 400 and writes nothing", async () => {
  for (
    const body of [
      { tos_version: "2026-09-10", privacy_version: "2026-09-17", age_attested: false },
      { tos_version: "", privacy_version: "2026-09-17", age_attested: true },
      { tos_version: "2026-09-10", age_attested: true },
    ]
  ) {
    const recorded: unknown[] = [];
    const res = await handle(
      new Request("http://127.0.0.1:1/account/consent", {
        method: "POST",
        headers: { authorization: "Bearer good" },
        body: JSON.stringify(body),
      }),
      consentDeps(recorded, false),
    ).catch((e) => e as Response);
    assertEquals(res.status, 400);
    assertEquals(recorded.length, 0);
  }
});

Deno.test("POST /account/consent — a null JSON body is 400, not 500", async () => {
  // A body of literal `null` is valid JSON, so `readJson` does not throw; without the `?? {}` guard
  // in `recordConsent`, `body.age_attested` would throw a bare TypeError that `asResponse` turns
  // into a 500 — the same class of bug `putSource`'s own null-body test pins for that route.
  const recorded: unknown[] = [];
  const res = await handle(
    new Request("http://127.0.0.1:1/account/consent", {
      method: "POST",
      headers: { authorization: "Bearer good" },
      body: "null",
    }),
    consentDeps(recorded, false),
  ).catch((e) => e as Response);
  assertEquals(res.status, 400);
  assertEquals(recorded.length, 0);
});

Deno.test("PUT /account/consent is 405, not 404 — the route exists", async () => {
  const res = await handle(
    new Request("http://127.0.0.1:1/account/consent", {
      method: "PUT",
      headers: { authorization: "Bearer good" },
    }),
    consentDeps([], false),
  );
  assertEquals(res.status, 405);
  assertEquals(res.headers.get("allow"), "GET, POST");
});

// L4 T2 (plan §4.3): GET /account/consent reads the caller's recorded pair, and nothing else.
const consentGet = (auth = "Bearer good") => req("GET", "/consent", undefined, auth);

Deno.test("GET /account/consent answers the recorded terms and privacy versions, and nothing else", async () => {
  const asked: string[] = [];
  const res = await handle(
    consentGet(),
    deps({
      getConsentVersions: (id) => {
        asked.push(id);
        // A dep that over-reads must not leak: the reply is built field by field, never passed on.
        return Promise.resolve(
          {
            tos_version: "2026-09-10",
            privacy_version: "2026-09-24",
            email: "a@example.invalid",
          } as unknown as {
            tos_version: string | null;
            privacy_version: string | null;
          },
        );
      },
    }),
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { tos_version: "2026-09-10", privacy_version: "2026-09-24" });
  assertEquals(asked, ["acc-1"], "the read is scoped to the verified caller");
});

Deno.test("GET /account/consent answers a never-recorded pair as nulls", async () => {
  const res = await handle(
    consentGet(),
    deps({ getConsentVersions: () => Promise.resolve({ tos_version: null, privacy_version: null }) }),
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { tos_version: null, privacy_version: null });
});

Deno.test("GET /account/consent needs a bearer, and an account with no row is 404", async () => {
  const noBearer = await handle(consentGet("Basic nope"), deps()).catch((e) => e as Response);
  assertEquals(noBearer.status, 401);
  const noRow = await handle(consentGet(), deps({ getConsentVersions: () => Promise.resolve(null) }))
    .catch((e) => e as Response);
  assertEquals(noRow.status, 404);
  assertEquals(await noRow.json(), { error: "no such account" });
});

Deno.test("/account/consent takes GET and POST only, and its 405 names both", async () => {
  for (const m of ["PUT", "DELETE", "PATCH"]) {
    const res = await handle(req(m, "/consent"), deps());
    assertEquals(res.status, 405, m);
    assertEquals(res.headers.get("allow"), "GET, POST", m);
  }
});

// L4 T2 (plan §4.3): an account that already consented can record a LATER privacy version, and only
// that: one `privacy` row and the column moved forward, never a `tos` or `age_18` row or a restamp.
type Writes = { first: unknown[]; later: unknown[] };

function laterDeps(w: Writes, recorded: string | null, already = true): Deps {
  return deps({
    hasConsent: () => Promise.resolve(already),
    getConsentVersions: () => Promise.resolve({ tos_version: "2026-09-10", privacy_version: recorded }),
    recordAccountConsent: (c) => {
      w.first.push(c);
      return Promise.resolve();
    },
    recordLaterPrivacy: (c) => {
      w.later.push(c);
      return Promise.resolve();
    },
    now: () => new Date("2026-10-20T12:00:00Z"),
  });
}

const consentPost = (privacy: string) =>
  req("POST", "/consent", { tos_version: "2026-09-10", privacy_version: privacy, age_attested: true });

Deno.test("POST /account/consent records a later privacy version once, and nothing of the first consent", async () => {
  const w: Writes = { first: [], later: [] };
  const res = await handle(consentPost("2026-10-20"), laterDeps(w, "2026-09-24"));
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { ok: true });
  assertEquals(w.later, [{
    account_id: "acc-1",
    email: "a@example.invalid",
    privacy_version: "2026-10-20",
    at: "2026-10-20T12:00:00.000Z",
  }]);
  assertEquals(w.first.length, 0, "a later version never re-runs the first consent's writes");
});

Deno.test("a recorded privacy version of null counts as older", async () => {
  const w: Writes = { first: [], later: [] };
  const res = await handle(consentPost("2026-09-24"), laterDeps(w, null));
  assertEquals(res.status, 200);
  assertEquals(w.later.length, 1);
  assertEquals(w.first.length, 0);
});

Deno.test("an equal or an older privacy version is the silent 200 and writes nothing", async () => {
  for (const sent of ["2026-09-24", "2026-09-17"]) {
    const w: Writes = { first: [], later: [] };
    const res = await handle(consentPost(sent), laterDeps(w, "2026-09-24"));
    assertEquals(res.status, 200, sent);
    assertEquals(await res.json(), { ok: true });
    assertEquals(w.later.length + w.first.length, 0, sent);
  }
});

Deno.test("a later privacy version that is not a YYYY-MM-DD date is 400 and writes nothing", async () => {
  // Each of these sorts after the recorded date as a string, so a forward-only filter alone would
  // let it through and then pin the column past every real version.
  for (const sent of ["2026-10-20x", "zzzz", "2026-10-2", "2026-13-45", "2026-11-31", "２０２６-10-20"]) {
    const w: Writes = { first: [], later: [] };
    const res = await handle(consentPost(sent), laterDeps(w, "2026-09-24")).catch((e) => e as Response);
    assertEquals(res.status, 400, sent);
    assertEquals(w.later.length + w.first.length, 0, sent);
  }
});

Deno.test("a fresh account still takes the first-consent path, not the later one", async () => {
  const w: Writes = { first: [], later: [] };
  const res = await handle(consentPost("2026-10-20"), laterDeps(w, null, false));
  assertEquals(res.status, 200);
  assertEquals(w.later.length, 0);
  assertEquals(w.first.length, 1);
});

// `index.ts` is never loaded by `deno test`, so its two consent deps are pinned by their text. The
// forward-only filter lives in the PATCH's query string, not only in the handler: two racing Accepts
// must not be able to move `accounts.privacy_version` backwards.
Deno.test("the later-version write moves the column forward only and inserts one privacy row", async () => {
  const text = await Deno.readTextFile(new URL("./index.ts", import.meta.url));
  const start = text.indexOf("recordLaterPrivacy:");
  assert(start >= 0, "index.ts must define recordLaterPrivacy");
  const end = text.indexOf("\n      },\n", start);
  assert(end > start, "could not find the end of recordLaterPrivacy");
  const block = text.slice(start, end);
  assert(
    block.includes("const v = encodeURIComponent(c.privacy_version);") &&
      block.includes("`or=(privacy_version.is.null,privacy_version.lt.${v})`") &&
      block.includes("`id=eq.${encodeURIComponent(c.account_id)}&${forward}`"),
    "the PATCH's query must only ever move privacy_version forward",
  );
  assertEquals([...block.matchAll(/restPatch\(/g)].length, 1, "one PATCH, on accounts");
  assert(/restPatch\(\s*rest,\s*"accounts",/.test(block), "the PATCH is on accounts");
  const patchBody = block.match(/restPatch\(rest, "accounts", `[^`]*`, \{([^}]*)\}\);/);
  assert(patchBody, "could not find the PATCH's body");
  assertEquals(
    patchBody[1].trim(),
    "privacy_version: c.privacy_version,",
    "the PATCH sets privacy_version alone",
  );
  assertEquals([...block.matchAll(/kind: "/g)].length, 1, "exactly one consents row");
  assert(
    /restUpsert\(rest, "consents", \[\s*\{[^\]]*kind: "privacy"[^\]]*\}\s*,?\s*\]\)/.test(block),
    "of kind privacy",
  );
  for (const name of ["tos_version", "tos_accepted_at", "age_attested_at", '"tos"', '"age_18"']) {
    assert(!block.includes(name), `the later-version write must not name ${name}`);
  }
});

Deno.test("the consent read selects the two version columns of the caller's row alone", async () => {
  const text = await Deno.readTextFile(new URL("./index.ts", import.meta.url));
  assert(
    text.includes("`id=eq.${encodeURIComponent(id)}&select=tos_version,privacy_version&limit=1`"),
    "getConsentVersions must read tos_version and privacy_version, scoped by id",
  );
});

// The purge list is what the privacy policy's deletion paragraph is written from (L8 §8.6, Q-PURGE):
// both tables cascade from `accounts`, and are named here regardless.
Deno.test("the purge list names usage_daily and judgments", async () => {
  const text = await Deno.readTextFile(new URL("./index.ts", import.meta.url));
  const match = text.match(/for \(\s*const table of \[([\s\S]*?)\]\s*\)/);
  assert(match, "could not find the purge list");
  for (const table of ["usage_daily", "judgments"]) {
    assert(match[1].includes(`"${table}",`), `the purge list must name ${table}`);
  }
});

// Friends testers (docs/specs/2026-10-18-friends-testers-design.md §3.3, §3.5, Q7, Q12, §9; L8 S4).
// `exportAll` and `purge` live in `index.ts`, which `deno test` never loads, so these pins read it as
// text, as the `sources` guard above does.
Deno.test("the export reads comp_grants, comp_audit, route_daily and usage_daily by account id, paged and ordered", async () => {
  const text = await Deno.readTextFile(new URL("./index.ts", import.meta.url));
  for (const table of ["comp_grants", "comp_audit", "route_daily", "usage_daily"]) {
    // `all(` or `one(`, the table, then its query template literal, on one line or several.
    const reads = [...text.matchAll(new RegExp(String.raw`(\w+)\(\s*"${table}",\s*` + "`([^`]*)`", "g"))];
    assertEquals(reads.length, 1, `exactly one read of ${table} in index.ts`);
    const [, via, query] = reads[0];
    assertEquals(via, "all", `${table} is read through the paging all(…), never one(…)`);
    assert(query.startsWith("${eq(id)}&select="), `${table} is read by the caller's account_id: ${query}`);
    assert(/&order=[a-z_,]+$/.test(query), `${table}'s read carries an order, so paging is stable: ${query}`);
  }
  // Q7: the audit's export columns exactly; `db_role` and `grant_id` are internal (§3.5).
  const audit = text.match(/all\(\s*"comp_audit",\s*`([^`]*)`/)![1];
  assert(
    audit.includes("&select=action,ends_at,cohort,at&"),
    `comp_audit must select exactly the four: ${audit}`,
  );
  assert(!/select=\*|db_role|grant_id/.test(audit), `comp_audit's read leaks an internal column: ${audit}`);
});

Deno.test("the purge names route_daily, and leaves comp_grants and comp_audit to the cascade, saying so", async () => {
  const text = await Deno.readTextFile(new URL("./index.ts", import.meta.url));
  const m = text.match(
    /for \(\s*const table of \[([\s\S]*?)\]\s*\)\s*\{\s*await restDelete\(rest, table, eq\(id\)\);/,
  );
  assert(m, "could not find purge's table list");
  const list = m[1];
  const names = [...list.matchAll(/^\s*"([a-z_]+)",/gm)].map((x) => x[1]);
  assert(names.includes("route_daily"), `the purge list must name route_daily: ${names}`);
  // The service role holds no delete right on the grant tables (§3.5); a restDelete there would fail
  // the whole purge, so the list must never name them, and nothing in the file may delete from them.
  for (const t of ["comp_grants", "comp_audit"]) {
    assert(!names.includes(t), `the purge list must not name ${t}`);
    assert(!new RegExp(String.raw`restDelete\(\s*rest,\s*"${t}"`).test(text), `nothing may restDelete ${t}`);
  }
  // §3.3: the comment beside the list names both as taken by the cascade from accounts.
  // One comment block (consecutive `//` lines) carries all three, so an older comment's "cascades"
  // elsewhere in the list cannot satisfy it.
  const blocks = [...list.matchAll(/(?:^\s*\/\/.*\n)+/gm)].map((x) => x[0].replace(/^\s*\/\/ ?/gm, ""));
  const block = blocks.find((b) => b.includes("comp_grants"));
  assert(block, "a comment in the purge list names comp_grants");
  assert(block.includes("comp_audit"), "the same comment names comp_audit");
  assert(
    /cascade from\s+`accounts`/.test(block),
    "the same comment says the cascade from accounts deletes them",
  );
});
