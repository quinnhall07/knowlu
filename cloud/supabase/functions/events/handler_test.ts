import { assert, assertEquals, assertRejects } from "@std/assert";
import { allowedUrl, eventsHandler, guardedFetch, type ResolveDns } from "./handler.ts";

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
      "https://localhost./e.ics",
      "https://calendar.example.edu:8443/e.ics",
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

// SSRF fix round 1 (R-C2-E22 findings 3/4/6/7b) — `guardedFetch` itself, with no socket: a fake
// `fetchImpl` and a fake `resolveDns` prove the redirect-hop re-validation, the resolved-address
// check, the streamed body cap, and the abort timeout without ever touching a real network.

Deno.test("a public host resolving to a public address is fetched once", async () => {
  const resolveDns: ResolveDns = () => Promise.resolve(["93.184.216.34"]);
  let calls = 0;
  const fetchImpl: typeof fetch = () => {
    calls++;
    return Promise.resolve(new Response("BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n", { status: 200 }));
  };
  const body = await guardedFetch("https://calendar.example.edu/e.ics", { fetchImpl, resolveDns });
  assert(body.includes("BEGIN:VCALENDAR"));
  assertEquals(calls, 1);
});

Deno.test("a name that resolves to a private address is refused, never fetched", async () => {
  const resolveDns: ResolveDns = () => Promise.resolve(["10.0.0.1"]);
  let fetched = false;
  const fetchImpl: typeof fetch = () => {
    fetched = true;
    return Promise.resolve(new Response("", { status: 200 }));
  };
  await assertRejects(() => guardedFetch("https://sneaky.example.edu/e.ics", { fetchImpl, resolveDns }));
  assertEquals(fetched, false);
});

Deno.test("a host with no DNS records at all is refused", async () => {
  const resolveDns: ResolveDns = () => Promise.resolve([]);
  await assertRejects(() =>
    guardedFetch("https://nowhere.example.edu/e.ics", {
      fetchImpl: () => Promise.reject("never"),
      resolveDns,
    })
  );
});

Deno.test("a 302 to a host that resolves privately is refused at the hop, and the sneaky host is never fetched", async () => {
  const resolveDns: ResolveDns = (hostname) =>
    Promise.resolve(hostname === "calendar.example.edu" ? ["93.184.216.34"] : ["10.0.0.1"]);
  let calls = 0;
  const fetchImpl: typeof fetch = (input) => {
    calls++;
    const url = String(input);
    if (url.startsWith("https://calendar.example.edu")) {
      return Promise.resolve(
        new Response(null, { status: 302, headers: { location: "https://sneaky.example.edu/e.ics" } }),
      );
    }
    throw new Error("the sneaky host must never be fetched");
  };
  await assertRejects(() => guardedFetch("https://calendar.example.edu/e.ics", { fetchImpl, resolveDns }));
  assertEquals(calls, 1, "the redirect target must be refused before it is ever fetched");
});

Deno.test("more redirect hops than the bound is refused", async () => {
  const resolveDns: ResolveDns = () => Promise.resolve(["93.184.216.34"]);
  let hop = 0;
  const fetchImpl: typeof fetch = () => {
    hop++;
    return Promise.resolve(
      new Response(null, { status: 302, headers: { location: `https://calendar.example.edu/${hop}` } }),
    );
  };
  await assertRejects(() =>
    guardedFetch("https://calendar.example.edu/e.ics", { fetchImpl, resolveDns, maxHops: 2 })
  );
  assertEquals(hop, 3, "the initial fetch plus exactly maxHops redirects, then refused");
});

Deno.test("a body over the cap is cancelled without being read whole", async () => {
  let cancelled = false;
  const resolveDns: ResolveDns = () => Promise.resolve(["93.184.216.34"]);
  const stream = new ReadableStream<Uint8Array>({
    pull(controller) {
      controller.enqueue(new Uint8Array(1024));
    },
    cancel() {
      cancelled = true;
    },
  });
  const fetchImpl: typeof fetch = () => Promise.resolve(new Response(stream, { status: 200 }));
  await assertRejects(() =>
    guardedFetch("https://calendar.example.edu/e.ics", { fetchImpl, resolveDns, maxBytes: 2048 })
  );
  assertEquals(cancelled, true);
});

Deno.test("a slow body is aborted at its timeout", async () => {
  const resolveDns: ResolveDns = () => Promise.resolve(["93.184.216.34"]);
  const fetchImpl: typeof fetch = (_input, init) =>
    new Promise((resolve, reject) => {
      const timer = setTimeout(() => resolve(new Response("late", { status: 200 })), 500);
      init?.signal?.addEventListener("abort", () => {
        clearTimeout(timer);
        reject(new DOMException("aborted", "AbortError"));
      });
    });
  await assertRejects(() =>
    guardedFetch("https://calendar.example.edu/e.ics", { fetchImpl, resolveDns, timeoutMs: 10 })
  );
});
