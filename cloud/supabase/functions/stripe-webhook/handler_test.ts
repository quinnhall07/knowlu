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
  subscribeConfirmation,
  trialReminder,
} from "./handler.ts";
import { SubscriptionReadError } from "./subscription_read.ts";

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

/** The catalog, and a mailer that must not be reached by any event but `trial_will_end` and
 * `checkout.session.completed` (no test using this helper sends either). */
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

// ---------------------------------------------------------------------------------------------
// The subscribe confirmation (note D6b): Knowlu's own email on `checkout.session.completed`.

Deno.test("the_trial_reminder_text_is_unchanged", () => {
  // Pinned whole before the plan words were shared with the confirmation: not a byte may move.
  const m = trialReminder(trialEvent().data.object, catalogOf(), "a@example.invalid");
  assertEquals(m, {
    to: "a@example.invalid",
    subject: "Your Knowlu free trial is ending",
    text: [
      "Your Knowlu free trial is ending.",
      "",
      `Your first charge is on ${chargeDate(T + 3 * DAY)}:`,
      "Knowlu, Yearly, $84.37 once a year, plus tax where it applies.",
      "It renews automatically until you cancel.",
      "",
      "To cancel before then, open Knowlu and go to Settings, then Membership, then",
      "Manage subscription, or reply to this message and we will cancel it for you.",
    ].join("\n"),
  });
});

const TRIAL_END = T + 14 * DAY;
const CANCEL_LINES = [
  "To cancel, open Knowlu and go to Settings, then Membership, then",
  "Manage subscription, or reply to this message and we will cancel it for you.",
];

/** A subscription as `fetchSubscription` returns it. Trialing on the yearly price by default. */
function confirmSub(over: Record<string, unknown> = {}, price: Record<string, unknown> = {}) {
  return {
    id: "sub_test_confirm",
    object: "subscription",
    customer: "cus_test_confirm",
    metadata: { account_id: "acc-1" },
    status: "trialing",
    trial_end: TRIAL_END,
    start_date: T,
    discounts: [],
    discount: null,
    items: {
      data: [{
        current_period_end: TRIAL_END,
        price: {
          id: YEARLY_ID,
          unit_amount: 8437,
          currency: "usd",
          recurring: { interval: "year", interval_count: 1 },
          ...price,
        },
      }],
    },
    ...over,
  };
}

const FOUR_WEEKLY_PRICE = {
  id: FOUR_WEEKLY_ID,
  unit_amount: 1299,
  recurring: { interval: "week", interval_count: 4 },
};

/** An active subscription: no trial, its period ending `endIn` seconds from T. */
function activeSub(endIn: number, price: Record<string, unknown> = {}) {
  const s = confirmSub({ status: "active", trial_end: null }, price);
  s.items.data[0].current_period_end = T + endIn;
  return s;
}

