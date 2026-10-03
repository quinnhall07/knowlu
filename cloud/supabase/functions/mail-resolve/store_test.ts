// Email plan T4.5: the wire store's requests are scoped to the account and its answer write is a
// compare-and-set; the charges are `mail_resolve`'s. Every request goes to an injected `fetch` or `Db` that
// answers itself, so nothing leaves the process.
import { assert, assertEquals, assertRejects } from "@std/assert";
import type { Db } from "../_shared/judge_db.ts";
import { MAX_CLAIMS } from "./handler.ts";
import { resolveCharges, storeOver } from "./store.ts";

const ACCT = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const uid = (n: number) => `mail:${n.toString(16).padStart(32, "0")}`;

function recording(answer: () => Response) {
  const seen: Array<{ method: string; url: string; body: string | null; prefer: string | null }> = [];
  const fetch = (input: string | URL | Request, init?: RequestInit) => {
    const headers = new Headers(init?.headers);
    seen.push({
      method: init?.method ?? "GET",
      url: decodeURIComponent(String(input)),
      body: typeof init?.body === "string" ? init.body : null,
      prefer: headers.get("prefer"),
    });
    return Promise.resolve(answer());
  };
  const store = storeOver({
    url: "http://127.0.0.1:9",
    serviceKey: "test-not-a-key",
    fetch: fetch as typeof globalThis.fetch,
  });
  return { seen, store };
}

Deno.test("the_rows_read_is_the_accounts_own_and_short", async () => {
  const { seen, store } = recording(() => new Response("[]", { status: 200 }));
  await store.rows(ACCT, Array.from({ length: MAX_CLAIMS }, (_, i) => uid(i + 1)));
  assertEquals(seen.length, 1);
  assert(seen[0].url.length <= 6 * 1024);
  assert(seen[0].url.includes(`/rest/v1/mail_claims?account_id=eq.${ACCT}&uid=in.(${uid(1)},`));
  assert(seen[0].url.endsWith("&select=id,uid,claims&order=id.asc"));
  // Nothing to read, no request.
  const empty = recording(() => new Response("[]", { status: 200 }));
  assertEquals(await empty.store.rows(ACCT, []), []);
  assertEquals(empty.seen, []);
});

Deno.test("an_answer_lands_only_while_each_claim_it_fills_is_unanswered", async () => {
  const claims = [{ kind: "change", resolution: { target_id: "n-1" } }, { kind: "change", resolution: null }];
  const won = recording(() => new Response(JSON.stringify([{ id: 42 }]), { status: 200 }));
  assertEquals(await won.store.answer(ACCT, 42, [0, 1], claims), true);
  const [patch] = won.seen;
  assertEquals(patch.method, "PATCH");
  assertEquals(
    patch.url,
    `http://127.0.0.1:9/rest/v1/mail_claims?id=eq.42&account_id=eq.${ACCT}` +
      "&claims->0->>resolution=is.null&claims->1->>resolution=is.null&select=id",
  );
  assertEquals(JSON.parse(patch.body ?? ""), { claims });
  assertEquals(patch.prefer, "return=representation");
  // No row matched: another call answered first.
  const lost = recording(() => new Response("[]", { status: 200 }));
  assertEquals(await lost.store.answer(ACCT, 42, [1], claims), false);
  // A database failure throws a fixed message and never reads the body.
  const broke = recording(() => new Response("row data", { status: 500 }));
  await assertRejects(() => broke.store.answer(ACCT, 42, [1], claims), Error, "store answer 500");
});

Deno.test("the_charges_are_mail_resolves_and_an_unreachable_one_refuses", async () => {
  const calls: Array<[string, Record<string, unknown>]> = [];
  let fail = false;
  const db = {
    rpc: (name: string, args: Record<string, unknown>) => {
      calls.push([name, args]);
      return fail ? Promise.reject(new Error("down")) : Promise.resolve(true);
    },
  } as unknown as Db;
  const charges = resolveCharges(db);
  assertEquals(await charges.withinBudget(ACCT), true);
  assertEquals(await charges.charge(ACCT), true);
  await charges.recordTokens(ACCT, 300, 60);
  assertEquals(calls, [
    ["enforce_budget", { p_account: ACCT, p_ceiling: 2 }],
    ["charge_call", { p_account: ACCT, p_kind: "mail_resolve", p_cap: 20 }],
    ["record_tokens", { p_account: ACCT, p_kind: "mail_resolve", p_in: 300, p_out: 60 }],
  ]);
  fail = true;
  assertEquals(await charges.withinBudget(ACCT), false);
  assertEquals(await charges.charge(ACCT), false);
  await charges.recordTokens(ACCT, 1, 1); // never throws
});
