import { assert, assertEquals } from "@std/assert";
import { icsHandler } from "./handler.ts";

const FEED = await Deno.readTextFile(
  new URL("../../../../engine/tests/fixtures/blackboard.ics", import.meta.url),
);
const OK = () => Promise.resolve({ account_id: "acct-1" });

Deno.test("the account's stored URL is fetched server-side and the URL never comes back", async () => {
  let asked = "";
  const handler = icsHandler(OK, {
    urlFor: () => Promise.resolve("https://lms.example.invalid/feed/secret-capability.ics"),
    fetchText: (url: string) => {
      asked = url;
      return Promise.resolve(FEED);
    },
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
  });
  const response = await handler(new Request("http://127.0.0.1/ingest-ics"));
  assertEquals(response.status, 502);
  assertEquals((await response.text()).includes("secret-capability"), false);
});
