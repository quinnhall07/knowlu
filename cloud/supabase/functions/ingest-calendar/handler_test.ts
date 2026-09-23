import { assert, assertEquals } from "@std/assert";
import { encryptString, importAesKey } from "../_shared/crypto.ts";
import { type CalendarDeps, calendarHandler, type GoogleEvent, type GooglePage, toIcs } from "./handler.ts";
import { guardedFetch } from "../_shared/guarded_fetch.ts";

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
    calendarList: () => {
      throw new Error("calendarList must not be called in this test");
    },
    seriesInstances: () => {
      throw new Error("seriesInstances must not be called in this test");
    },
    seriesMasters: () => {
      throw new Error("seriesMasters must not be called in this test");
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

// C2 final review F-1 (regrading m36): the decrypted secret iCal address goes through the same
// `guardedFetch` `/events` uses (`ingest-calendar/index.ts`'s `fetchText`) — real `guardedFetch`,
// no fake, on a loopback host. The guard's refusal must land in the SAME catch as an unfetchable
// feed: the named 502 the handler already answers for a network failure, with nothing about the
// address or which guard fired reflected back.
Deno.test("a personal source that decrypts to a loopback host is refused by the guard and answers the same 502 as an unfetchable feed", async () => {
  const key = await importAesKey(TEST_KEY_B64);
  const { ciphertext, iv } = await encryptString(key, "https://127.0.0.1/feed/secret-address.ics");
  const handler = calendarHandler(
    OK,
    deps({
      personalSource: () => Promise.resolve({ ciphertext, iv }),
      encKey: () => Promise.resolve(key),
      fetchText: (url: string) => guardedFetch(url),
    }),
  );
  const response = await handler(new Request("http://127.0.0.1/ingest-calendar?name=personal"));
  assertEquals(response.status, 502);
  const reply = await response.json();
  assertEquals(reply.error, "the calendar could not be fetched");
  assertEquals(JSON.stringify(reply).includes("secret-address"), false);
  assertEquals(JSON.stringify(reply).includes("127.0.0.1"), false);
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

// ---- Series (commitment model §4.1). Every title, room and calendar below is invented. ----

const NOW = new Date("2026-09-09T00:00:00Z");
const DAY = 86_400_000;

const PINNED_EVENTS = [
  {
    uid: "g-lecture",
    summary: "Invented lecture, hall B",
    start: "2026-09-10T14:00:00Z",
    end: "2026-09-10T15:15:00Z",
    allDay: false,
  },
  { uid: "g-break", summary: "Invented break", start: "2026-09-12", end: "2026-09-13", allDay: true },
  {
    uid: "g;odd\\uid",
    summary: "Line one\nline two; with \\ slash",
    start: "2026-09-11T09:30:00-05:00",
    end: "2026-09-11T10:00:00-05:00",
    allDay: false,
  },
];

// Captured from `handler.ts` before series existed, for PINNED_EVENTS. Raw strings: the backslashes
// here are the bytes on the wire.
const PINNED_ICS = [
  "BEGIN:VCALENDAR",
  "VERSION:2.0",
  "PRODID:-//Knowlu//ingest-calendar//EN",
  "BEGIN:VEVENT",
  "UID:g-lecture",
  String.raw`SUMMARY:Invented lecture\, hall B`,
  "DTSTART:20260910T140000Z",
  "DTEND:20260910T151500Z",
  "END:VEVENT",
  "BEGIN:VEVENT",
  "UID:g-break",
  "SUMMARY:Invented break",
  "DTSTART;VALUE=DATE:20260912",
  "DTEND;VALUE=DATE:20260913",
  "END:VEVENT",
  "BEGIN:VEVENT",
  String.raw`UID:g\;odd\\uid`,
  String.raw`SUMMARY:Line one\nline two\; with \\ slash`,
  "DTSTART:20260911T143000Z",
  "DTEND:20260911T150000Z",
  "END:VEVENT",
  "END:VCALENDAR",
].join("\r\n") + "\r\n";

const PINNED_BODY = String
  .raw`{"ics":"BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Knowlu//ingest-calendar//EN\r\nBEGIN:VEVENT\r\nUID:g-lecture\r\nSUMMARY:Invented lecture\\, hall B\r\nDTSTART:20260910T140000Z\r\nDTEND:20260910T151500Z\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:g-break\r\nSUMMARY:Invented break\r\nDTSTART;VALUE=DATE:20260912\r\nDTEND;VALUE=DATE:20260913\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:g\\;odd\\\\uid\r\nSUMMARY:Line one\\nline two\\; with \\\\ slash\r\nDTSTART:20260911T143000Z\r\nDTEND:20260911T150000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n","source":"google_calendar"}`;

/** One recurring instance on `day` (1..28 of September 2026), 14:00-14:50 UTC unless overridden. */
function inst(series: string, day: number, extra: Partial<GoogleEvent> = {}): GoogleEvent {
  const d = String(day).padStart(2, "0");
  return {
    id: `${series}_202609${d}`,
    recurringEventId: series,
    status: "confirmed",
    summary: `Invented ${series}`,
    location: "Invented Hall 1",
    start: { dateTime: `2026-09-${d}T09:00:00-05:00` },
    end: { dateTime: `2026-09-${d}T09:50:00-05:00` },
    ...extra,
  };
}

function master(series: string, extra: Partial<GoogleEvent> = {}): GoogleEvent {
  return {
    id: series,
    status: "confirmed",
    summary: `Invented ${series}`,
    location: "Invented Hall 1",
    start: { dateTime: "2026-08-19T09:00:00-05:00" },
    end: { dateTime: "2026-08-19T09:50:00-05:00" },
    recurrence: ["RRULE:FREQ=WEEKLY;BYDAY=MO,WE,FR;UNTIL=20261205T055959Z"],
    ...extra,
  };
}

const onePage = (items: GoogleEvent[]): Promise<GooglePage> => Promise.resolve({ items });

/** Deps for a connected Google account whose only calendar is the primary one. */
function google(overrides: Partial<CalendarDeps> = {}) {
  return deps({
    calendarTokenFor: () => Promise.resolve("token-not-a-secret"),
    googleEvents: () => Promise.resolve(PINNED_EVENTS),
    calendarList: () => Promise.resolve([{ id: "primary-cal@invalid", accessRole: "owner", primary: true }]),
    seriesInstances: () => onePage([]),
    seriesMasters: () => onePage([]),
    ...overrides,
  });
}

async function calendarKey(accountId: string, calendarId: string): Promise<string> {
  const digest = await crypto.subtle.digest(
    "SHA-256",
    new TextEncoder().encode(`${accountId}\n${calendarId}`),
  );
  return "google:" + Array.from(new Uint8Array(digest)).map((b) => b.toString(16).padStart(2, "0")).join("")
    .slice(0, 16);
}

async function ask(handler: (req: Request) => Promise<Response>, query: string) {
  const response = await handler(new Request(`http://127.0.0.1/ingest-calendar?${query}`));
  assertEquals(response.status, 200);
  return await response.json();
}

Deno.test("toIcs output is pinned byte-for-byte", () => {
  assertEquals(toIcs(PINNED_EVENTS), PINNED_ICS);
});

Deno.test("without accepts=series the google reply is byte-for-byte today's", async () => {
  let listed = 0;
  const handler = calendarHandler(
    OK,
    google({
      calendarList: () => {
        listed++;
        return Promise.resolve([]);
      },
    }),
  );
  for (const query of ["name=google", "name=google&accepts=", "name=google&accepts=nothing,Series"]) {
    const response = await handler(new Request(`http://127.0.0.1/ingest-calendar?${query}`));
    assertEquals(await response.text(), PINNED_BODY, query);
  }
  assertEquals(listed, 0, "no series call is made without the capability");
});

Deno.test("accepts=series,unknown gives series; unknown words are ignored", async () => {
  const handler = calendarHandler(OK, google({ seriesInstances: () => onePage([inst("s1", 9)]) }));
  const reply = await ask(handler, "name=google&accepts=series,unknown");
  assertEquals(Object.keys(reply), ["ics", "source", "series"]);
  assertEquals(reply.ics, PINNED_ICS);
  assertEquals(reply.source, "google_calendar");
  assertEquals(reply.series.calendars_read, [await calendarKey("acct-1", "primary-cal@invalid")]);
  assertEquals(reply.series.items.length, 1);
  assertEquals((await ask(handler, "name=google&accepts=unknown,series")).series.items.length, 1);
});

Deno.test("name=personal never carries series, with or without accepts", async () => {
  const key = await importAesKey(TEST_KEY_B64);
  const { ciphertext, iv } = await encryptString(key, "https://cal.example.invalid/feed.ics");
  const handler = calendarHandler(
    OK,
    deps({
      personalSource: () => Promise.resolve({ ciphertext, iv }),
      encKey: () => Promise.resolve(key),
      fetchText: () => Promise.resolve("BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n"),
    }),
  );
  for (const query of ["name=personal", "name=personal&accepts=series", "accepts=series"]) {
    const response = await handler(new Request(`http://127.0.0.1/ingest-calendar?${query}`));
    assertEquals(
      await response.text(),
      String.raw`{"ics":"BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n","source":"calendar_ics"}`,
      query,
    );
  }
});

Deno.test("calendars are primary then owned non-hidden by id, at most 10", async () => {
  const owned = Array.from({ length: 12 }, (_, i) => `owned-${String(i).padStart(2, "0")}@invalid`);
  const entries = [
    ...owned.slice(6).map((id) => ({ id, accessRole: "owner" })),
    { id: "a-shared@invalid", accessRole: "reader" },
    { id: "a-hidden@invalid", accessRole: "owner", hidden: true },
    { id: "zz-primary@invalid", accessRole: "owner", primary: true },
    ...owned.slice(0, 6).map((id) => ({ id, accessRole: "owner" })),
  ];
  const asked: string[] = [];
  const handler = calendarHandler(
    OK,
    google({
      calendarList: () => Promise.resolve(entries),
      seriesInstances: (_t, calendarId) => {
        asked.push(calendarId);
        return onePage([]);
      },
    }),
  );
  const reply = await ask(handler, "name=google&accepts=series");
  const expected = ["zz-primary@invalid", ...owned.slice(0, 9)];
  assertEquals(
    reply.series.calendars_read,
    await Promise.all(expected.map((id) => calendarKey("acct-1", id))),
  );
  assertEquals([...asked].sort(), [...expected].sort());
});

Deno.test("a calendar key is google: plus 16 hex of sha256(account_id newline id)", async () => {
  const handler = calendarHandler(
    () => Promise.resolve({ account_id: "acct-invented-7" }),
    google({ seriesInstances: () => onePage([inst("s1", 9)]) }),
  );
  const reply = await ask(handler, "name=google&accepts=series");
  const digest = new Uint8Array(
    await crypto.subtle.digest("SHA-256", new TextEncoder().encode("acct-invented-7\nprimary-cal@invalid")),
  );
  const hex = Array.from(digest).map((b) => b.toString(16).padStart(2, "0")).join("");
  const key = `google:${hex.slice(0, 16)}`;
  assert(/^google:[0-9a-f]{16}$/.test(key));
  assertEquals(reply.series.calendars_read, [key]);
  assertEquals(reply.series.items[0].calendar, key);
  assertEquals(JSON.stringify(reply.series).includes("primary-cal"), false, "the calendar id never leaves");
});

Deno.test("pages are followed; a sixth page leaves the calendar out of calendars_read and sends none of its items", async () => {
  const pageCalls: Record<string, number> = {};
  const handler = calendarHandler(
    OK,
    google({
      calendarList: () =>
        Promise.resolve([
          { id: "a-primary@invalid", accessRole: "owner", primary: true },
          { id: "b-long@invalid", accessRole: "owner" },
        ]),
      seriesInstances: (_t, calendarId, _f, _to, pageToken) => {
        const n = pageToken === undefined ? 0 : Number(pageToken);
        pageCalls[calendarId] = (pageCalls[calendarId] ?? 0) + 1;
        if (calendarId === "a-primary@invalid") {
          // Three pages, then done: every page's item arrives.
          return Promise.resolve({
            items: [inst("s-short", 9 + n)],
            nextPageToken: n < 2 ? String(n + 1) : undefined,
          });
        }
        // Never runs out: a sixth page would be needed.
        return Promise.resolve({ items: [inst("s-long", 9 + n)], nextPageToken: String(n + 1) });
      },
    }),
  );
  const reply = await ask(handler, "name=google&accepts=series");
  assertEquals(reply.series.calendars_read, [await calendarKey("acct-1", "a-primary@invalid")]);
  assertEquals(reply.series.items.map((i: { id: string }) => i.id), ["s-short"]);
  assertEquals(reply.series.items[0].instances.length, 3);
  assertEquals(pageCalls["a-primary@invalid"], 3);
  assertEquals(pageCalls["b-long@invalid"], 5, "a sixth page is never asked for");
});

Deno.test("the 100-series cap leaves out the whole calendar that would cross it", async () => {
  const seriesOn = (prefix: string, n: number) =>
    Array.from({ length: n }, (_, i) => inst(`${prefix}-${String(i).padStart(3, "0")}`, 9));
  const handler = calendarHandler(
    OK,
    google({
      calendarList: () =>
        Promise.resolve([
          { id: "a-primary@invalid", accessRole: "owner", primary: true },
          { id: "b-big@invalid", accessRole: "owner" },
          { id: "c-small@invalid", accessRole: "owner" },
        ]),
      seriesInstances: (_t, calendarId) =>
        onePage(
          calendarId === "a-primary@invalid"
            ? seriesOn("a", 60)
            : calendarId === "b-big@invalid"
            ? seriesOn("b", 41)
            : seriesOn("c", 40),
        ),
    }),
  );
  const reply = await ask(handler, "name=google&accepts=series");
  assertEquals(
    reply.series.calendars_read,
    [await calendarKey("acct-1", "a-primary@invalid"), await calendarKey("acct-1", "c-small@invalid")],
  );
  assertEquals(reply.series.items.length, 100);
  assertEquals(reply.series.items.some((i: { id: string }) => i.id.startsWith("b-")), false);
  assertEquals(reply.series.items[0].id, "a-000");
  assertEquals(reply.series.items[99].id, "c-039");
});

Deno.test("a series with 41 instances is dropped", async () => {
  const many = Array.from({ length: 41 }, (_, i) => inst("s-busy", 1 + (i % 28), { id: `busy-${i}` }));
  const forty = Array.from({ length: 40 }, (_, i) => inst("s-full", 1 + (i % 28), { id: `full-${i}` }));
  const handler = calendarHandler(OK, google({ seriesInstances: () => onePage([...many, ...forty]) }));
  const reply = await ask(handler, "name=google&accepts=series");
  assertEquals(reply.series.items.map((i: { id: string }) => i.id), ["s-full"]);
  assertEquals(reply.series.items[0].instances.length, 40);
  assertEquals(reply.series.calendars_read.length, 1, "the calendar itself is still read");
});

Deno.test("cancelled instances, all-day items and a declined self attendee are not items", async () => {
  const handler = calendarHandler(
    OK,
    google({
      seriesInstances: () =>
        onePage([
          inst("s-kept", 9, { attendees: [{ self: true, responseStatus: "accepted" }] }),
          inst("s-kept", 11, { attendees: [{ email: "other@invalid", responseStatus: "declined" }] }),
          inst("s-kept", 14, { status: "cancelled" }),
          inst("s-kept", 16, { attendees: [{ self: true, responseStatus: "declined" }] }),
          inst("s-allday", 10, { start: { date: "2026-09-10" }, end: { date: "2026-09-11" } }),
          { ...inst("s-single", 12), recurringEventId: undefined },
        ]),
    }),
  );
  const reply = await ask(handler, "name=google&accepts=series");
  assertEquals(reply.series.items.map((i: { id: string }) => i.id), ["s-kept"]);
  assertEquals(reply.series.items[0].instances, [
    { start: "2026-09-09T14:00:00Z", end: "2026-09-09T14:50:00Z" },
    { start: "2026-09-11T14:00:00Z", end: "2026-09-11T14:50:00Z" },
  ]);
});

Deno.test("recurrence keeps RRULE, EXDATE and RDATE lines only", async () => {
  const handler = calendarHandler(
    OK,
    google({
      seriesInstances: () => onePage([inst("s1", 9), inst("s2", 9)]),
      seriesMasters: () =>
        onePage([
          master("s1", {
            recurrence: [
              "RRULE:FREQ=WEEKLY;BYDAY=MO,WE,FR;UNTIL=20261205T055959Z",
              "EXDATE;TZID=America/Chicago:20261125T090000",
              "RDATE;VALUE=DATE:20261201",
              "EXRULE:FREQ=MONTHLY",
              "X-INVENTED:1",
            ],
          }),
          { ...inst("s1", 11), summary: "an exception, not a master" },
          master("not-in-window"),
        ]),
    }),
  );
  const reply = await ask(handler, "name=google&accepts=series");
  assertEquals(reply.series.items[0], {
    calendar: await calendarKey("acct-1", "primary-cal@invalid"),
    id: "s1",
    title: "Invented s1",
    location: "Invented Hall 1",
    description: "",
    event_type: "default",
    first: "2026-08-19T09:00:00-05:00",
    recurrence: [
      "RRULE:FREQ=WEEKLY;BYDAY=MO,WE,FR;UNTIL=20261205T055959Z",
      "EXDATE;TZID=America/Chicago:20261125T090000",
      "RDATE;VALUE=DATE:20261201",
    ],
    instances: [{ start: "2026-09-09T14:00:00Z", end: "2026-09-09T14:50:00Z" }],
  });
  // No master returned: sent without `recurrence` (and so without `first`); the device rules it out.
  assertEquals(Object.keys(reply.series.items[1]), [
    "calendar",
    "id",
    "title",
    "location",
    "description",
    "event_type",
    "instances",
  ]);
  assertEquals(reply.series.items.length, 2, "a master with no instance in the window adds no item");
});

Deno.test("eventType passes through, default when Google omits it", async () => {
  const handler = calendarHandler(
    OK,
    google({ seriesInstances: () => onePage([inst("s1", 9), inst("s2", 9, { eventType: "focusTime" })]) }),
  );
  const reply = await ask(handler, "name=google&accepts=series");
  assertEquals(reply.series.items.map((i: { event_type: string }) => i.event_type), ["default", "focusTime"]);
});

Deno.test("description is sent only when location is empty, cut to 200", async () => {
  const long = "Invented room 101. " + "x".repeat(300);
  const handler = calendarHandler(
    OK,
    google({
      seriesInstances: () =>
        onePage([
          inst("s-placed", 9, { description: "Invented private note" }),
          inst("s-unplaced", 9, { location: undefined, description: long }),
        ]),
    }),
  );
  const reply = await ask(handler, "name=google&accepts=series");
  const [placed, unplaced] = reply.series.items;
  assertEquals(placed.description, "");
  assertEquals(JSON.stringify(reply).includes("Invented private note"), false);
  assertEquals(unplaced.location, "");
  assertEquals(unplaced.description, long.slice(0, 200));
});

Deno.test("title and location are cut to 200", async () => {
  const title = "T".repeat(249) + "\u{1F4DA}";
  const location = "L".repeat(250);
  const handler = calendarHandler(
    OK,
    google({
      seriesMasters: () => onePage([master("s1", { summary: title, location })]),
      seriesInstances: () => onePage([inst("s1", 9)]),
    }),
  );
  const reply = await ask(handler, "name=google&accepts=series");
  assertEquals(reply.series.items[0].title, "T".repeat(200));
  assertEquals(reply.series.items[0].location, "L".repeat(200));
});

Deno.test("the series window starts 24 hours before now", async () => {
  const windows: Array<[string, string, string]> = [];
  const handler = calendarHandler(
    OK,
    google({
      googleEvents: (_t, from, to) => {
        windows.push(["ics", from.toISOString(), to.toISOString()]);
        return Promise.resolve(PINNED_EVENTS);
      },
      seriesInstances: (_t, _c, from, to) => {
        windows.push(["instances", from.toISOString(), to.toISOString()]);
        return onePage([]);
      },
      seriesMasters: (_t, _c, from, to) => {
        windows.push(["masters", from.toISOString(), to.toISOString()]);
        return onePage([]);
      },
    }),
  );
  await ask(handler, "name=google&accepts=series");
  const before = new Date(NOW.getTime() - DAY).toISOString();
  const after = new Date(NOW.getTime() + 28 * DAY).toISOString();
  assertEquals(windows.sort(), [
    ["ics", NOW.toISOString(), after],
    ["instances", before, after],
    ["masters", before, after],
  ]);
});

/** Runs `body` with `console.error` captured; returns every argument it was called with. */
async function capturingErrors(body: () => Promise<void>): Promise<unknown[]> {
  const seen: unknown[] = [];
  const original = console.error;
  console.error = (...args: unknown[]) => void seen.push(...args);
  try {
    await body();
  } finally {
    console.error = original;
  }
  return seen;
}

Deno.test("past the 5-second budget series is omitted and ics is unchanged", async () => {
  // A fake clock: every read of `now` is two seconds after the last, so the series calls cross
  // five seconds of the handler's own clock while the test runs in milliseconds.
  let reads = 0;
  const handler = calendarHandler(
    OK,
    google({
      now: () => new Date(NOW.getTime() + 2000 * reads++),
      seriesInstances: () => onePage([inst("s1", 9)]),
      seriesMasters: () => onePage([master("s1")]),
    }),
  );
  let body = "";
  const errors = await capturingErrors(async () => {
    body = await (await handler(new Request("http://127.0.0.1/ingest-calendar?name=google&accepts=series")))
      .text();
  });
  assertEquals(body, PINNED_BODY);
  assertEquals(errors.map(String), ["ingest-calendar series: SeriesBudgetExceeded"]);
});

Deno.test("a hung seriesInstances call is abandoned at the budget", async () => {
  let seen: AbortSignal | undefined;
  const handler = calendarHandler(
    OK,
    google({
      seriesInstances: (_t, _c, _f, _to, _p, signal) => {
        seen = signal;
        return new Promise<GooglePage>(() => {}); // never settles, and ignores the signal
      },
    }),
    { seriesBudgetMs: 50 },
  );
  let body = "";
  await capturingErrors(async () => {
    body = await (await handler(new Request("http://127.0.0.1/ingest-calendar?name=google&accepts=series")))
      .text();
  });
  assertEquals(body, PINNED_BODY);
  assert(seen !== undefined && seen.aborted, "the call's signal was aborted by the budget");
});

Deno.test("a throwing seriesMasters omits series and keeps ics", async () => {
  const handler = calendarHandler(
    OK,
    google({
      seriesInstances: () => onePage([inst("s1", 9)]),
      seriesMasters: () => Promise.reject(new RangeError("invented")),
    }),
  );
  let body = "";
  await capturingErrors(async () => {
    body = await (await handler(new Request("http://127.0.0.1/ingest-calendar?name=google&accepts=series")))
      .text();
  });
  assertEquals(body, PINNED_BODY);
});

Deno.test("console.error receives only exception class names", async () => {
  const handler = calendarHandler(
    OK,
    google({
      seriesInstances: () => onePage([inst("s1", 9)]),
      seriesMasters: () => Promise.reject(new TypeError("Invented Secret Seminar token-not-a-secret")),
    }),
  );
  const errors = await capturingErrors(async () => {
    await handler(new Request("http://127.0.0.1/ingest-calendar?name=google&accepts=series"));
  });
  const text = errors.map(String).join(" ");
  assert(text.includes("TypeError"), text);
  assertEquals(text.includes("Invented Secret Seminar"), false);
  assertEquals(text.includes("token-not-a-secret"), false);
  assertEquals(text.includes("Invented s1"), false);
});

Deno.test("CalendarDeps has no write or persist method", () => {
  // An object literal typed as `CalendarDeps` must name every member and no other (excess-property
  // checking), so its keys are the interface's keys.
  const exact: CalendarDeps = {
    personalSource: () => Promise.resolve(null),
    calendarTokenFor: () => Promise.resolve(null),
    googleEvents: () => Promise.resolve([]),
    calendarList: () => Promise.resolve([]),
    seriesInstances: () => onePage([]),
    seriesMasters: () => onePage([]),
    fetchText: () => Promise.resolve(""),
    encKey: () => Promise.reject(new Error("unused")),
    now: () => NOW,
  };
  const keys = Object.keys(exact).sort();
  assertEquals(keys, [
    "calendarList",
    "calendarTokenFor",
    "encKey",
    "fetchText",
    "googleEvents",
    "now",
    "personalSource",
    "seriesInstances",
    "seriesMasters",
  ]);
  for (const k of keys) {
    assertEquals(/write|insert|upsert|persist|save|store|put|update|delete|create|log/i.test(k), false, k);
  }
});
