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
  const seen: Array<{ method: string; url: string; body?: string }> = [];
  const fetch = (input: string | URL | Request, init?: RequestInit) => {
    const method = init?.method ?? "GET";
    seen.push({
      method,
      url: String(input),
      ...(init?.body === undefined ? {} : { body: String(init.body) }),
    });
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

// 2026-10-19: a lapsed mailbox leaves `lapsed` on its account's first entitled pull.
Deno.test("unlapse_patches_only_the_accounts_lapsed_rows", async () => {
  const { seen, store } = recording();
  await store.unlapse(ACCT);
  const base = `http://127.0.0.1:9/rest/v1/mail_addresses?account_id=eq.${ACCT}&status=eq.lapsed`;
  // One compare-and-set each way: a mailbox a canary ever reached reads `quiet`, one never reached
  // `pending`. Neither touches `lapsed_since`, which only the device's `covered` clears.
  assertEquals(seen, [
    { method: "PATCH", url: `${base}&last_canary_ok_at=not.is.null`, body: '{"status":"quiet"}' },
    { method: "PATCH", url: `${base}&last_canary_ok_at=is.null`, body: '{"status":"pending"}' },
  ]);
});

// Review of the lapsed fix, #1: the pull reads each mailbox's latest lapse with the gap's start.
Deno.test("the_mailbox_read_asks_for_lapsed_since_and_last_lapsed_at", async () => {
  const { seen, store } = recording();
  await store.mailboxes(ACCT);
  assertEquals(seen.length, 1);
  const select = new URL(seen[0].url).searchParams.get("select")!.split(",");
  for (const c of ["lapsed_since", "last_lapsed_at"]) assert(select.includes(c), c);
});
