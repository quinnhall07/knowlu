import { assert, assertEquals } from "@std/assert";
import { hmacHex } from "../_shared/stripe.ts";
import { accountIdFromEvent, entitlementFromSubscription, handle } from "./handler.ts";

const SECRET = "whsec_test_only_not_a_real_key";

Deno.test("a Stripe subscription becomes exactly the three fields the device reads", () => {
  assertEquals(
    entitlementFromSubscription({
      status: "trialing",
      current_period_end: 1_760_000_000,
      items: { data: [{ price: { id: "price_monthly", recurring: { interval: "month" } } }] },
    }),
    { plan: "monthly", status: "trialing", current_period_end: "2025-10-09T08:53:20.000Z" },
  );
  // A paused collection (the June-August window, R3) leaves Stripe's own status `active`, so the
  // student keeps the product through the summer and is simply not charged. That is the whole
  // mechanism, and this assertion is what stops a later refactor from "fixing" it.
  assertEquals(
    entitlementFromSubscription({
      status: "active",
      pause_collection: { behavior: "void" },
      current_period_end: 1_760_000_000,
      items: { data: [{ price: { id: "price_year", recurring: { interval: "year" } } }] },
    }).status,
    "active",
  );
  assertEquals(
    entitlementFromSubscription({ status: "unpaid", current_period_end: null, items: { data: [] } }).status,
    "past_due",
  );
  assertEquals(
    entitlementFromSubscription({
      status: "incomplete_expired",
      current_period_end: null,
      items: { data: [] },
    }).status,
    "canceled",
  );
  assertEquals(
    entitlementFromSubscription({ status: "wat", current_period_end: null, items: { data: [] } }).status,
    "none",
  );
});

Deno.test("the account id comes from the metadata, then the client reference, then nothing", () => {
  assertEquals(accountIdFromEvent({ data: { object: { metadata: { account_id: "acc-1" } } } }), "acc-1");
  assertEquals(accountIdFromEvent({ data: { object: { client_reference_id: "acc-2" } } }), "acc-2");
  assertEquals(accountIdFromEvent({ data: { object: {} } }), null);
});

Deno.test("an unsigned or badly signed webhook is 400 and writes nothing", async () => {
  let wrote = false;
  const body = JSON.stringify({ type: "customer.subscription.updated", data: { object: {} } });
  const res = await handle(
    new Request("http://127.0.0.1:1/", {
      method: "POST",
      body,
      headers: { "stripe-signature": "t=1,v1=deadbeef" },
    }),
    {
      secret: SECRET,
      nowSeconds: () => 1,
      writeEntitlement: () => {
        wrote = true;
        return Promise.resolve();
      },
      fetchSubscription: () => Promise.resolve(null),
    },
  );
  assertEquals(res.status, 400);
  assert(!wrote, "a bad signature must never reach the write");
});

Deno.test("no stripe-signature header at all is 400 and writes nothing", async () => {
  let wrote = false;
  const body = JSON.stringify({ type: "customer.subscription.updated", data: { object: {} } });
  const res = await handle(
    new Request("http://127.0.0.1:1/", { method: "POST", body }),
    {
      secret: SECRET,
      nowSeconds: () => 1,
      writeEntitlement: () => {
        wrote = true;
        return Promise.resolve();
      },
      fetchSubscription: () => Promise.resolve(null),
    },
  );
  assertEquals(res.status, 400);
  assertEquals(await res.json(), { error: "signature does not verify" });
  assert(!wrote, "no signature header must never reach the write");
});

Deno.test("a signed subscription event writes the entitlement, once, keyed to the account", async () => {
  const written: unknown[] = [];
  const event = {
    type: "customer.subscription.updated",
    data: {
      object: {
        metadata: { account_id: "acc-1" },
        status: "active",
        current_period_end: 1_760_000_000,
        items: { data: [{ price: { id: "price_monthly", recurring: { interval: "month" } } }] },
      },
    },
  };
  const body = JSON.stringify(event);
  const t = 1_700_000_000;
  const sig = await hmacHex(SECRET, `${t}.${body}`);
  const res = await handle(
    new Request("http://127.0.0.1:1/", {
      method: "POST",
      body,
      headers: { "stripe-signature": `t=${t},v1=${sig}` },
    }),
    {
      secret: SECRET,
      nowSeconds: () => t,
      writeEntitlement: (accountId, row) => {
        written.push([accountId, row]);
        return Promise.resolve();
      },
      fetchSubscription: () => Promise.resolve(null),
    },
  );
  assertEquals(res.status, 200);
  assertEquals(written, [["acc-1", {
    plan: "monthly",
    status: "active",
    current_period_end: "2025-10-09T08:53:20.000Z",
  }]]);
});

Deno.test("under the pinned API version, current_period_end comes off the subscription item when the top-level field is absent", async () => {
  const written: unknown[] = [];
  const event = {
    type: "customer.subscription.updated",
    data: {
      object: {
        metadata: { account_id: "acc-1" },
        status: "active",
        items: {
          data: [{
            current_period_end: 1_760_000_000,
            price: { id: "price_monthly", recurring: { interval: "month" } },
          }],
        },
      },
    },
  };
  const body = JSON.stringify(event);
  const t = 1_700_000_000;
  const sig = await hmacHex(SECRET, `${t}.${body}`);
  const res = await handle(
    new Request("http://127.0.0.1:1/", {
      method: "POST",
      body,
      headers: { "stripe-signature": `t=${t},v1=${sig}` },
    }),
    {
      secret: SECRET,
      nowSeconds: () => t,
      writeEntitlement: (accountId, row) => {
        written.push([accountId, row]);
        return Promise.resolve();
      },
      fetchSubscription: () => Promise.resolve(null),
    },
  );
  assertEquals(res.status, 200);
  assertEquals(written, [["acc-1", {
    plan: "monthly",
    status: "active",
    current_period_end: "2025-10-09T08:53:20.000Z",
  }]]);
});

Deno.test("an event type we do not handle is 200 and a no-op — Stripe must not retry it forever", async () => {
  const body = JSON.stringify({ type: "customer.created", data: { object: {} } });
  const t = 1_700_000_000;
  const sig = await hmacHex(SECRET, `${t}.${body}`);
  let wrote = false;
  const res = await handle(
    new Request("http://127.0.0.1:1/", {
      method: "POST",
      body,
      headers: { "stripe-signature": `t=${t},v1=${sig}` },
    }),
    {
      secret: SECRET,
      nowSeconds: () => t,
      writeEntitlement: () => {
        wrote = true;
        return Promise.resolve();
      },
      fetchSubscription: () => Promise.resolve(null),
    },
  );
  assertEquals(res.status, 200);
  assert(!wrote);
});
