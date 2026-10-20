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
  const last = "2026-10-01T10:00:00.456+00:00";
  const { seen, store } = recording(1);
  assertEquals(await store.clearLapsed(ACCT, ADDR, stamp, last), true);
  // Both stamps go through whole and encoded (a `+` would read as a space), compared by Postgres as
  // instants; a newer lapse (`last_lapsed_at` moved), or a mailbox still lapsed, matches nothing (review
  // of the lapsed fix, #1).
  assertEquals(seen, [{
    method: "PATCH",
    path: `mail_addresses?id=eq.${ADDR}&account_id=eq.${ACCT}` +
      `&lapsed_since=eq.2026-09-28T08%3A15%3A00.123456%2B00%3A00` +
      `&last_lapsed_at=eq.2026-10-01T10%3A00%3A00.456%2B00%3A00&status=not.in.(lapsed,revoked)&select=id`,
    body: { lapsed_since: null },
    prefer: "return=representation",
  }]);
  // No row changed: false.
  assertEquals(await recording(0).store.clearLapsed(ACCT, ADDR, stamp, last), false);
});

Deno.test("rotate_writes_rotated_from_in_the_same_guarded_patch", async () => {
  // Email follow-ups T4 (Q4, 2026-10-20): the status before the Rotate goes in the one PATCH that retires
  // the old token, still filtered on the account, the id and the token Rotate read (a concurrent Rotate
  // matches nothing). Every value here is invented.
  const until = "2026-10-27T12:00:00.000Z";
  const { seen, store } = recording(1);
  const next = {
    tokenHmac: "new-hmac",
    tokenCiphertext: "new-ciphertext",
    tokenIv: "new-iv",
    retiredUntil: until,
    status: "pending",
    rotatedFrom: "active",
    unboundOwnSinceLive: 0 as const,
    unboundForeignSinceLive: 0 as const,
  };
  assertEquals(await store.rotate(ACCT, ADDR, "old-hmac", next), true);
  assertEquals(seen, [{
    method: "PATCH",
    path: `mail_addresses?id=eq.${ADDR}&account_id=eq.${ACCT}&token_hmac=eq.old-hmac&select=id`,
    body: {
      token_hmac: "new-hmac",
      token_ciphertext: "new-ciphertext",
      token_iv: "new-iv",
      retired_token_hmac: "old-hmac",
      retired_until: until,
      rotated_from: "active",
      unbound_own_since_live: 0,
      unbound_foreign_since_live: 0,
      status: "pending",
    },
    prefer: "return=representation",
  }]);
  // A provider whose status Rotate leaves alone records its prior status too, and writes no status.
  const other = recording(1);
  await other.store.rotate(ACCT, ADDR, "old-hmac", { ...next, status: null, rotatedFrom: "quiet" });
  const body = other.seen[0].body as Record<string, unknown>;
  assertEquals([body.rotated_from, Object.hasOwn(body, "status")], ["quiet", false]);
  // The token moved meanwhile: nothing changed, false.
  assertEquals(await recording(0).store.rotate(ACCT, ADDR, "old-hmac", next), false);
});
