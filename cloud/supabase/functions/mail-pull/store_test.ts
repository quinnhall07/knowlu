// Email plan T4.4 review: the wire store's requests stay short enough for any gateway in front of
// PostgREST. Every request goes to an injected `fetch` that answers itself, so nothing leaves the process.
import { assert, assertEquals } from "@std/assert";
import { ACK_CAP, PAGE } from "./handler.ts";
import { storeOver } from "./store.ts";

const ACCT = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
/** Well under the ~8 KB request line common gateways accept. */
const MAX_URL = 6 * 1024;
const uid = (n: number) => `mail:${n.toString(16).padStart(32, "0")}`;

function recording() {
  const seen: Array<{ method: string; url: string }> = [];
  const fetch = (input: string | URL | Request, init?: RequestInit) => {
    const method = init?.method ?? "GET";
    seen.push({ method, url: String(input) });
    return Promise.resolve(
      method === "PATCH" ? new Response(null, { status: 204 }) : new Response("[]", { status: 200 }),
    );
  };
  const store = storeOver({
    url: "http://127.0.0.1:9",
    serviceKey: "test-not-a-key",
    fetch: fetch as typeof globalThis.fetch,
  });
  return { seen, store };
}

Deno.test("a_full_ack_is_sent_in_patches_whose_urls_stay_short", async () => {
  const { seen, store } = recording();
  const all = Array.from({ length: ACK_CAP }, (_, i) => uid(1000 + i));
  await store.deliver(ACCT, all, "2026-10-02T12:00:00.000Z");
  assert(seen.length > 1);
  const sent: string[] = [];
  for (const { method, url } of seen) {
    assertEquals(method, "PATCH");
    assert(url.length <= MAX_URL, `a ${url.length}-character URL`);
    // Each part is still scoped to the account and to rows not yet delivered.
    assert(url.includes(`account_id=eq.${ACCT}`));
    assert(url.includes("delivered_at=is.null"));
    const list = decodeURIComponent(url.split("uid=in.(")[1].split(")")[0]);
    sent.push(...list.split(","));
  }
  assertEquals(sent, all);
  // No ack, no request.
  const empty = recording();
  await empty.store.deliver(ACCT, [], "2026-10-02T12:00:00.000Z");
  assertEquals(empty.seen, []);
});

Deno.test("a_full_page_of_claims_is_read_in_a_short_url", async () => {
  const { seen, store } = recording();
  await store.claims(ACCT, Array.from({ length: PAGE }, (_, i) => 9_000_000_000 + i));
  assertEquals(seen.length, 1);
  assert(seen[0].url.length <= MAX_URL, `a ${seen[0].url.length}-character URL`);
});
