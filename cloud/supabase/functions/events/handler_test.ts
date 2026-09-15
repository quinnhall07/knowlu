import { assert, assertEquals } from "@std/assert";
import { allowedUrl, eventsHandler } from "./handler.ts";

const OK = () => Promise.resolve({ account_id: "acct-1" });

function post(url: unknown): Request {
  return new Request("http://127.0.0.1/events", { method: "POST", body: JSON.stringify({ url }) });
}

Deno.test("a public https feed is fetched and returned whole", async () => {
  let asked = "";
  const handler = eventsHandler(OK, (url) => {
    asked = url;
    return Promise.resolve("BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n");
  });
  const response = await handler(post("https://calendar.example.edu/events.ics"));
  assertEquals(response.status, 200);
  assert((await response.json()).body.includes("BEGIN:VCALENDAR"));
  assertEquals(asked, "https://calendar.example.edu/events.ics");
});

Deno.test("nothing private, plaintext, bare or malformed is ever fetched", async () => {
  // An outbound fetch on a client-supplied URL is an SSRF surface; the project's own metadata
  // endpoint is one hop away on the loopback interface.
  for (
    const bad of [
      "http://calendar.example.edu/e.ics",
      "https://127.0.0.1/e.ics",
      "https://localhost/e.ics",
      "https://10.1.2.3/e.ics",
      "https://192.168.0.9/e.ics",
      "https://169.254.169.254/latest/meta-data/",
      "https://172.16.4.4/e.ics",
      "https://intranet/e.ics",
      "not a url",
      "",
    ]
  ) {
    assertEquals(allowedUrl(bad), false, `${bad} must not be fetchable`);
    let fetched = false;
    const handler = eventsHandler(OK, () => {
      fetched = true;
      return Promise.resolve("");
    });
    const response = await handler(post(bad));
    assertEquals(response.status, 400, bad);
    assertEquals(fetched, false, `${bad} reached the fetcher`);
  }
});

Deno.test("a fetch that throws is a 502 whose body names no URL", async () => {
  const handler = eventsHandler(
    OK,
    () => Promise.reject(new Error("getaddrinfo ENOTFOUND calendar.example.edu/secret-token")),
  );
  const response = await handler(post("https://calendar.example.edu/secret-token/e.ics"));
  assertEquals(response.status, 502);
  assertEquals((await response.text()).includes("secret-token"), false);
});

Deno.test("the entitlement check runs before any fetch", async () => {
  let fetched = false;
  const refuse = () => Promise.reject(Response.json({ error: "no active subscription" }, { status: 402 }));
  const handler = eventsHandler(refuse, () => {
    fetched = true;
    return Promise.resolve("");
  });
  assertEquals((await handler(post("https://calendar.example.edu/e.ics"))).status, 402);
  assertEquals(fetched, false);
});

Deno.test("a GET is a 405", async () => {
  const handler = eventsHandler(OK, () => Promise.resolve(""));
  assertEquals((await handler(new Request("http://127.0.0.1/events"))).status, 405);
});