Deno.test("the_four_confirmation_texts", () => {
  const catalog = catalogOf();
  const cases: [string, unknown, Mail][] = [
    ["four_weekly, trialing", confirmSub({}, FOUR_WEEKLY_PRICE), {
      to: "a@example.invalid",
      subject: "Your Knowlu free trial has started",
      text: [
        "Your Knowlu free trial has started. Keep this message: it is your confirmation.",
        "",
        "Your plan: Knowlu, Every 4 weeks, $12.99 every 4 weeks, plus tax where it applies.",
        "Your first charge is on Thursday, January 28, 2027, when your free trial ends.",
        "It renews automatically until you cancel.",
        "",
        ...CANCEL_LINES,
      ].join("\n"),
    }],
    ["yearly, trialing", confirmSub(), {
      to: "a@example.invalid",
      subject: "Your Knowlu free trial has started",
      text: [
        "Your Knowlu free trial has started. Keep this message: it is your confirmation.",
        "",
        "Your plan: Knowlu, Yearly, $84.37 once a year, plus tax where it applies.",
        "Your first charge is on Thursday, January 28, 2027, when your free trial ends.",
        "It renews automatically until you cancel.",
        "30-day money-back guarantee on each yearly charge.",
        "",
        ...CANCEL_LINES,
      ].join("\n"),
    }],
    ["four_weekly, active", activeSub(28 * DAY, FOUR_WEEKLY_PRICE), {
      to: "a@example.invalid",
      subject: "Your Knowlu subscription has started",
      text: [
        "Your Knowlu subscription has started. Keep this message: it is your confirmation.",
        "",
        "Your plan: Knowlu, Every 4 weeks, $12.99 every 4 weeks, plus tax where it applies.",
        "Your next charge is on Thursday, February 11, 2027.",
        "It renews automatically until you cancel.",
        "",
        ...CANCEL_LINES,
      ].join("\n"),
    }],
    ["yearly, active", activeSub(365 * DAY), {
      to: "a@example.invalid",
      subject: "Your Knowlu subscription has started",
      text: [
        "Your Knowlu subscription has started. Keep this message: it is your confirmation.",
        "",
        "Your plan: Knowlu, Yearly, $84.37 once a year, plus tax where it applies.",
        "Your next charge is on Friday, January 14, 2028.",
        "It renews automatically until you cancel.",
        "30-day money-back guarantee on each yearly charge.",
        "",
        ...CANCEL_LINES,
      ].join("\n"),
    }],
  ];
  for (const [name, sub, want] of cases) {
    assertEquals(subscribeConfirmation(sub, catalog, "a@example.invalid"), want, name);
  }
  // The dates are the shared formatter's, in UTC-10.
  assertEquals(chargeDate(TRIAL_END), "Thursday, January 28, 2027");
  assertEquals(chargeDate(T + 28 * DAY), "Thursday, February 11, 2027");
  assertEquals(chargeDate(T + 365 * DAY), "Friday, January 14, 2028");
});

Deno.test("the_confirmation_takes_its_words_from_the_catalog", () => {
  const invented = catalogOf({
    ...TABLE,
    words: { tax: "and an invented levy", moneyBack: "An invented promise about yearly refunds" },
  });
  for (const sub of [confirmSub(), activeSub(365 * DAY)]) {
    const { text } = subscribeConfirmation(sub, invented, "a@example.invalid");
    assert(text.includes("Your plan: Knowlu, Yearly, $84.37 once a year, and an invented levy.\n"), text);
    assert(text.includes("\nAn invented promise about yearly refunds.\n"), text);
    assert(!text.includes(WORDS.tax), `not the real tax words: ${text}`);
    assert(!text.includes(WORDS.moneyBack), `not the real money-back words: ${text}`);
  }
  // An invented amount for the same id: the figure is the catalog's too.
  const dearer = catalogOf({
    ...TABLE,
    selling: [{ key: "yearly", cents: 9100 }, { key: "four_weekly", cents: 1299 }],
  });
  assert(
    subscribeConfirmation(confirmSub(), dearer, "a@example.invalid").text.includes("$91.00 once a year"),
  );
});

Deno.test("an_unknown_price_still_gets_its_confirmation_with_the_reminders_fallback_words", () => {
  const catalog = catalogOf();
  const unknown = confirmSub({}, {
    id: "price_unknown",
    unit_amount: 1500,
    recurring: { interval: "month", interval_count: 2 },
  });
  const m = subscribeConfirmation(unknown, catalog, "a@example.invalid");
  assert(m.text.includes("\nYour plan: Knowlu, $15.00 every 2 months, plus tax where it applies.\n"), m.text);
  assert(!m.text.includes(WORDS.moneyBack), `no money-back line without a yearly entry: ${m.text}`);
  assert(!m.text.includes("Yearly") && !m.text.includes("Every 4 weeks"), m.text);
  // The same words the reminder writes for the same price.
  assert(
    trialReminder(unknown, catalog, "a@example.invalid").text.includes("Knowlu, $15.00 every 2 months, "),
  );
  // A price that cannot be stated truthfully: the reminder's last fallback.
  const eur = confirmSub({}, { id: "price_unknown", currency: "eur" });
  assert(
    subscribeConfirmation(eur, catalog, "a@example.invalid").text.includes(
      "\nYour plan: Knowlu, at the price shown in Manage subscription, plus tax where it applies.\n",
    ),
  );
});

