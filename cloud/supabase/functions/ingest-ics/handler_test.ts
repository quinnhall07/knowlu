import { assert, assertEquals } from "@std/assert";
import { icsHandler } from "./handler.ts";
import { guardedFetch } from "../_shared/guarded_fetch.ts";

const FEED = await Deno.readTextFile(
  new URL("../../../../engine/tests/fixtures/blackboard.ics", import.meta.url),
);
const OK = () => Promise.resolve({ account_id: "acct-1" });
// F8: a fixed clock, so `pastDueUids`'s "strictly before TODAY" comparison is deterministic.
const NOW = () => new Date("2026-09-09T12:00:00Z");

Deno.test("the account's stored URL is fetched server-side and the URL never comes back", async () => {
  let asked = "";
  const handler = icsHandler(OK, {
    urlFor: () => Promise.resolve("https://lms.example.invalid/feed/secret-capability.ics"),
    fetchText: (url: string) => {
      asked = url;
      return Promise.resolve(FEED);
    },
    now: NOW,
  });
  const response = await handler(new Request("http://127.0.0.1/ingest-ics"));
  assertEquals(response.status, 200);
  const reply = await response.json();
  assert(reply.ics.includes("BEGIN:VCALENDAR"));
  assertEquals(asked, "https://lms.example.invalid/feed/secret-capability.ics");
  // The capability URL is a credential in all but name: it must never come back to the device,
  // into a log, or into an error body (cloud design §3.1, §9 Alabama SPII).
  assertEquals(JSON.stringify(reply).includes("secret-capability"), false);
});

Deno.test("an account with no stored feed is a 404 that says so", async () => {
  const handler = icsHandler(OK, {
    urlFor: () => Promise.resolve(null),
    fetchText: () => Promise.reject(new Error("must not fetch")),
    now: NOW,
  });
  const response = await handler(new Request("http://127.0.0.1/ingest-ics"));
  assertEquals(response.status, 404);
  assertEquals((await response.json()).error, "no lms_ics source for this account");
});

Deno.test("a feed that will not fetch is a 502 whose body carries no URL", async () => {
  const handler = icsHandler(OK, {
    urlFor: () => Promise.resolve("https://lms.example.invalid/feed/secret-capability.ics"),
    fetchText: () =>
      Promise.reject(new Error("getaddrinfo ENOTFOUND lms.example.invalid/feed/secret-capability.ics")),
    now: NOW,
  });
  const response = await handler(new Request("http://127.0.0.1/ingest-ics"));
  assertEquals(response.status, 502);
  assertEquals((await response.text()).includes("secret-capability"), false);
});

// C2 final review F-1 (regrading m36): the account's stored feed URL goes through the same
// `guardedFetch` `/events` uses (`ingest-ics/index.ts`'s `fetchText`) — real `guardedFetch`, no
// fake, on a loopback host. The guard's refusal must land in the SAME catch as an unfetchable
// feed: the named 502 the handler already answers for a network failure, with nothing about the
// URL or which guard fired reflected back.
Deno.test("a stored URL that resolves to a loopback host is refused by the guard and answers the same 502 as an unfetchable feed", async () => {
  const handler = icsHandler(OK, {
    urlFor: () => Promise.resolve("https://127.0.0.1/feed/secret-capability.ics"),
    fetchText: (url: string) => guardedFetch(url),
    now: NOW,
  });
  const response = await handler(new Request("http://127.0.0.1/ingest-ics"));
  assertEquals(response.status, 502);
  const reply = await response.json();
  assertEquals(reply.error, "the calendar feed could not be fetched");
  assertEquals(JSON.stringify(reply).includes("secret-capability"), false);
  assertEquals(JSON.stringify(reply).includes("127.0.0.1"), false);
});

Deno.test("first_run reports which uids are already past, and otherwise reports none", async () => {
  // R-OB-3's corroborating half. The DEVICE's check is the guarantee — it is the half that works
  // with no account and the half that knows the vault's timezone — and this is what the wizard
  // counts to say "14 upcoming, 4 already past" on the finish panel.
  const feed = "BEGIN:VCALENDAR\r\n" +
    "BEGIN:VEVENT\r\nUID:bb-old\r\nSUMMARY:Old\r\nDTSTART:20250902T045900Z\r\nEND:VEVENT\r\n" +
    "BEGIN:VEVENT\r\nUID:bb-new\r\nSUMMARY:New\r\nDTSTART:20991002T045900Z\r\nEND:VEVENT\r\n" +
    "END:VCALENDAR\r\n";
  const deps = {
    urlFor: () => Promise.resolve("https://lms.example.invalid/feed/secret-capability.ics"),
    fetchText: () => Promise.resolve(feed),
    now: NOW,
  };
  const first = await (await icsHandler(OK, deps)(new Request("http://127.0.0.1/ingest-ics?first_run=1")))
    .json();
  assertEquals(first.past_due_uids, ["bb-old"]);
  const later = await (await icsHandler(OK, deps)(new Request("http://127.0.0.1/ingest-ics"))).json();
  assertEquals(later.past_due_uids, [], "only a first ingest has a past to skip");
});

Deno.test("an event with no date at all is never called past due", async () => {
  const feed =
    "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:bb-none\r\nSUMMARY:No date\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
  const reply = await (await icsHandler(OK, {
    urlFor: () => Promise.resolve("https://lms.example.invalid/f.ics"),
    fetchText: () => Promise.resolve(feed),
    now: NOW,
  })(new Request("http://127.0.0.1/ingest-ics?first_run=1"))).json();
  assertEquals(reply.past_due_uids, []);
});

/// R-C2-E21: the server must read the same property, in the same order, the device does —
/// `["DUE", "DTEND", "DTSTART"]`, first present wins (`engine/src/ingest.rs`'s `event_from`) —
/// never DTSTART-first, which is not what creates the vault's task.
Deno.test("past_due_uids reads DUE, then DTEND, then DTSTART — the device's order, not DTSTART-first", async () => {
  // Under fixed NOW (2026-09-09T12:00:00Z), "today" is 2026-09-09 UTC and "yesterday" is 2026-09-08.
  const feed = "BEGIN:VCALENDAR\r\n" +
    // DTSTART is today (not past); DTEND is yesterday. No DUE, so DTEND is what must be read —
    // a DTSTART-first reading would wrongly leave this uid out of the list.
    "BEGIN:VEVENT\r\nUID:bb-dtend-past\r\nSUMMARY:DTEND past\r\n" +
    "DTSTART:20260909T090000Z\r\nDTEND:20260908T100000Z\r\nEND:VEVENT\r\n" +
    // DUE is yesterday; DTSTART is today. DUE must win over DTSTART.
    "BEGIN:VEVENT\r\nUID:bb-due-past\r\nSUMMARY:DUE past\r\n" +
    "DUE:20260908T100000Z\r\nDTSTART:20260909T090000Z\r\nEND:VEVENT\r\n" +
    "END:VCALENDAR\r\n";
  const reply = await (await icsHandler(OK, {
    urlFor: () => Promise.resolve("https://lms.example.invalid/f.ics"),
    fetchText: () => Promise.resolve(feed),
    now: NOW,
  })(new Request("http://127.0.0.1/ingest-ics?first_run=1"))).json();
  assertEquals(reply.past_due_uids, ["bb-dtend-past", "bb-due-past"]);
});
