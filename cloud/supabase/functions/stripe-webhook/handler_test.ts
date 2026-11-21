import { assert, assertEquals } from "@std/assert";
import { hmacHex } from "../_shared/stripe.ts";
import {
  type CatalogDef,
  priceVariable,
  resolveCatalog,
  type ResolvedCatalog,
  TERMS_VERSION,
  WORDS,
} from "../_shared/price_catalog.ts";
import {
  accountIdFromEvent,
  chargeDate,
  type Deps,
  entitlementFromSubscription,
  handle,
  type Mail,
} from "./handler.ts";

const SECRET = "whsec_test_only_not_a_real_key";

// Every test builds its own catalog from a table here, through the catalog's resolve helper (note
// D13); none reads the real `SELLING`. One `KEPT` entry is the `stay` case: same key, other cents.
const TABLE: CatalogDef = {
  termsVersion: TERMS_VERSION,
  selling: [{ key: "yearly", cents: 8437 }, { key: "four_weekly", cents: 1299 }],
  kept: [{ key: "four_weekly", cents: 1199, rule: "stay" }],
  words: WORDS,
};
const YEARLY_ID = "price_test_yearly";
const FOUR_WEEKLY_ID = "price_test_four_weekly";
const KEPT_ID = "price_test_kept_four_weekly";

/** The test's invented environment: each entry's derived variable holds an invented id. */
function catalogOf(def: CatalogDef = TABLE, unset: string[] = []): ResolvedCatalog {
  const ids = new Map<string, string>([
    [priceVariable({ key: "yearly", cents: 8437 }), YEARLY_ID],
    [priceVariable({ key: "four_weekly", cents: 1299 }), FOUR_WEEKLY_ID],
    [priceVariable({ key: "four_weekly", cents: 1199 }), KEPT_ID],
  ]);
  // An invented entry at other cents takes the id of the same key's table entry, so the same Stripe
  // id can be made to mean another amount.
  for (const e of def.selling) {
    if (!ids.has(priceVariable(e))) {
      ids.set(priceVariable(e), e.key === "yearly" ? YEARLY_ID : FOUR_WEEKLY_ID);
    }
  }
  return resolveCatalog(def, (name) => (unset.includes(name) ? undefined : ids.get(name)));
}

/** Every test below that reaches the write path needs the same three no-op guards; this is the one
 * place their shape is written down. */
function noopGuards() {
  return {
    seenEvent: () => Promise.resolve(false),
    recordEvent: () => Promise.resolve(),
    currentUpdatedAt: () => Promise.resolve(null as string | null),
    accountExists: () => Promise.resolve(true),
  };
}

/** The catalog, and a mailer that must not be reached by any event but `trial_will_end`. */
function billing(catalog: ResolvedCatalog = catalogOf()) {
  return {
    catalog,
    accountEmail: () => Promise.resolve("a@example.invalid" as string | null),
    sendEmail: (_m: Mail): Promise<void> => Promise.reject(new Error("no mail on this path")),
  };
}