Deno.test("the_discount_line_comes_only_with_a_discount", () => {
  const line = "A promotion code applies to this subscription, so you are charged less than the price above.";
  const catalog = catalogOf();
  const shapes: [string, Record<string, unknown>, boolean][] = [
    ["a non-empty discounts array", { discounts: ["di_test_1"] }, true],
    ["a discount object", { discounts: undefined, discount: { id: "di_test_2", object: "discount" } }, true],
    ["an empty array and a null discount", { discounts: [], discount: null }, false],
    ["neither field", { discounts: undefined, discount: undefined }, false],
  ];
  for (const [name, over, present] of shapes) {
    for (const sub of [confirmSub(over), { ...activeSub(28 * DAY, FOUR_WEEKLY_PRICE), ...over }]) {
      const { text } = subscribeConfirmation(sub, catalog, "a@example.invalid");
      assertEquals(text.includes(line), present, `${name}: ${text}`);
      if (present) {
        // After the date line, before the renewal line.
        const lines = text.split("\n");
        const at = lines.indexOf(line);
        assert(lines[at - 1].includes("charge is on"), text);
        assertEquals(lines[at + 1], "It renews automatically until you cancel.");
      }
    }
  }
});

Deno.test("no_period_end_and_no_trial_means_no_date_line", () => {
  const sub = activeSub(28 * DAY, FOUR_WEEKLY_PRICE);
  delete (sub.items.data[0] as { current_period_end?: number }).current_period_end;
  assertEquals(
    subscribeConfirmation(sub, catalogOf(), "a@example.invalid").text,
    [
      "Your Knowlu subscription has started. Keep this message: it is your confirmation.",
      "",
      "Your plan: Knowlu, Every 4 weeks, $12.99 every 4 weeks, plus tax where it applies.",
      "It renews automatically until you cancel.",
      "",
      ...CANCEL_LINES,
    ].join("\n"),
  );
  // The pre-basil shape: the period end on the subscription itself is read, as the entitlement reads it.
  const older = { ...sub, current_period_end: T + 28 * DAY };
  assert(
    subscribeConfirmation(older, catalogOf(), "a@example.invalid").text.includes(
      "\nYour next charge is on Thursday, February 11, 2027.\n",
    ),
  );
});

function checkoutEvent(session: Record<string, unknown> = {}) {
  return {
    id: "evt_checkout_1",
    type: "checkout.session.completed",
    created: T,
    data: {
      object: {
        id: "cs_test_confirm",
        object: "checkout.session",
        mode: "subscription",
        client_reference_id: "acc-1",
        customer: "cus_test_confirm",
        subscription: "sub_test_confirm",
        ...session,
      },
    },
  };
}

/** `trialDeps`, with `fetchSubscription` answering `sub` for the event's id. */
function confirmDeps(sub: unknown, over: Partial<Deps> = {}) {
  return trialDeps({
    fetchSubscription: (id) => Promise.resolve(id === "sub_test_confirm" ? sub : null),
    ...over,
  });
}

Deno.test("checkout_completed_sends_one_confirmation_to_the_accounts_address", async () => {
  for (const sub of [confirmSub(), activeSub(28 * DAY, FOUR_WEEKLY_PRICE)]) {
    const { deps, seen } = confirmDeps(sub);
    const res = await handle(await signed(checkoutEvent()), deps);
    assertEquals(res.status, 200);
    assertEquals(await res.json(), { ok: true, confirmed: true });
    assertEquals(seen.emailsAskedFor, ["acc-1"], "the account's own address, by its id");
    assertEquals(seen.mails, [subscribeConfirmation(sub, deps.catalog, "a@example.invalid")]);
    assertEquals(seen.written.length, 1, "the entitlement is written, as before");
    assertEquals(seen.recorded, ["evt_checkout_1"]);
  }
  // A session that does not name its mode but carries a subscription is a subscription checkout.
  const { deps, seen } = confirmDeps(confirmSub());
  const res = await handle(await signed(checkoutEvent({ mode: undefined })), deps);
  assertEquals(await res.json(), { ok: true, confirmed: true });
  assertEquals(seen.mails.length, 1);
});

Deno.test("the_confirmation_does_not_wait_behind_the_stale_guard", async () => {
  // `customer.subscription.created` often lands first with the same timestamp: the entitlement write
  // is skipped, and the mail still goes.
  const { deps, seen } = confirmDeps(confirmSub(), {
    currentUpdatedAt: () => Promise.resolve(new Date(T * 1000).toISOString()),
  });
  const res = await handle(await signed(checkoutEvent()), deps);
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { stale: "evt_checkout_1", confirmed: true });
  assertEquals(seen.written, [], "the stale write is still skipped");
  assertEquals(seen.mails.length, 1);
  assertEquals(seen.recorded, ["evt_checkout_1"]);
});

