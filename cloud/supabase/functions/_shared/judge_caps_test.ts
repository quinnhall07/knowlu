// The caps' two pinned numbers, so a future edit to either one is a deliberate diff, not a drift
// nobody notices (whole-branch review M1).
import { assert, assertEquals } from "@std/assert";
import { DAILY_CAP, MAIL_DAILY_CAP, MONTHLY_CEILING_USD } from "./judge_caps.ts";

Deno.test("MONTHLY_CEILING_USD is pinned at $2.00 (the provider swap plan's runaway-guard constraint)", () => {
  assertEquals(MONTHLY_CEILING_USD, 2.0);
});

Deno.test("DAILY_CAP is pinned per kind", () => {
  assertEquals(DAILY_CAP, { task: 60, event: 80, email: 120 });
});

// Email plan T3c (spec §5.1, plan P6): the mail kinds' daily caps sit in their own record, keyed by
// `MailKind`, so `DAILY_CAP` and every `Record<Kind, …>` stay as they were.
Deno.test("mail_daily_caps_are_120_for_claims_and_20_for_resolve", () => {
  assertEquals(MAIL_DAILY_CAP.mail_claim, 120);
  assertEquals(MAIL_DAILY_CAP.mail_resolve, 20);
  // The digest (T16, optional) is not built: no call is allowed until its own task sets a cap.
  assertEquals(MAIL_DAILY_CAP, { mail_claim: 120, mail_resolve: 20, mail_digest: 0 });
});

Deno.test("the_existing_kinds_caps_are_unchanged", () => {
  assertEquals(DAILY_CAP, { task: 60, event: 80, email: 120 });
  assertEquals(Object.keys(DAILY_CAP).sort(), ["email", "event", "task"]);
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

// L3, the first-day event allowance (spec 2026-10-06 §3 step 3, §5, §6; plan T2a): the store tells
// a refusal from an error, and can charge and refund the one-time event allowance.
import { type ChargeOutcome, EVENT_ALLOWANCE_CALLS, EVENT_ALLOWANCE_DAYS } from "./judge_caps.ts";

type Script = { value: unknown } | { reject: string };

/** A `Db` whose `rpc` answers per function name: a value to resolve, or a rejection. */
class ScriptedDb implements Db {
  calls: Array<[string, Record<string, unknown>]> = [];
  constructor(private readonly script: Record<string, Script>) {}
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
    const answer = this.script[fn];
    if (answer === undefined) return Promise.reject(new Error(`unscripted rpc ${fn}`));
    return "reject" in answer ? Promise.reject(new Error(answer.reject)) : Promise.resolve(answer.value);
  }
}

const CHARGE_CALL_CASES: Array<[string, Script, ChargeOutcome, boolean]> = [
  ["true", { value: true }, "charged", true],
  ["false", { value: false }, "refused", false],
  ["a rejection", { reject: "PostgREST 503" }, "error", false],
  ["null", { value: null }, "error", false],
];

Deno.test("chargeOutcome tells a charge, a refusal and an error apart", async () => {
  for (const [label, script, outcome] of CHARGE_CALL_CASES) {
    const db = new ScriptedDb({ charge_call: script });
    assertEquals(
      await capStore(db).chargeOutcome("acct-1", "event"),
      outcome,
      `charge_call answering ${label}`,
    );
    assertEquals(db.calls, [["charge_call", {
      p_account: "acct-1",
      p_kind: "event",
      p_cap: DAILY_CAP.event,
    }]]);
  }
});

Deno.test("charge still folds a refusal and an error into false", async () => {
  for (const [label, script, , charged] of CHARGE_CALL_CASES) {
    const db = new ScriptedDb({ charge_call: script });
    assertEquals(await capStore(db).charge("acct-1", "task"), charged, `charge_call answering ${label}`);
    assertEquals(db.calls, [["charge_call", { p_account: "acct-1", p_kind: "task", p_cap: DAILY_CAP.task }]]);
  }
});

Deno.test("a charge_call error never counts toward the breaker", async () => {
  const caps = capStore(new ScriptedDb({ charge_call: { reject: "PostgREST 503" } }));
  for (let i = 0; i < BREAKER_FAILURES; i += 1) {
    assertEquals(await caps.chargeOutcome("acct-1", "event"), "error");
  }
  assertEquals(caps.tripped(), false, "a store that cannot charge is not a model that failed");
});

Deno.test("chargeAllowance calls charge_judge_allowance with the account and the kind", async () => {
  const db = new ScriptedDb({ charge_judge_allowance: { value: true } });
  assertEquals(await capStore(db).chargeAllowance("acct-1", "event"), true);
  assertEquals(db.calls, [["charge_judge_allowance", { p_account: "acct-1", p_kind: "event" }]]);
});

Deno.test("a chargeAllowance that cannot reach the database refuses", async () => {
  assertEquals(await capStore(new FakeDb(true)).chargeAllowance("acct-1", "event"), false);
});

Deno.test("refundAllowance calls refund_judge_allowance and never throws", async () => {
  const db = new FakeDb();
  await capStore(db).refundAllowance("acct-1", "event");
  assertEquals(db.calls, [["refund_judge_allowance", { p_account: "acct-1", p_kind: "event" }]]);
  await capStore(new FakeDb(true)).refundAllowance("acct-1", "event");
});

Deno.test("the breaker counts refundAllowance failures too", async () => {
  const caps = capStore(new FakeDb());
  for (let i = 0; i < BREAKER_FAILURES; i += 1) {
    assertEquals(caps.tripped(), false, `still closed after ${i} failures`);
    if (i % 2 === 0) await caps.refundAllowance("acct-1", "event");
    else await caps.refund("acct-1", "event");
  }
  assertEquals(caps.tripped(), true);
  await caps.recordTokens("acct-1", "event", 300, 60);
  assertEquals(caps.tripped(), false, "a reply means the provider is back");
});

Deno.test("EVENT_ALLOWANCE_CALLS and _DAYS are the numbers the migration opens with", async () => {
  // Spec test 11: the constants are mirrors of the SQL, never sent to it, so they are pinned to its text.
  const dir = new URL("../../migrations/", import.meta.url);
  const names = [...Deno.readDirSync(dir)].map((e) => e.name).filter((n) =>
    /^\d{14}_event_allowance\.sql$/.test(n)
  );
  assertEquals(names.length, 1, "exactly one event_allowance migration");
  const sql = (await Deno.readTextFile(new URL(names[0], dir))).replace(/\s+/g, " ");
  const fn = (name: string) => {
    const body = new RegExp(`create or replace function public\\.${name}\\(.*?\\$\\$(.*?)\\$\\$;`).exec(sql);
    assert(body !== null, `${name} is defined`);
    return body[1];
  };
  const opens = /now\(\) \+ interval '(\d+) days', (\d+) - 1, 0\)/.exec(fn("charge_judge_allowance"));
  assert(opens !== null, "charge_judge_allowance opens with now() + interval 'N days' and N - 1 left");
  assertEquals(Number(opens[1]), EVENT_ALLOWANCE_DAYS);
  assertEquals(Number(opens[2]), EVENT_ALLOWANCE_CALLS);
  const refund = fn("refund_judge_allowance");
  const left = /calls_left < (\d+)/.exec(refund);
  const refunds = /refunds < (\d+)/.exec(refund);
  assert(left !== null && refunds !== null, "refund_judge_allowance bounds calls_left and refunds");
  assertEquals(Number(left[1]), EVENT_ALLOWANCE_CALLS);
  assertEquals(Number(refunds[1]), EVENT_ALLOWANCE_CALLS);
});