Deno.test("the_plan_comes_from_the_price_id", () => {
  const catalog = catalogOf();
  const sub = (id: string, recurring: Record<string, unknown>) => ({
    status: "active",
    items: { data: [{ current_period_end: 1_760_000_000, price: { id, recurring } }] },
  });
  // Both selling ids map to their keys.
  assertEquals(
    entitlementFromSubscription(sub(YEARLY_ID, { interval: "year", interval_count: 1 }), catalog).plan,
    "yearly",
  );
  assertEquals(
    entitlementFromSubscription(sub(FOUR_WEEKLY_ID, { interval: "week", interval_count: 4 }), catalog).plan,
    "four_weekly",
  );
  // (r4) A kept id, same key at other cents, maps to that key.
  assertEquals(
    entitlementFromSubscription(sub(KEPT_ID, { interval: "week", interval_count: 4 }), catalog).plan,
    "four_weekly",
  );
  // An unknown id gives null, whatever its cadence: the interval never decides the plan.
  assertEquals(
    entitlementFromSubscription(sub("price_unknown", { interval: "year", interval_count: 1 }), catalog).plan,
    null,
  );
  assertEquals(
    entitlementFromSubscription(sub("price_unknown", { interval: "week", interval_count: 4 }), catalog).plan,
    null,
  );
  assertEquals(
    entitlementFromSubscription(sub("price_unknown", { interval: "month", interval_count: 1 }), catalog).plan,
    null,
  );
  assertEquals(entitlementFromSubscription({ status: "active", items: { data: [] } }, catalog).plan, null);

  assertEquals(
    entitlementFromSubscription({
      status: "trialing",
      items: {
        data: [{
          current_period_end: 1_760_000_000,
          price: { id: FOUR_WEEKLY_ID, recurring: { interval: "week", interval_count: 4 } },
        }],
      },
    }, catalog),
    {
      plan: "four_weekly",
      status: "trialing",
      current_period_end: "2025-10-09T08:53:20.000Z",
      stripe_subscription_id: null,
      paused: false,
      started_at: null,
    },
  );
  // A paused collection leaves Stripe's own status `active`. The summer pause is gone (D10), but the
  // `paused` column and this field stay (dropping them needs a migration and buys nothing), so the
  // mapping is still pinned.
  assertEquals(
    entitlementFromSubscription({
      status: "active",
      id: "sub_1",
      pause_collection: { behavior: "void" },
      start_date: 1_700_000_000,
      items: {
        data: [{
          current_period_end: 1_760_000_000,
          price: { id: YEARLY_ID, recurring: { interval: "year", interval_count: 1 } },
        }],
      },
    }, catalog),
    {
      plan: "yearly",
      status: "active",
      current_period_end: "2025-10-09T08:53:20.000Z",
      stripe_subscription_id: "sub_1",
      paused: true,
      started_at: "2023-11-14T22:13:20.000Z",
    },
  );
  assertEquals(
    entitlementFromSubscription({ status: "unpaid", items: { data: [] } }, catalog).status,
    "past_due",
  );
  assertEquals(
    entitlementFromSubscription({ status: "incomplete_expired", items: { data: [] } }, catalog).status,
    "canceled",
  );
  assertEquals(
    entitlementFromSubscription({ status: "wat", items: { data: [] } }, catalog).status,
    "none",
  );
  // The pre-basil shape: `current_period_end` still lives on the Subscription itself, not the item.
  // An account whose events still carry it that way must keep working.
  assertEquals(
    entitlementFromSubscription({
      status: "active",
      current_period_end: 1_760_000_000,
      items: {
        data: [{ price: { id: FOUR_WEEKLY_ID, recurring: { interval: "week", interval_count: 4 } } }],
      },
    }, catalog).current_period_end,
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
      ...billing(),
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
      ...billing(),
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
            price: { id: FOUR_WEEKLY_ID, recurring: { interval: "week", interval_count: 4 } },
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
      ...billing(),
      fetchSubscription: () => Promise.resolve(null),
    },
  );
  assertEquals(res.status, 200);
  assertEquals(written, [["acc-1", {
    plan: "four_weekly",
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
            price: { id: FOUR_WEEKLY_ID, recurring: { interval: "week", interval_count: 4 } },
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
      ...billing(),
      fetchSubscription: () => Promise.resolve(null),
    },
  );
  assertEquals(res.status, 200);
  assertEquals(written, [["acc-1", {
    plan: "four_weekly",
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
      ...billing(),
      fetchSubscription: () => Promise.resolve(null),
    },
  );
  assertEquals(res.status, 200);
  assert(!wrote);
});

Deno.test("an event for an account that no longer exists is 200 and a no-op — DELETE /account must not leave a retry storm behind", async () => {
  const event = {
    id: "evt_gone_1",
    type: "customer.subscription.updated",
    created: 1_700_000_000,
    data: {
      object: {
        metadata: { account_id: "acc-deleted" },
        status: "active",
        items: {
          data: [{
            current_period_end: 1_760_000_000,
            price: { id: FOUR_WEEKLY_ID, recurring: { interval: "week", interval_count: 4 } },
          }],
        },
      },
    },
  };
  const body = JSON.stringify(event);
  const t = 1_700_000_000;
  const sig = await hmacHex(SECRET, `${t}.${body}`);
  let fetchedAny = false;
  let wrote = false;
  let recordedAny = false;
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
      recordEvent: () => {
        recordedAny = true;
        return Promise.resolve();
      },
      currentUpdatedAt: () => Promise.resolve(null),
      accountExists: () => Promise.resolve(false),
      writeEntitlement: () => {
        wrote = true;
        return Promise.resolve();
      },
      ...billing(),
      fetchSubscription: (id) => {
        fetchedAny = true;
        return Promise.resolve({ id });
      },
    },
  );
  assertEquals(res.status, 200);
  assertEquals(await res.clone().json(), { ignored: "no such account" });
  assert(!fetchedAny, "a deleted account must never reach fetchSubscription");
  assert(!wrote, "a deleted account must never reach writeEntitlement");
  assert(!recordedAny, "a deleted account must never reach recordEvent");
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
      ...billing(),
      fetchSubscription: (id) => {
        fetched.push(id);
        return Promise.resolve({
          id: "sub_9",
          status: "past_due",
          items: {
            data: [{ price: { id: FOUR_WEEKLY_ID, recurring: { interval: "week", interval_count: 4 } } }],
          },
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
      ...billing(),
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
            price: { id: FOUR_WEEKLY_ID, recurring: { interval: "week", interval_count: 4 } },
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
    accountExists: () => Promise.resolve(true),
    writeEntitlement: (accountId: string, row: unknown, updatedAt: string) => {
      written.push([accountId, row, updatedAt]);
      return Promise.resolve();
    },
    ...billing(),
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
            price: { id: FOUR_WEEKLY_ID, recurring: { interval: "week", interval_count: 4 } },
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
      accountExists: () => Promise.resolve(true),
      writeEntitlement: () => {
        wrote = true;
        return Promise.resolve();
      },
      ...billing(),
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
            price: { id: FOUR_WEEKLY_ID, recurring: { interval: "week", interval_count: 4 } },
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
      accountExists: () => Promise.resolve(true),
      writeEntitlement: () => {
        wrote = true;
        return Promise.resolve();
      },
      ...billing(),
      fetchSubscription: () => Promise.resolve(null),
    },
  );
  assertEquals(res.status, 200);
  assertEquals(await res.clone().json(), { stale: "evt_stale_2" });
  assert(!wrote);
});

// ---------------------------------------------------------------------------------------------
// v0.4.1: a missing price setting, and the trial reminder (note D2, D6).

const T = 1_800_000_000;
const DAY = 24 * 60 * 60;

async function signed(event: unknown): Promise<Request> {
  const body = JSON.stringify(event);
  const sig = await hmacHex(SECRET, `${T}.${body}`);
  return new Request("http://127.0.0.1:1/", {
    method: "POST",
    body,
    headers: { "stripe-signature": `t=${T},v1=${sig}` },
  });
}

interface Seen {
  mails: Mail[];
  recorded: string[];
  written: unknown[];
  emailsAskedFor: string[];
}

function trialDeps(over: Partial<Deps> = {}): { deps: Deps; seen: Seen } {
  const seen: Seen = { mails: [], recorded: [], written: [], emailsAskedFor: [] };
  const deps: Deps = {
    secret: SECRET,
    nowSeconds: () => T,
    seenEvent: () => Promise.resolve(false),
    recordEvent: (id) => (seen.recorded.push(id), Promise.resolve()),
    currentUpdatedAt: () => Promise.resolve(null),
    accountExists: () => Promise.resolve(true),
    writeEntitlement: (...a) => (seen.written.push(a), Promise.resolve()),
    fetchSubscription: () => Promise.resolve(null),
    catalog: catalogOf(),
    accountEmail: (id) => (seen.emailsAskedFor.push(id), Promise.resolve("a@example.invalid")),
    sendEmail: (m) => (seen.mails.push(m), Promise.resolve()),
    ...over,
  };
  return { deps, seen };
}

function trialEvent(sub: Record<string, unknown> = {}, price: Record<string, unknown> = {}) {
  return {
    id: "evt_trial_1",
    type: "customer.subscription.trial_will_end",
    created: T,
    data: {
      object: {
        id: "sub_1",
        object: "subscription",
        metadata: { account_id: "acc-1" },
        status: "trialing",
        cancel_at_period_end: false,
        trial_end: T + 3 * DAY,
        items: {
          data: [{
            current_period_end: T + 3 * DAY,
            price: {
              id: YEARLY_ID,
              unit_amount: 8437,
              currency: "usd",
              recurring: { interval: "year", interval_count: 1 },
              ...price,
            },
          }],
        },
        ...sub,
      },
    },
  };
}

Deno.test("a_missing_price_setting_matches_nothing", async () => {
  const unset = catalogOf(TABLE, [priceVariable({ key: "yearly", cents: 8437 })]);
  const sub = (id: string) => ({ status: "active", items: { data: [{ price: { id } }] } });
  assertEquals(unset.selling.find((e) => e.key === "yearly")?.priceId, null);
  assertEquals(
    entitlementFromSubscription(sub(YEARLY_ID), unset).plan,
    null,
    "the unset entry matches nothing",
  );
  assertEquals(
    entitlementFromSubscription(sub(FOUR_WEEKLY_ID), unset).plan,
    "four_weekly",
    "the others still map",
  );
  assertEquals(entitlementFromSubscription(sub(KEPT_ID), unset).plan, "four_weekly");
  assertEquals(entitlementFromSubscription(sub(""), unset).plan, null);
  // No event answers 500 for it: each is written with plan null, or reminded without plan words.
  for (
    const event of [
      {
        id: "evt_u1",
        type: "customer.subscription.updated",
        created: T,
        data: { object: { metadata: { account_id: "acc-1" }, ...sub(YEARLY_ID) } },
      },
      trialEvent(),
    ]
  ) {
    const { deps, seen } = trialDeps({ catalog: unset });
    const res = await handle(await signed(event), deps);
    assertEquals(res.status, 200, event.type);
    if (event.type === "customer.subscription.updated") {
      assertEquals((seen.written[0] as [string, { plan: string | null }])[1].plan, null);
    } else {
      assertEquals(seen.mails.length, 1);
      assert(!seen.mails[0].text.includes("Yearly"), seen.mails[0].text);
    }
  }
});

Deno.test("trial_will_end_sends_one_reminder", async () => {
  const cases = [
    { price: {}, catalog: catalogOf(), plan: "Yearly", amount: "$84.37 once a year" },
    {
      price: { id: FOUR_WEEKLY_ID, unit_amount: 1299, recurring: { interval: "week", interval_count: 4 } },
      catalog: catalogOf(),
      plan: "Every 4 weeks",
      amount: "$12.99 every 4 weeks",
    },
    // (r4) An invented catalog entry at 1399 for that id: the words are the catalog's.
    {
      price: { id: FOUR_WEEKLY_ID, unit_amount: 1299, recurring: { interval: "week", interval_count: 4 } },
      catalog: catalogOf({
        ...TABLE,
        selling: [{ key: "yearly", cents: 8437 }, { key: "four_weekly", cents: 1399 }],
      }),
      plan: "Every 4 weeks",
      amount: "$13.99 every 4 weeks",
    },
  ];
  for (const c of cases) {
    const { deps, seen } = trialDeps({ catalog: c.catalog });
    const res = await handle(await signed(trialEvent({}, c.price)), deps);
    assertEquals(res.status, 200, c.amount);
    assertEquals(seen.mails.length, 1, "one reminder");
    assertEquals(seen.emailsAskedFor, ["acc-1"], "the account's own address, by its id");
    const m = seen.mails[0];
    assertEquals(m.to, "a@example.invalid");
    assert(m.text.includes(c.plan), `the plan words: ${m.text}`);
    assert(m.text.includes(c.amount), `the amount: ${m.text}`);
    assert(m.text.includes("plus tax where it applies"), m.text);
    assert(m.text.includes(chargeDate(T + 3 * DAY)), `the UTC-10 date: ${m.text}`);
    assert(m.text.includes("Manage subscription"), m.text);
    assert(m.text.includes("Settings") && m.text.includes("Membership"), m.text);
    for (const never of ["in 3 days", "3 days", "per month", "a month"]) {
      assert(!`${m.subject}\n${m.text}`.includes(never), `never "${never}": ${m.text}`);
    }
    assertEquals(seen.recorded, ["evt_trial_1"], "the event is recorded");
    assertEquals(seen.written, [], "and no entitlement is written");
  }
});

Deno.test("an_unknown_price_still_gets_its_reminder", async () => {
  const { deps, seen } = trialDeps();
  const res = await handle(
    await signed(
      trialEvent({}, {
        id: "price_unknown",
        unit_amount: 1500,
        recurring: { interval: "month", interval_count: 2 },
      }),
    ),
    deps,
  );
  assertEquals(res.status, 200);
  assertEquals(seen.mails.length, 1);
  const text = seen.mails[0].text;
  assert(text.includes("$15.00 every 2 months"), text);
  assert(text.includes("plus tax where it applies"), text);
  assert(!text.includes("Yearly") && !text.includes("Every 4 weeks"), `no plan words: ${text}`);
  assertEquals(seen.recorded, ["evt_trial_1"]);
});

Deno.test("no_reminder_for_a_subscription_that_will_not_charge", async () => {
  for (
    const [name, sub] of [
      ["set to cancel at period end", { cancel_at_period_end: true }],
      ["active, not trialing", { status: "active" }],
      ["canceled", { status: "canceled" }],
      ["a trial that already ended", { trial_end: T - 60 }],
      ["no trial end", { trial_end: null }],
    ] as const
  ) {
    const { deps, seen } = trialDeps();
    const res = await handle(await signed(trialEvent(sub)), deps);
    assertEquals(res.status, 200, name);
    assertEquals(seen.mails, [], `${name}: nothing sent`);
    assertEquals(seen.recorded, ["evt_trial_1"], `${name}: the event is recorded`);
    assertEquals(seen.written, [], name);
  }
});

Deno.test("a_failed_reminder_is_retried_by_stripe", async () => {
  const logged: string[] = [];
  const original = console.error;
  console.error = (...a: unknown[]) => void logged.push(a.map(String).join(" "));
  try {
    const { deps, seen } = trialDeps({
      sendEmail: () => Promise.reject(new Error("mail to a@example.invalid refused")),
    });
    const res = await handle(await signed(trialEvent()), deps);
    assertEquals(res.status, 502);
    assertEquals(seen.recorded, [], "nothing recorded, so Stripe's retry is not a duplicate");
    assertEquals(seen.written, []);
    assert(logged.length > 0, "the failure is logged");
    for (const l of logged) {
      assert(l.includes("acc-1"), `the log names the account id: ${l}`);
      assert(!l.includes("@"), `and never an address: ${l}`);
    }
  } finally {
    console.error = original;
  }
});

Deno.test("a_reminder_sent_but_not_recorded_is_not_retried", async () => {
  // The send succeeded and the record failed (a transient database error). A 5xx here would make Stripe
  // retry, `seenEvent` would still be false, and the student would get a second reminder: so 200.
  const logged: string[] = [];
  const original = console.error;
  console.error = (...a: unknown[]) => void logged.push(a.map(String).join(" "));
  try {
    const { deps, seen } = trialDeps({
      recordEvent: () => Promise.reject(new Error("database down for a@example.invalid")),
    });
    const res = await handle(await signed(trialEvent()), deps).catch((e) => e as Response);
    assertEquals(res.status, 200);
    assertEquals(seen.mails.length, 1, "one mail");
    assertEquals(seen.written, []);
    assert(logged.some((l) => l.includes("acc-1") && l.includes("not recorded")), logged.join(" | "));
    for (const l of logged) assert(!l.includes("@"), `the log holds no address: ${l}`);
  } finally {
    console.error = original;
  }
});

Deno.test("a_duplicate_trial_will_end_sends_nothing", async () => {
  const { deps, seen } = trialDeps({ seenEvent: () => Promise.resolve(true) });
  const res = await handle(await signed(trialEvent()), deps);
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { duplicate: "evt_trial_1" });
  assertEquals(seen.mails, []);
  assertEquals(seen.emailsAskedFor, []);
});

Deno.test("the_date_is_never_later_than_the_students_own", () => {
  const at = (iso: string) => Date.parse(iso) / 1000;
  // Midday UTC: the same date everywhere in the fifty states.
  assertEquals(chargeDate(at("2026-12-15T12:00:00Z")), "Tuesday, December 15, 2026");
  // 05:00 UTC is still the 14th in Hawaii (UTC-10), so the mail says the 14th: one day before UTC.
  assertEquals(chargeDate(at("2026-12-15T05:00:00Z")), "Monday, December 14, 2026");
  // The edge: exactly 10:00 UTC is midnight in Hawaii, the 15th.
  assertEquals(chargeDate(at("2026-12-15T10:00:00Z")), "Tuesday, December 15, 2026");
  assertEquals(chargeDate(at("2026-12-15T09:59:59Z")), "Monday, December 14, 2026");
  assertEquals(chargeDate(at("2027-01-01T03:00:00Z")), "Thursday, December 31, 2026");
});

Deno.test("index.ts resolves the catalog with every variable optional and names the mail settings", async () => {
  const text = await Deno.readTextFile(new URL("./index.ts", import.meta.url));
  assert(/from "\.\.\/_shared\/price_catalog\.ts"/.test(text), "index.ts imports the catalog");
  assert(
    /resolveCatalog\(\s*CATALOG,\s*\(name\)\s*=>\s*Deno\.env\.get\(name\)\s*\)/.test(text),
    "the catalog is resolved through Deno.env.get, every variable optional",
  );
  assert(!/resolveCatalog\([^)]*\benv\(/.test(text), "never through the file's throwing env()");
  assert(!/STRIPE_PRICE_/.test(text), "no price variable is named here");
  for (const name of ["EMAIL_API_URL", "EMAIL_API_KEY", "EMAIL_FROM"]) {
    assert(text.includes(`"${name}"`), name);
  }
  assert(/\bcatalog[,:]/.test(text), "the catalog is handed to the handler");
  const m = text.match(/accountEmail: async \(accountId\) => \{[\s\S]*?\n {6}\},/);
  assert(m, "could not find accountEmail's body in index.ts");
  assert(m[0].includes('"accounts",'), m[0]);
  assert(m[0].includes("`id=eq.${encodeURIComponent(accountId)}&select=email&limit=1`"), m[0]);
});