Deno.test("no_confirmation_for_a_subscription_that_has_not_started", async () => {
  for (const status of ["incomplete", "past_due", "canceled"]) {
    const { deps, seen } = confirmDeps(confirmSub({ status }));
    const res = await handle(await signed(checkoutEvent()), deps);
    assertEquals(res.status, 200, status);
    assertEquals(await res.json(), { ok: true, confirmed: false }, status);
    assertEquals(seen.mails, [], `${status}: nothing sent`);
    assertEquals(seen.written.length, 1, `${status}: the entitlement is written, as before`);
    assertEquals(seen.recorded, ["evt_checkout_1"], status);
  }
});

Deno.test("no_confirmation_without_an_address_or_for_a_payment_session", async () => {
  {
    const { deps, seen } = confirmDeps(confirmSub(), { accountEmail: () => Promise.resolve(null) });
    const res = await handle(await signed(checkoutEvent()), deps);
    assertEquals(res.status, 200);
    assertEquals(await res.json(), { ok: true, confirmed: false });
    assertEquals(seen.mails, []);
    assertEquals(seen.recorded, ["evt_checkout_1"]);
  }
  {
    const { deps, seen } = confirmDeps(confirmSub());
    const res = await handle(await signed(checkoutEvent({ mode: "payment" })), deps);
    assertEquals(res.status, 200);
    assertEquals(await res.json(), { ok: true, confirmed: false });
    assertEquals(seen.mails, [], "a payment session is not a subscription");
    assertEquals(seen.emailsAskedFor, []);
    assertEquals(seen.recorded, ["evt_checkout_1"], "and the event is still recorded");
  }
});

Deno.test("no_confirmation_on_any_other_event", async () => {
  const sub = { ...confirmSub(), metadata: { account_id: "acc-1" } };
  const events = [
    ...["created", "updated", "deleted"].map((t) => ({
      id: `evt_sub_${t}`,
      type: `customer.subscription.${t}`,
      created: T,
      data: { object: sub },
    })),
    {
      id: "evt_invoice_confirm",
      type: "invoice.payment_failed",
      created: T,
      data: {
        object: {
          parent: {
            subscription_details: { subscription: "sub_test_confirm", metadata: { account_id: "acc-1" } },
          },
        },
      },
    },
    trialEvent(),
  ];
  for (const event of events) {
    const { deps, seen } = confirmDeps(activeSub(28 * DAY));
    const res = await handle(await signed(event), deps);
    assertEquals(res.status, 200, event.type);
    const body = await res.json();
    assert(!("confirmed" in body), `${event.type}: ${JSON.stringify(body)}`);
    for (const m of seen.mails) {
      assert(!m.subject.includes("has started"), `${event.type} sent ${m.subject}`);
    }
    // Only the trial event mails at all, and that is the reminder.
    assertEquals(
      seen.mails.length,
      event.type === "customer.subscription.trial_will_end" ? 1 : 0,
      event.type,
    );
  }
});

Deno.test("a_duplicate_checkout_event_sends_nothing", async () => {
  const { deps, seen } = confirmDeps(confirmSub(), { seenEvent: () => Promise.resolve(true) });
  const res = await handle(await signed(checkoutEvent()), deps);
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { duplicate: "evt_checkout_1" });
  assertEquals(seen.mails, []);
  assertEquals(seen.emailsAskedFor, []);
});

/** Captures `console.error` for `body`, and gives the lines back. */
async function logsOf(body: () => Promise<void>): Promise<string[]> {
  const logged: string[] = [];
  const original = console.error;
  console.error = (...a: unknown[]) => void logged.push(a.map(String).join(" "));
  try {
    await body();
  } finally {
    console.error = original;
  }
  return logged;
}

