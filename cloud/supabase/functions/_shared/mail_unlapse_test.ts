// `unlapseAccount` over an injected `fetch` that answers itself, so nothing leaves the process. Every id
// here is invented.
import { assertEquals, assertRejects } from "@std/assert";
import { unlapseAccount } from "./mail_unlapse.ts";

const ACCT = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";

function rest(status: number) {
  const seen: Array<{ method: string; url: string; body: string; prefer: string | null }> = [];
  const fetch = (input: string | URL | Request, init?: RequestInit) => {
    seen.push({
      method: init?.method ?? "GET",
      url: String(input),
      body: String(init?.body),
      prefer: new Headers(init?.headers).get("prefer"),
    });
    return Promise.resolve(new Response(status === 204 ? null : "an error that echoes a value", { status }));
  };
  return {
    seen,
    rest: {
      url: "http://127.0.0.1:9",
      serviceKey: "test-not-a-key",
      fetch: fetch as typeof globalThis.fetch,
    },
  };
}

Deno.test("unlapse_moves_only_the_accounts_lapsed_rows_split_on_the_canary", async () => {
  const { seen, rest: r } = rest(204);
  await unlapseAccount(r, ACCT);
  const base = `http://127.0.0.1:9/rest/v1/mail_addresses?account_id=eq.${ACCT}&status=eq.lapsed`;
  assertEquals(seen, [
    {
      method: "PATCH",
      url: `${base}&last_canary_ok_at=not.is.null`,
      body: '{"status":"quiet"}',
      prefer: "return=minimal",
    },
    {
      method: "PATCH",
      url: `${base}&last_canary_ok_at=is.null`,
      body: '{"status":"pending"}',
      prefer: "return=minimal",
    },
  ]);
});

Deno.test("a_refused_unlapse_throws_a_fixed_message_and_stops", async () => {
  const { seen, rest: r } = rest(500);
  // The message names the step and the status only, never the database's answer.
  await assertRejects(() => unlapseAccount(r, ACCT), Error, "store unlapse 500");
  assertEquals(seen.length, 1);
});
