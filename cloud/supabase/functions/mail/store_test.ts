// The lapsed-mailbox fix (2026-10-19): `covered`'s write as the wire sends it, over an injected `fetch`
// that answers itself, so nothing leaves the process. Every id here is invented.
import { assertEquals } from "@std/assert";
import { storeOver } from "./store.ts";

const ACCT = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const ADDR = "11111111-1111-4111-8111-111111111111";
const BASE = "http://127.0.0.1:9/rest/v1/";

function recording(changed: number) {
  const seen: Array<{ method: string; path: string; body: unknown; prefer: string | null }> = [];
  const fetch = (input: string | URL | Request, init?: RequestInit) => {
    seen.push({
      method: init?.method ?? "GET",
      path: String(input).slice(BASE.length),
      body: init?.body === undefined ? undefined : JSON.parse(String(init.body)),
      prefer: new Headers(init?.headers).get("prefer"),
    });
    const ids = Array.from({ length: changed }, () => ({ id: ADDR }));
    return Promise.resolve(new Response(JSON.stringify(ids), { status: 200 }));
  };
  const store = storeOver({
    url: "http://127.0.0.1:9",
    serviceKey: "test-not-a-key",
    fetch: fetch as typeof globalThis.fetch,
  });
  return { seen, store };
}

Deno.test("covered_patches_only_the_stamp_given_on_a_mailbox_no_longer_lapsed", async () => {
  const stamp = "2026-09-28T08:15:00.123456+00:00";
  const { seen, store } = recording(1);
  assertEquals(await store.clearLapsed(ACCT, ADDR, stamp), true);
  // The stamp goes through whole and encoded (its `+` would read as a space), compared by Postgres as an
  // instant; a newer lapse's stamp, or a mailbox still lapsed, matches nothing.
  assertEquals(seen, [{
    method: "PATCH",
    path: `mail_addresses?id=eq.${ADDR}&account_id=eq.${ACCT}` +
      `&lapsed_since=eq.2026-09-28T08%3A15%3A00.123456%2B00%3A00&status=not.in.(lapsed,revoked)&select=id`,
    body: { lapsed_since: null },
    prefer: "return=representation",
  }]);
  // No row changed: false.
  assertEquals(await recording(0).store.clearLapsed(ACCT, ADDR, stamp), false);
});