Deno.test("a_failed_confirmation_is_retried_by_stripe_and_sent_once", async () => {
  // One account's state across two deliveries of the same event: Stripe's retry after a 502.
  const recorded = new Set<string>();
  let updatedAt: string | null = null;
  let refuse = true;
  const sendsTried: number[] = [];
  const { deps, seen } = confirmDeps(confirmSub(), {
    seenEvent: (id) => Promise.resolve(recorded.has(id)),
    recordEvent: (id) => (recorded.add(id), seen.recorded.push(id), Promise.resolve()),
    currentUpdatedAt: () => Promise.resolve(updatedAt),
    writeEntitlement: (...a) => (updatedAt = a[2], seen.written.push(a), Promise.resolve()),
    sendEmail: (m) => {
      // The order of effects: the entitlement is written before the send is tried.
      sendsTried.push(seen.written.length);
      if (refuse) return Promise.reject(new Error("mail to a@example.invalid refused by the relay"));
      seen.mails.push(m);
      return Promise.resolve();
    },
  });
  const out: { first?: Response } = {};
  const logged = await logsOf(async () => {
    out.first = await handle(await signed(checkoutEvent()), deps);
  });
  assertEquals(out.first?.status, 502);
  assertEquals(seen.written.length, 1, "the entitlement was written");
  assertEquals(sendsTried, [1], "before the send was tried");
  assertEquals(seen.recorded, [], "nothing recorded, so Stripe's retry is not a duplicate");
  assertEquals(seen.mails, []);
  assertEquals(logged.length, 1, logged.join(" | "));
  assert(logged[0].includes("acc-1"), `the log names the account id: ${logged[0]}`);
  for (const never of ["@", "refused by the relay", "sub_", "cus_", "cs_", "evt_"]) {
    assert(!logged[0].includes(never), `never "${never}": ${logged[0]}`);
  }

  refuse = false;
  const second = await handle(await signed(checkoutEvent()), deps);
  assertEquals(second.status, 200);
  assertEquals(await second.json(), { stale: "evt_checkout_1", confirmed: true });
  assertEquals(seen.written.length, 1, "the retry's write is stale and skipped");
  assertEquals(seen.mails.length, 1, "sent once");
  assertEquals(sendsTried, [1, 1], "the retry tried once more, its write skipped as stale");
  assertEquals(seen.recorded, ["evt_checkout_1"]);

  const third = await handle(await signed(checkoutEvent()), deps);
  assertEquals(await third.json(), { duplicate: "evt_checkout_1" });
  assertEquals(seen.mails.length, 1, "and never again");
});

Deno.test("a_confirmation_sent_but_not_recorded_is_not_retried", async () => {
  const out: { res?: Response } = {};
  const { deps, seen } = confirmDeps(confirmSub(), {
    recordEvent: () => Promise.reject(new Error("database down for a@example.invalid")),
  });
  const logged = await logsOf(async () => {
    out.res = await handle(await signed(checkoutEvent()), deps);
  });
  assertEquals(out.res?.status, 200);
  assertEquals(await out.res?.json(), { ok: true, confirmed: true });
  assertEquals(seen.mails.length, 1, "one mail");
  assertEquals(logged.length, 1, logged.join(" | "));
  assert(logged[0].includes("acc-1") && logged[0].includes("not recorded"), logged[0]);
  for (const never of ["@", "database down", "sub_", "cus_", "cs_", "evt_"]) {
    assert(!logged[0].includes(never), `never "${never}": ${logged[0]}`);
  }
});

Deno.test("the_confirmation_holds_no_id", async () => {
  const subs = [
    confirmSub({ discounts: ["di_test_1"] }),
    activeSub(365 * DAY),
    confirmSub({}, FOUR_WEEKLY_PRICE),
    confirmSub({}, { id: "price_unknown" }),
  ];
  for (const sub of subs) {
    const { deps, seen } = confirmDeps(sub);
    await handle(await signed(checkoutEvent()), deps);
    assertEquals(seen.mails.length, 1);
    const all = `${seen.mails[0].subject}\n${seen.mails[0].text}`;
    for (const never of ["sub_", "cus_", "cs_", "price_", "di_", "evt_", "acc-1", "http", "14 days"]) {
      assert(!all.includes(never), `never "${never}": ${all}`);
    }
  }
});

// ---------------------------------------------------------------------------------------------
// Fix round 1: only Stripe's 404 is "no such subscription"; any other failed read is a 502.

