import { assert, assertEquals } from "@std/assert";
import { encryptString, importAesKey } from "../_shared/crypto.ts";
import { calendarHandler, toIcs } from "./handler.ts";

const OK = () => Promise.resolve({ account_id: "acct-1" });

// A key generated for this test file only — not the project's real `SOURCES_ENC_KEY` — so the
// "fetched server-side" tests below can exercise the real `decryptString` call `handler.ts` makes,
// rather than faking around it.
const TEST_KEY_B64 = "9DmRINQlZzHfE3touabkOdAah9b1eR+RqPO8P9wdtOw=";

function deps(overrides: Partial<Parameters<typeof calendarHandler>[1]> = {}) {
  return {
    personalSource: () => {
      throw new Error("personalSource must not be called in this test");
    },
    calendarTokenFor: () => {
      throw new Error("calendarTokenFor must not be called in this test");
    },
    googleEvents: () => {
      throw new Error("googleEvents must not be called in this test");
    },
    fetchText: () => {
      throw new Error("fetchText must not be called in this test");
    },
    encKey: () => {
      throw new Error("encKey must not be called in this test");
    },
    now: () => new Date("2026-09-09T00:00:00Z"),
    ...overrides,
  };
}

Deno.test("?name=personal and no name at all both reach personalSource and never the Google token", async () => {
  let personalCalls = 0;
  let tokenCalls = 0;
  const handler = calendarHandler(
    OK,
    deps({
      personalSource: () => {
        personalCalls++;
        return Promise.resolve(null);
      },
      calendarTokenFor: () => {
        tokenCalls++;
        return Promise.resolve(null);
      },
    }),
  );
  await handler(new Request("http://127.0.0.1/ingest-calendar?name=personal"));
  await handler(new Request("http://127.0.0.1/ingest-calendar"));
  assertEquals(personalCalls, 2);
  assertEquals(tokenCalls, 0);
});

Deno.test("?name=google reaches the token and never personalSource", async () => {
  let personalCalls = 0;
  let tokenCalls = 0;
  const handler = calendarHandler(
    OK,
    deps({
      personalSource: () => {
        personalCalls++;
        return Promise.resolve(null);
      },
      calendarTokenFor: () => {
        tokenCalls++;
        return Promise.resolve(null);
      },
    }),
  );
  await handler(new Request("http://127.0.0.1/ingest-calendar?name=google"));
  assertEquals(personalCalls, 0);
  assertEquals(tokenCalls, 1);
});

Deno.test("?name=work is a 404 that names 'work' and touches neither source", async () => {
  const handler = calendarHandler(OK, deps());
  const response = await handler(new Request("http://127.0.0.1/ingest-calendar?name=work"));
  assertEquals(response.status, 404);
  const reply = await response.json();
  assertEquals(reply.error, "no calendar named 'work' for this account");
});

Deno.test("a Google grant becomes an ICS document whose DTSTART/DTEND round-trip", async () => {
  const handler = calendarHandler(
    OK,
    deps({
      calendarTokenFor: () => Promise.resolve("token-not-a-secret"),
      googleEvents: () =>
        Promise.resolve([
          {
            uid: "g1",
            summary: "Seminar",
            start: "2026-09-09T14:00:00Z",
            end: "2026-09-09T15:00:00Z",
            allDay: false,
          },
        ]),
    }),
  );
  const response = await handler(new Request("http://127.0.0.1/ingest-calendar?name=google"));
  assertEquals(response.status, 200);
  const reply = await response.json();
  assertEquals(reply.source, "google_calendar");
  assert(reply.ics.includes("DTSTART:20260909T140000Z"));
  assert(reply.ics.includes("DTEND:20260909T150000Z"));
});

Deno.test("an all-day event uses VALUE=DATE", async () => {
  const handler = calendarHandler(
    OK,
    deps({
      calendarTokenFor: () => Promise.resolve("token-not-a-secret"),
      googleEvents: () =>
        Promise.resolve([
          { uid: "g2", summary: "Fall break", start: "2026-11-26", end: "2026-11-27", allDay: true },
        ]),
    }),
  );
  const response = await handler(new Request("http://127.0.0.1/ingest-calendar?name=google"));
  const reply = await response.json();
  assert(reply.ics.includes("DTSTART;VALUE=DATE:20261126"));
  assert(reply.ics.includes("DTEND;VALUE=DATE:20261127"));
});

Deno.test("the personal source is fetched server-side and its address appears in neither the reply nor a later 502 body", async () => {
  const key = await importAesKey(TEST_KEY_B64);
  const secretUrl = "https://cal.example.invalid/feed/secret-address.ics";
  const { ciphertext, iv } = await encryptString(key, secretUrl);

  let asked = "";
  const okHandler = calendarHandler(
    OK,
    deps({
      personalSource: () => Promise.resolve({ ciphertext, iv }),
      encKey: () => Promise.resolve(key),
      fetchText: (url: string) => {
        asked = url;
        return Promise.resolve("BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n");
      },
    }),
  );
  const okResponse = await okHandler(new Request("http://127.0.0.1/ingest-calendar?name=personal"));
  assertEquals(okResponse.status, 200);
  const okReply = await okResponse.json();
  assertEquals(okReply.source, "calendar_ics");
  assertEquals(asked, secretUrl);
  assertEquals(JSON.stringify(okReply).includes("secret-address"), false);

  const failHandler = calendarHandler(
    OK,
    deps({
      personalSource: () => Promise.resolve({ ciphertext, iv }),
      encKey: () => Promise.resolve(key),
      fetchText: () =>
        Promise.reject(new Error("getaddrinfo ENOTFOUND cal.example.invalid/feed/secret-address.ics")),
    }),
  );
  const failResponse = await failHandler(new Request("http://127.0.0.1/ingest-calendar?name=personal"));
  assertEquals(failResponse.status, 502);
  assertEquals((await failResponse.text()).includes("secret-address"), false);
});

Deno.test("a Google grant without calendar.readonly is a 409, not a 404, and no fetch is attempted", async () => {
  const handler = calendarHandler(
    OK,
    deps({
      calendarTokenFor: () => Promise.resolve(null),
      googleEvents: () => {
        throw new Error("must not fetch when there is no token");
      },
    }),
  );
  const response = await handler(new Request("http://127.0.0.1/ingest-calendar?name=google"));
  assertEquals(response.status, 409);
  assertEquals((await response.json()).error, "the Google calendar is not connected");
});

Deno.test("a summary with a comma, a semicolon and a newline is escaped so the feed still parses", () => {
  const ics = toIcs([
    {
      uid: "g3",
      summary: "Office hours, room 3;\nbring questions",
      start: "2026-09-09T14:00:00Z",
      end: "2026-09-09T15:00:00Z",
      allDay: false,
    },
  ]);
  assert(ics.includes("SUMMARY:Office hours\\, room 3\\;\\nbring questions"));
  assert(ics.includes("BEGIN:VCALENDAR"));
  assert(ics.includes("END:VCALENDAR"));
});

Deno.test("the google calendar is never looked for in the sources table", async () => {
  // Ruling R-X-9: `google_calendar` is a reserved value in C1's check constraint that nobody
  // writes. A grant has no URL, and `sources.url_ciphertext` / `url_iv` are both `not null`, so a
  // row of that kind could not be filled — and `sources` has no `name` column to look one up by.
  const source = await Deno.readTextFile(new URL("./index.ts", import.meta.url));
  assertEquals(source.includes("google_calendar"), false, "the grant lives in google_accounts");
  assert(source.includes("kind=eq.calendar_ics"), "the personal calendar is the only sources row read here");
});
