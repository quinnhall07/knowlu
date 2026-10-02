// The caps' two pinned numbers, so a future edit to either one is a deliberate diff, not a drift
// nobody notices (whole-branch review M1).
import { assertEquals } from "@std/assert";
import { DAILY_CAP, MONTHLY_CEILING_USD } from "./judge_caps.ts";

Deno.test("MONTHLY_CEILING_USD is pinned at $2.00 (the provider swap plan's runaway-guard constraint)", () => {
  assertEquals(MONTHLY_CEILING_USD, 2.0);
});

Deno.test("DAILY_CAP is pinned per kind", () => {
  assertEquals(DAILY_CAP, { task: 60, event: 80, email: 120 });
});

// F1 (live proof 2026-10-02): the live store's refund and its per-invocation breaker.
import { BREAKER_FAILURES, capStore } from "./judge_caps.ts";
import type { Db } from "./judge_db.ts";

class FakeDb implements Db {
  calls: Array<[string, Record<string, unknown>]> = [];
  constructor(private readonly fail = false) {}
  select(): Promise<unknown[]> {
    return Promise.resolve([]);
  }
  insert(): Promise<Record<string, unknown> | null> {
    return Promise.resolve(null);
  }
  update(): Promise<void> {
    return Promise.resolve();
  }
  rpc(fn: string, args: Record<string, unknown>): Promise<unknown> {
    this.calls.push([fn, args]);
    return this.fail ? Promise.reject(new Error("PostgREST 503")) : Promise.resolve(true);
  }
}

Deno.test("F1: refund calls refund_call with the account, the kind and that kind's cap", async () => {
  const db = new FakeDb();
  await capStore(db).refund("acct-1", "event");
  assertEquals(db.calls, [["refund_call", { p_account: "acct-1", p_kind: "event", p_cap: DAILY_CAP.event }]]);
});

Deno.test("F1: a refund that cannot reach the database never throws", async () => {
  await capStore(new FakeDb(true)).refund("acct-1", "task");
});

Deno.test("F1: the breaker opens after BREAKER_FAILURES consecutive failed calls, and a reply closes it", async () => {
  const caps = capStore(new FakeDb());
  for (let i = 0; i < BREAKER_FAILURES; i += 1) {
    assertEquals(caps.tripped(), false, `still closed after ${i} failures`);
    await caps.refund("acct-1", "email");
  }
  assertEquals(caps.tripped(), true);
  await caps.recordTokens("acct-1", "email", 300, 60);
  assertEquals(caps.tripped(), false, "a reply means the provider is back");
});

Deno.test("F1: the breaker is per store, so one invocation's outage never trips the next", async () => {
  const db = new FakeDb();
  const first = capStore(db);
  for (let i = 0; i < BREAKER_FAILURES; i += 1) await first.refund("acct-1", "task");
  assertEquals(first.tripped(), true);
  assertEquals(capStore(db).tripped(), false);
});

Deno.test("F1: BREAKER_FAILURES is pinned at 3", () => {
  assertEquals(BREAKER_FAILURES, 3);
});