function invoiceEvent() {
  return {
    id: "evt_invoice_read",
    type: "invoice.payment_failed",
    created: T,
    data: {
      object: {
        parent: {
          subscription_details: { subscription: "sub_test_confirm", metadata: { account_id: "acc-1" } },
        },
      },
    },
  };
}

Deno.test("a_404_on_the_subscription_read_is_ignored_as_before", async () => {
  for (const event of [checkoutEvent(), invoiceEvent()]) {
    const { deps, seen } = confirmDeps(null);
    const out: { res?: Response } = {};
    const logged = await logsOf(async () => {
      out.res = await handle(await signed(event), deps);
    });
    const res = out.res!;
    // One line, the function and the event type only: a 404 here usually means a key of the wrong
    // mode or account, which nothing else would show.
    assertEquals(logged, [`stripe-webhook: ${event.type}: Stripe has no such subscription`], event.type);
    for (const never of ["acc-1", "sub_", "cus_", "cs_", "evt_", "@"]) {
      assert(!logged[0].includes(never), `never "${never}": ${logged[0]}`);
    }
    assertEquals(res.status, 200, event.type);
    assertEquals(await res.json(), { ignored: `no subscription on ${event.type}` });
    assertEquals(seen.written, [], event.type);
    assertEquals(seen.mails, [], event.type);
    assertEquals(seen.recorded, [], `${event.type}: nothing recorded, as before`);
  }
});

Deno.test("a_failed_subscription_read_is_502_and_stripe_redelivers", async () => {
  const failures: [string, () => Promise<never>, string][] = [
    ["a 500", () => Promise.reject(new SubscriptionReadError(500)), "(500)"],
    ["a 429", () => Promise.reject(new SubscriptionReadError(429)), "(429)"],
    [
      "a network error",
      () =>
        Promise.reject(
          new TypeError(
            "error sending request for url (https://api.stripe.com/v1/subscriptions/sub_test_confirm)",
          ),
        ),
      "(network)",
    ],
  ];
  for (const event of [checkoutEvent(), invoiceEvent()]) {
    for (const [name, failing, named] of failures) {
      const recorded = new Set<string>();
      let readWorks = false;
      const { deps, seen } = confirmDeps(null, {
        seenEvent: (id) => Promise.resolve(recorded.has(id)),
        recordEvent: (id) => (recorded.add(id), seen.recorded.push(id), Promise.resolve()),
        fetchSubscription: (id) =>
          readWorks ? Promise.resolve(id === "sub_test_confirm" ? confirmSub() : null) : failing(),
      });
      const out: { res?: Response } = {};
      const logged = await logsOf(async () => {
        out.res = await handle(await signed(event), deps);
      });
      const at = `${event.type}, ${name}`;
      assertEquals(out.res?.status, 502, at);
      assertEquals(seen.written, [], `${at}: nothing written`);
      assertEquals(seen.mails, [], `${at}: nothing sent`);
      assertEquals(seen.emailsAskedFor, [], `${at}: no address read`);
      assertEquals(seen.recorded, [], `${at}: nothing recorded, so Stripe redelivers`);
      assertEquals(logged.length, 1, `${at}: ${logged.join(" | ")}`);
      assert(logged[0].startsWith("stripe-webhook: "), logged[0]);
      assert(logged[0].includes(event.type) && logged[0].includes(named), logged[0]);
      for (const never of ["acc-1", "sub_", "cus_", "cs_", "evt_", "@", "https", "error sending"]) {
        assert(!logged[0].includes(never), `${at}: never "${never}": ${logged[0]}`);
      }

      // Stripe's redelivery, with the read working again: written, mailed once (checkout only), recorded.
      readWorks = true;
      const again = await handle(await signed(event), deps);
      assertEquals(again.status, 200, at);
      assertEquals(seen.written.length, 1, `${at}: written on the redelivery`);
      assertEquals(seen.mails.length, event.type === "checkout.session.completed" ? 1 : 0, at);
      assertEquals(seen.recorded, [event.id], at);
      const third = await handle(await signed(event), deps);
      assertEquals(await third.json(), { duplicate: event.id }, at);
      assertEquals(
        seen.mails.length,
        event.type === "checkout.session.completed" ? 1 : 0,
        `${at}: never again`,
      );
    }
  }
});
