import { assert, assertEquals } from "@std/assert";
import { icsHandler } from "./handler.ts";

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
  assert(typeof reply.courses === "number");
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
  assertEquals(first.courses, 2);
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
