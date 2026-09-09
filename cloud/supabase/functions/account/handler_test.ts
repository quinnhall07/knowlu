import { assert, assertEquals } from "@std/assert";
import { Deps, handle } from "./handler.ts";

const NOW = new Date("2026-09-10T12:00:00.000Z");

function deps(over: Partial<Deps> = {}): Deps {
  return {
    verify: (t) => Promise.resolve(t === "good" ? { id: "acc-1", email: "a@example.invalid" } : null),
    requireEntitled: () => Promise.resolve({ account_id: "acc-1" }),
    getAccount: () => Promise.resolve({ email: "a@example.invalid", stripe_customer_id: "cus_1" }),
    getSubscriptionId: () => Promise.resolve("sub_1"),
    stripe: () => Promise.resolve({}),
    purge: () => Promise.resolve(),
    deleteAuthUser: () => Promise.resolve(),
    tombstone: () => Promise.resolve(),
    exportAll: () =>
      Promise.resolve({
        account: {},
        entitlement: null,
        consents: [],
        sources: [],
        telemetry_events: [],
        corrections: [],
        issues: [],
      }),
    hashEmail: (e) => Promise.resolve(`hash(${e})`),
    getSources: () => Promise.resolve([]),
    putSource: () => Promise.resolve(),
    now: () => NOW,
    ...over,
  };
}

const req = (method: string, path: string, body?: unknown, auth = "Bearer good") =>
  new Request(`http://127.0.0.1:1/functions/v1/account${path}`, {
    method,
    headers: { authorization: auth },
    body: body === undefined ? undefined : JSON.stringify(body),
  });

Deno.test("DELETE /account cancels at period end, purges, tombstones, and kills the login LAST", async () => {
  const order: string[] = [];
  const res = await handle(
    req("DELETE", ""),
    deps({
      stripe: (path, form) => {
        order.push(`stripe ${path} ${JSON.stringify(form)}`);
        return Promise.resolve({});
      },
      purge: () => {
        order.push("purge");
        return Promise.resolve();
      },
      tombstone: (h) => {
        order.push(`tombstone ${h}`);
        return Promise.resolve();
      },
      deleteAuthUser: () => {
        order.push("deleteAuthUser");
        return Promise.resolve();
      },
    }),
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { deleted: true });
  assertEquals(order, [
    'stripe /v1/subscriptions/sub_1 {"cancel_at_period_end":"true"}',
    "purge",
    "tombstone hash(a@example.invalid)",
    "deleteAuthUser",
  ]);
});

Deno.test("DELETE /account on an account that never subscribed still deletes everything", async () => {
  let purged = false;
  const res = await handle(
    req("DELETE", ""),
    deps({
      getSubscriptionId: () => Promise.resolve(null),
      stripe: () => {
        throw new Error("Stripe must not be called when there is no subscription");
      },
      purge: () => {
        purged = true;
        return Promise.resolve();
      },
    }),
  );
  assertEquals(res.status, 200);
  assert(purged);
});

Deno.test("GET /account/export hands back every table, keyed and complete", async () => {
  const res = await handle(req("GET", "/export"), deps());
  assertEquals(res.status, 200);
  const body = await res.json();
  assertEquals(Object.keys(body).sort(), [
    "account",
    "consents",
    "corrections",
    "entitlement",
    "exported_at",
    "issues",
    "sources",
    "telemetry_events",
  ]);
  assertEquals(body.exported_at, "2026-09-10T12:00:00.000Z");
});

Deno.test("an unknown path is 404 and an unknown method on a known path is 405", async () => {
  assertEquals((await handle(req("GET", "/nope"), deps())).status, 404);
  assertEquals((await handle(req("POST", "/export"), deps())).status, 405);
});

Deno.test("every route needs a bearer token", async () => {
  for (const [m, p] of [["DELETE", ""], ["GET", "/export"]] as const) {
    const res = await handle(req(m, p, undefined, "Basic nope"), deps()).catch((e) => e as Response);
    assertEquals(res.status, 401, `${m} ${p}`);
  }
});
