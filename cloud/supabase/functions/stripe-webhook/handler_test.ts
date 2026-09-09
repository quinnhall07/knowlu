import { assert, assertEquals } from "@std/assert";
import { hmacHex } from "../_shared/stripe.ts";
import { accountIdFromEvent, entitlementFromSubscription, handle } from "./handler.ts";

const SECRET = "whsec_test_only_not_a_real_key";

/** Every test below that reaches the write path needs the same three no-op guards; this is the one
 * place their shape is written down. */
function noopGuards() {
  return {
    seenEvent: () => Promise.resolve(false),
    recordEvent: () => Promise.resolve(),
    currentUpdatedAt: () => Promise.resolve(null as string | null),
  };
}

Deno.test("a Stripe subscription becomes exactly the six fields the row carries", () => {
  assertEquals(
    entitlementFromSubscription({
      status: "trialing",
      items: {
        data: [{
          current_period_end: 1_760_000_000,
          price: { id: "price_monthly", recurring: { interval: "month" } },
        }],
      },
    }),
    {
      plan: "monthly",
      status: "trialing",
      current_period_end: "2025-10-09T08:53:20.000Z",
      stripe_subscription_id: null,
      paused: false,
      started_at: null,
    },
  );
  // A paused collection (the June-August window, R3) leaves Stripe's own status `active`, so the
  // student keeps the product through the summer and is simply not charged. That is the whole
  // mechanism, and this assertion is what stops a later refactor from "fixing" it.
  assertEquals(
    entitlementFromSubscription({
      status: "active",
      id: "sub_1",
      pause_collection: { behavior: "void" },
      start_date: 1_700_000_000,
      items: {
        data: [{
          current_period_end: 1_760_000_000,
          price: { id: "price_year", recurring: { interval: "year" } },
        }],
      },
    }),
    {
      plan: "academic_year",
      status: "active",
      current_period_end: "2025-10-09T08:53:20.000Z",
      stripe_subscription_id: "sub_1",
      paused: true,
      started_at: "2023-11-14T22:13:20.000Z",
    },
  );
  assertEquals(
    entitlementFromSubscription({ status: "unpaid", items: { data: [] } }).status,
    "past_due",
  );
  assertEquals(
    entitlementFromSubscription({ status: "incomplete_expired", items: { data: [] } }).status,
    "canceled",
  );
  assertEquals(
    entitlementFromSubscription({ status: "wat", items: { data: [] } }).status,
    "none",
  );
  // The pre-basil shape: `current_period_end` still lives on the Subscription itself, not the item.
  // An account whose events still carry it that way must keep working.
  assertEquals(
    entitlementFromSubscription({
      status: "active",
      current_period_end: 1_760_000_000,
      items: { data: [{ price: { id: "price_monthly", recurring: { interval: "month" } } }] },
    }).current_period_end,
    "2025-10-09T08:53:20.000Z",
  );
});

Deno.test("the account id comes from a subscription's own metadata, the basil invoice location, the older invoice location, the client reference, or nothing", () => {
  assertEquals(accountIdFromEvent({ data: { object: { metadata: { account_id: "acc-1" } } } }), "acc-1");
  assertEquals(
    accountIdFromEvent({
      data: { object: { parent: { subscription_details: { metadata: { account_id: "acc-2" } } } } },
    }),
    "acc-2",
  );
  assertEquals(
    accountIdFromEvent({ data: { object: { subscription_details: { metadata: { account_id: "acc-3" } } } } }),
    "acc-3",
  );
  assertEquals(accountIdFromEvent({ data: { object: { client_reference_id: "acc-4" } } }), "acc-4");
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
      ...noopGuards(),
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
      ...noopGuards(),
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
    id: "evt_1",
    type: "customer.subscription.updated",
    created: 1_700_000_000,
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
      ...noopGuards(),
      writeEntitlement: (accountId, row, updatedAt) => {
        written.push([accountId, row, updatedAt]);
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
    stripe_subscription_id: null,
    paused: false,
    started_at: null,
  }, "2023-11-14T22:13:20.000Z"]]);
});

Deno.test("under the pinned API version, current_period_end comes off the subscription item when the top-level field is absent", async () => {
  const written: unknown[] = [];
  const event = {
    id: "evt_2",
    type: "customer.subscription.updated",
    created: 1_700_000_000,
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
      ...noopGuards(),
      writeEntitlement: (accountId, row, updatedAt) => {
        written.push([accountId, row, updatedAt]);
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
    stripe_subscription_id: null,
    paused: false,
    started_at: null,
  }, "2023-11-14T22:13:20.000Z"]]);
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
      ...noopGuards(),
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

Deno.test("invoice.payment_failed under the pinned API version reaches fetchSubscription and writes", async () => {
  const fetched: string[] = [];
  const written: unknown[] = [];
  const event = {
    id: "evt_invoice_1",
    type: "invoice.payment_failed",
    created: 1_700_000_000,
    data: {
      object: {
        parent: {
          subscription_details: {
            subscription: "sub_9",
            metadata: { account_id: "acc-9" },
          },
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
      ...noopGuards(),
      writeEntitlement: (accountId, row, updatedAt) => {
        written.push([accountId, row, updatedAt]);
        return Promise.resolve();
      },
      fetchSubscription: (id) => {
        fetched.push(id);
        return Promise.resolve({
          id: "sub_9",
          status: "past_due",
          items: { data: [{ price: { id: "price_monthly", recurring: { interval: "month" } } }] },
        });
      },
    },
  );
  assertEquals(res.status, 200);
  assertEquals(fetched, ["sub_9"]);
  assertEquals(written.length, 1);
  assertEquals((written[0] as [string, unknown, string])[0], "acc-9");
});

Deno.test("invoice.payment_failed carrying neither an account id nor a client reference is still ignored", async () => {
  let fetchedAny = false;
  let wrote = false;
  const event = {
    id: "evt_invoice_2",
    type: "invoice.payment_failed",
    created: 1_700_000_000,
    data: { object: {} },
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
      ...noopGuards(),
      writeEntitlement: () => {
        wrote = true;
        return Promise.resolve();
      },
      fetchSubscription: (id) => {
        fetchedAny = true;
        return Promise.resolve({ id });
      },
    },
  );
  assertEquals(res.status, 200);
  assertEquals(await res.clone().json(), { ignored: "no account id on the event" });
  assert(!fetchedAny, "no account id means no subscription fetch");
  assert(!wrote);
});

Deno.test("the same signed event twice writes once", async () => {
  const written: unknown[] = [];
  const seen = new Set<string>();
  const event = {
    id: "evt_dup_1",
    type: "customer.subscription.updated",
    created: 1_700_000_000,
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
  const deps = {
    secret: SECRET,
    nowSeconds: () => t,
    seenEvent: (eventId: string) => Promise.resolve(seen.has(eventId)),
    recordEvent: (eventId: string) => {
      seen.add(eventId);
      return Promise.resolve();
    },
    currentUpdatedAt: () => Promise.resolve(null as string | null),
    writeEntitlement: (accountId: string, row: unknown, updatedAt: string) => {
      written.push([accountId, row, updatedAt]);
      return Promise.resolve();
    },
    fetchSubscription: () => Promise.resolve(null),
  };
  const req = () =>
    new Request("http://127.0.0.1:1/", {
      method: "POST",
      body,
      headers: { "stripe-signature": `t=${t},v1=${sig}` },
    });
  const first = await handle(req(), deps);
  const second = await handle(req(), deps);
  assertEquals(first.status, 200);
  assertEquals(await first.clone().json(), { ok: true });
  assertEquals(second.status, 200);
  assertEquals(await second.clone().json(), { duplicate: "evt_dup_1" });
  assertEquals(written.length, 1, "a duplicate delivery must never write twice");
});

Deno.test("an event whose created precedes the row's updated_at writes nothing", async () => {
  let wrote = false;
  let recorded: [string, string, string] | null = null;
  const event = {
    id: "evt_stale_1",
    type: "customer.subscription.updated",
    // Older than the row's own updated_at below.
    created: 1_600_000_000,
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
      seenEvent: () => Promise.resolve(false),
      recordEvent: (eventId, type, createdIso) => {
        recorded = [eventId, type, createdIso];
        return Promise.resolve();
      },
      // Newer than the event's `created` (1_600_000_000 -> 2020-09-13...): the row already moved on.
      // In PostgREST's own `timestamptz` text form (`+00:00`, not `.000Z`) — the shape the stale
      // guard must compare as an instant, not as a string.
      currentUpdatedAt: () => Promise.resolve("2025-01-01T00:00:00+00:00"),
      writeEntitlement: () => {
        wrote = true;
        return Promise.resolve();
      },
      fetchSubscription: () => Promise.resolve(null),
    },
  );
  assertEquals(res.status, 200);
  assertEquals(await res.clone().json(), { stale: "evt_stale_1" });
  assert(!wrote, "a stale event must never overwrite newer state");
  assertEquals(recorded, ["evt_stale_1", "customer.subscription.updated", "2020-09-13T12:26:40.000Z"]);
});

Deno.test("an event whose created exactly equals the row's updated_at is also stale", async () => {
  let wrote = false;
  const event = {
    id: "evt_stale_2",
    type: "customer.subscription.updated",
    created: 1_700_000_000,
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
      seenEvent: () => Promise.resolve(false),
      recordEvent: () => Promise.resolve(),
      // PostgREST's own text form of the SAME instant as the event's `created`
      // (1_700_000_000 -> 2023-11-14T22:13:20Z): `+00:00`, not `.000Z`. A byte-for-byte string
      // compare would call this NOT stale (`'+' < '.'`); comparing as instants correctly calls it
      // stale — the row is not older, so a late-arriving duplicate must not overwrite it.
      currentUpdatedAt: () => Promise.resolve("2023-11-14T22:13:20+00:00"),
      writeEntitlement: () => {
        wrote = true;
        return Promise.resolve();
      },
      fetchSubscription: () => Promise.resolve(null),
    },
  );
  assertEquals(res.status, 200);
  assertEquals(await res.clone().json(), { stale: "evt_stale_2" });
  assert(!wrote);
});
