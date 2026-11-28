import { assert, assertEquals } from "@std/assert";
import {
  checkoutForm,
  type ConsentRecord,
  type Deps,
  handle,
  OPEN_PAGE_SIZE,
  OPEN_ROUND_BOUND,
  openCheckoutsPath,
  unloggedRefusals,
} from "./handler.ts";
import { stripePostFrom } from "../_shared/stripe.ts";
import { CASES } from "../../migrations/testers_cases.ts";
import {
  type CatalogDef,
  offer,
  priceVariable,
  resolveCatalog,
  type ResolvedCatalog,
  TERMS_VERSION,
  WORDS,
} from "../_shared/price_catalog.ts";

// Every handler test builds its own catalog from a table here, through the catalog's own resolve
// helper (note D13); none reads the real `SELLING`, so a price change edits no test in this file.
const TABLE: CatalogDef = {
  termsVersion: TERMS_VERSION,
  selling: [{ key: "yearly", cents: 8437 }, { key: "four_weekly", cents: 1299 }],
  kept: [],
  words: WORDS,
};

/** The price id each selling entry's variable holds, in the test's own invented environment. */
const PRICE_ID: Record<string, string> = {
  yearly: "price_test_yearly",
  four_weekly: "price_test_four_weekly",
};

function catalogOf(def: CatalogDef = TABLE, unset: string[] = []): ResolvedCatalog {
  const env: Record<string, string> = {};
  for (const e of def.selling) env[priceVariable(e)] = PRICE_ID[e.key];
  return resolveCatalog(def, (name) => (unset.includes(name) ? undefined : env[name]));
}

/** The Stripe Price each selling entry should point at: active, USD, its cents, its cadence, tax added. */
function goodPrice(c: ResolvedCatalog, id: string): Record<string, unknown> | null {
  const e = c.selling.find((x) => x.priceId === id);
  if (!e) return null;
  return {
    id,
    active: true,
    currency: "usd",
    unit_amount: e.cents,
    recurring: { interval: e.interval, interval_count: e.count },
    tax_behavior: "exclusive",
  };
}

/** T1b: Stripe's list of a customer's open Checkout Sessions, when there are none. */
const NOTHING_OPEN: Record<string, unknown> = { object: "list", data: [], has_more: false };

interface Seen {
  calls: string[];
  consents: ConsentRecord[];
  forms: Record<string, string>[];
}

/** A recording fake. `calls` names every read and write after the session check, in order. */
function fakes(over: Partial<Deps> = {}): { deps: Deps; seen: Seen } {
  const seen: Seen = { calls: [], consents: [], forms: [] };
  const catalog = over.catalog ?? catalogOf();
  const deps: Deps = {
    verify: () => Promise.resolve({ id: "acc-1", email: "a@example.invalid" }),
    getAccount: () => {
      seen.calls.push("getAccount");
      return Promise.resolve({
        email: "a@example.invalid",
        stripe_customer_id: "cus_1",
        age_attested_at: "2026-09-17T12:00:00Z",
      });
    },
    isTester: () => (seen.calls.push("isTester"), Promise.resolve(false)),
    subscriptionState: () => (seen.calls.push("subscriptionState"), Promise.resolve(null)),
    getPrice: (id) => (seen.calls.push(`getPrice:${id}`), Promise.resolve(goodPrice(catalog, id))),
    // T1b: nothing open, unless a test says otherwise. Not recorded, so every order above is today's.
    listOpenCheckouts: () => Promise.resolve(NOTHING_OPEN),
    expireCheckout: (id) => (seen.calls.push(`expire:${id}`), Promise.resolve({ id, status: "expired" })),
    saveCustomerId: () => (seen.calls.push("saveCustomerId"), Promise.resolve()),
    recordConsent: (c) => {
      seen.calls.push("recordConsent");
      seen.consents.push(c);
      return Promise.resolve();
    },
    stripe: (path, form) => {
      seen.calls.push(path);
      if (path === "/v1/checkout/sessions") seen.forms.push(form);
      return Promise.resolve(
        path === "/v1/customers"
          ? { id: "cus_new" }
          : { id: "cs_1", url: "https://checkout.stripe.com/c/cs_1" },
      );
    },
    successUrl: "https://knowlu.com/subscribed.html",
    cancelUrl: "https://knowlu.com/index.html",
    ...over,
    catalog,
  };
  return { deps, seen };
}

/** A purchase as the app sends it: the plan, and the offer's amount and terms version. */
function purchase(
  body: Record<string, unknown>,
  headers: Record<string, string> = {},
): Request {
  return new Request("http://127.0.0.1:1/", {
    method: "POST",
    headers: { authorization: "Bearer good", ...headers },
    body: JSON.stringify(body),
  });
}

const OFFERED: Record<string, number> = { yearly: 8437, four_weekly: 1299 };
const buy = (plan: string, headers: Record<string, string> = {}) =>
  purchase({ plan, amount_cents: OFFERED[plan], terms_version: TERMS_VERSION }, headers);

const run = (req: Request, deps: Deps) => handle(req, deps).catch((e) => e as Response);

const THE_PURCHASE = [
  "getAccount",
  "isTester",
  "subscriptionState",
  "getPrice:",
  "recordConsent",
  "/v1/checkout/sessions",
];

Deno.test("the_signed_plans_reach_stripe_with_their_price", async () => {
  for (const plan of ["four_weekly", "yearly"]) {
    const { deps, seen } = fakes();
    const res = await run(buy(plan), deps);
    assertEquals(res.status, 200, plan);
    assertEquals(await res.json(), { url: "https://checkout.stripe.com/c/cs_1" });
    assertEquals(seen.forms.length, 1);
    assertEquals(seen.forms[0]["line_items[0][price]"], PRICE_ID[plan], plan);
    assertEquals(seen.forms[0]["line_items[0][quantity]"], "1");
    assertEquals(seen.consents.length, 1);
    assertEquals(seen.consents[0].price_cents, OFFERED[plan], plan);
    assertEquals(seen.consents[0].version, TERMS_VERSION, "the catalog's terms version");
    assertEquals(
      seen.calls,
      THE_PURCHASE.map((c) => (c === "getPrice:" ? `getPrice:${PRICE_ID[plan]}` : c)),
      `${plan}: the account, the gates, the price check, then the consent row and the session`,
    );
  }
});

Deno.test("the_offer_is_the_catalogs", async () => {
  const get = (auth: boolean) =>
    new Request("http://127.0.0.1:1/", {
      method: "GET",
      headers: auth ? { authorization: "Bearer good" } : {},
    });
  {
    const { deps, seen } = fakes();
    const res = await run(get(true), deps);
    assertEquals(res.status, 200);
    const body = await res.json();
    assertEquals(body, offer(TABLE));
    assertEquals(Object.keys(body).sort(), ["note", "plans", "terms_version"]);
    assertEquals(
      body.plans.map((p: { plan: string }) => p.plan),
      ["yearly", "four_weekly"],
      "the catalog's order",
    );
    for (const p of body.plans) assertEquals(Object.keys(p).sort(), ["amount_cents", "lines", "plan"]);
    assertEquals(seen.calls, [], "the offer reads no account and calls no Stripe");
  }
  {
    // An invented catalog at 1399: the answer is the catalog's, not a copy held anywhere else.
    const invented: CatalogDef = {
      ...TABLE,
      selling: [{ key: "yearly", cents: 9087 }, { key: "four_weekly", cents: 1399 }],
    };
    const { deps } = fakes({ catalog: catalogOf(invented) });
    const res = await run(get(true), deps);
    const body = await res.json();
    assertEquals(body.plans[1], {
      plan: "four_weekly",
      amount_cents: 1399,
      lines: ["$13.99 every 4 weeks."],
    });
    assert(JSON.stringify(body).includes("$13.99"));
  }
  {
    const { deps, seen } = fakes();
    const res = await run(get(false), deps);
    assertEquals(res.status, 401);
    assertEquals(seen.calls, []);
  }
  {
    const { deps } = fakes();
    const res = await run(new Request("http://127.0.0.1:1/", { method: "PUT" }), deps);
    assertEquals(res.status, 405);
    assertEquals(await res.json(), { error: "method not allowed; use GET, POST" });
    assertEquals(res.headers.get("allow"), "GET, POST");
  }
});

Deno.test("an_older_build_is_told_to_update", async () => {
  for (const plan of ["monthly", "academic_year"]) {
    const { deps, seen } = fakes();
    // Exactly what a pre-v0.4.1 build sends.
    const res = await run(purchase({ plan, terms_version: "2026-09-10" }), deps);
    assertEquals(res.status, 409, plan);
    assertEquals(await res.json(), {
      error: "This version of Knowlu can't subscribe any more. Update Knowlu, then try again.",
    });
    assertEquals(
      seen.calls,
      [],
      `${plan}: no account read, no tester read, no customer, no consent row, no session`,
    );
  }
});

Deno.test("an_unknown_plan_is_still_400", async () => {
  const { deps, seen } = fakes();
  const res = await run(
    purchase({ plan: "lifetime", amount_cents: 1299, terms_version: TERMS_VERSION }),
    deps,
  );
  assertEquals(res.status, 400);
  assertEquals(await res.json(), { error: "unknown plan; use yearly or four_weekly" });
  assertEquals(seen.calls, []);
});

Deno.test("an_offer_with_a_missing_field_is_400", async () => {
  for (
    const [body, field] of [
      [{ plan: "four_weekly", terms_version: TERMS_VERSION }, "amount_cents"],
      [{ plan: "yearly", amount_cents: 8437 }, "terms_version"],
      [{ plan: "four_weekly", amount_cents: null, terms_version: TERMS_VERSION }, "amount_cents"],
    ] as const
  ) {
    const { deps, seen } = fakes();
    const res = await run(purchase(body), deps);
    assertEquals(res.status, 400, JSON.stringify(body));
    const { error } = await res.json();
    assert(String(error).startsWith(`${field} `), `the refusal names ${field}: ${error}`);
    assertEquals(seen.calls, [], JSON.stringify(body));
  }
});

Deno.test("a_changed_offer_is_refused", async () => {
  for (
    const body of [
      { plan: "four_weekly", amount_cents: 1298, terms_version: TERMS_VERSION },
      { plan: "four_weekly", amount_cents: 8437, terms_version: TERMS_VERSION },
      { plan: "four_weekly", amount_cents: 1299, terms_version: "2026-09-09" },
      // Present but not equal, whatever its type (note D3: "Anything else is refused with 409").
      { plan: "four_weekly", amount_cents: "1299", terms_version: TERMS_VERSION },
      { plan: "four_weekly", amount_cents: 12.99, terms_version: TERMS_VERSION },
      // The terms version's own digits, as a number.
      { plan: "four_weekly", amount_cents: 1299, terms_version: Number(TERMS_VERSION.replaceAll("-", "")) },
    ]
  ) {
    const { deps, seen } = fakes();
    const res = await run(purchase(body), deps);
    assertEquals(res.status, 409, JSON.stringify(body));
    assertEquals(await res.json(), {
      error: "Knowlu's prices or terms have changed. Look them over again, then subscribe.",
    });
    assertEquals(seen.calls, [], "refused before any account read");
  }
});

Deno.test("the_trial_is_fourteen_days_on_a_first_subscription_only", async () => {
  // No row; and the sign-up row, which carries no subscription id.
  for (const row of [null, { stripe_subscription_id: null, status: "none" }]) {
    const { deps, seen } = fakes({ subscriptionState: () => Promise.resolve(row) });
    const res = await run(buy("four_weekly"), deps);
    assertEquals(res.status, 200);
    assertEquals(seen.forms[0]["subscription_data[trial_period_days]"], "14", JSON.stringify(row));
    assertEquals(seen.forms[0]["payment_method_collection"], "always");
  }
  // A subscription that came before: no trial, whatever its status now.
  for (const status of ["canceled", "none"]) {
    const { deps, seen } = fakes({
      subscriptionState: () => Promise.resolve({ stripe_subscription_id: "sub_old", status }),
    });
    const res = await run(buy("yearly"), deps);
    assertEquals(res.status, 200, status);
    assert(!("subscription_data[trial_period_days]" in seen.forms[0]), `${status}: no trial key`);
    assertEquals(seen.forms[0]["payment_method_collection"], "always");
  }
});

Deno.test("an_account_with_a_live_subscription_is_refused", async () => {
  for (const status of ["active", "trialing", "past_due", "unpaid"]) {
    const { deps, seen } = fakes({
      subscriptionState: (
        id,
      ) => (seen.calls.push(`subscriptionState:${id}`),
        Promise.resolve({ stripe_subscription_id: "sub_live", status })),
    });
    const res = await run(buy("yearly"), deps);
    assertEquals(res.status, 409, status);
    assertEquals(await res.json(), {
      error: "This account already has a subscription. Manage it in Settings, then Membership.",
    });
    assertEquals(
      seen.calls,
      ["getAccount", "isTester", "subscriptionState:acc-1"],
      `${status}: before any price read, customer or consent row`,
    );
  }
});

Deno.test("the_price_is_checked_before_it_is_sold", async () => {
  type Price = Record<string, unknown>;
  const wrong = (plan: string, change: (p: Price) => Price) => ({ plan, change });
  const cases: { name: string; plan: string; change: (p: Price) => Price; field: string }[] = [
    {
      name: "a wrong amount",
      ...wrong("four_weekly", (p) => ({ ...p, unit_amount: 1298 })),
      field: "unit_amount",
    },
    {
      name: "a wrong currency",
      ...wrong("four_weekly", (p) => ({ ...p, currency: "eur" })),
      field: "currency",
    },
    { name: "an archived price", ...wrong("yearly", (p) => ({ ...p, active: false })), field: "active" },
    {
      name: "tax included",
      ...wrong("yearly", (p) => ({ ...p, tax_behavior: "inclusive" })),
      field: "tax_behavior",
    },
    {
      name: "a monthly interval",
      ...wrong("four_weekly", (p) => ({ ...p, recurring: { interval: "month", interval_count: 4 } })),
      field: "interval",
    },
    {
      name: "every week, not every 4",
      ...wrong("four_weekly", (p) => ({ ...p, recurring: { interval: "week", interval_count: 1 } })),
      field: "interval_count",
    },
    {
      name: "four_weekly pointing at a yearly price at its own amount",
      ...wrong("four_weekly", (p) => ({ ...p, recurring: { interval: "year", interval_count: 1 } })),
      field: "interval",
    },
    {
      name: "yearly pointing at a 4-week price at its own amount",
      ...wrong("yearly", (p) => ({ ...p, recurring: { interval: "week", interval_count: 4 } })),
      field: "interval",
    },
  ];
  for (const c of cases) {
    const logged: string[] = [];
    const original = console.error;
    console.error = (...a: unknown[]) => void logged.push(a.map(String).join(" "));
    try {
      const catalog = catalogOf();
      const { deps, seen } = fakes({
        catalog,
        getPrice: (
          id,
        ) => (seen.calls.push(`getPrice:${id}`), Promise.resolve(c.change(goodPrice(catalog, id)!))),
      });
      const res = await run(buy(c.plan), deps);
      assertEquals(res.status, 500, c.name);
      assertEquals(await res.json(), { error: "the function is not configured" }, c.name);
      assertEquals(seen.consents, [], `${c.name}: no consent row`);
      assertEquals(seen.forms, [], `${c.name}: no session`);
      assert(!seen.calls.includes("/v1/customers"), `${c.name}: no customer`);
      assertEquals(seen.calls.at(-1), `getPrice:${PRICE_ID[c.plan]}`, c.name);
      assert(logged.some((l) => l.includes(c.field)), `${c.name}: the log names ${c.field}: ${logged}`);
    } finally {
      console.error = original;
    }
  }
  // (r4) A selling variable left unset in the resolved catalog: refused before Stripe is asked.
  {
    const unset = catalogOf(TABLE, [priceVariable({ key: "four_weekly", cents: 1299 })]);
    const original = console.error;
    console.error = () => {};
    try {
      const { deps, seen } = fakes({ catalog: unset });
      const res = await run(buy("four_weekly"), deps);
      assertEquals(res.status, 500);
      assertEquals(await res.json(), { error: "the function is not configured" });
      assertEquals(seen.consents, []);
      assertEquals(seen.forms, []);
      assert(!seen.calls.some((c) => c.startsWith("getPrice") || c === "/v1/customers"), seen.calls.join());
      // The other plan still sells.
      const other = fakes({ catalog: unset });
      assertEquals((await run(buy("yearly"), other.deps)).status, 200);
    } finally {
      console.error = original;
    }
  }
  // A price Stripe would not hand back (an id from the other Stripe mode, say) is not sold either, and
  // it is the same 500 as every other D4 failure, never a 502 that reads as a passing outage.
  {
    const original = console.error;
    console.error = () => {};
    try {
      const { deps, seen } = fakes({ getPrice: () => Promise.resolve(null) });
      const res = await run(buy("yearly"), deps);
      assertEquals(res.status, 500);
      assertEquals(await res.json(), { error: "the function is not configured" });
      assertEquals(seen.consents, []);
      assertEquals(seen.forms, []);
      assert(!seen.calls.includes("/v1/customers"), "no customer");
    } finally {
      console.error = original;
    }
  }
});

Deno.test("the_form_keeps_every_load_bearing_key", () => {
  const form = (trial: boolean) =>
    checkoutForm({
      price: "price_test_yearly",
      customer: "cus_1",
      accountId: "acc-1",
      successUrl: "https://knowlu.com/subscribed.html",
      cancelUrl: "https://knowlu.com/index.html",
      trial,
    });
  for (const trial of [true, false]) {
    const f = form(trial);
    assertEquals(f["mode"], "subscription");
    assertEquals(f["line_items[0][price]"], "price_test_yearly");
    assertEquals(f["line_items[0][quantity]"], "1");
    // D7's load-bearing keys. `customer`: the portal and account deletion depend on it.
    assertEquals(f["customer"], "cus_1");
    // How the webhook finds the account.
    assertEquals(f["client_reference_id"], "acc-1");
    assertEquals(f["subscription_data[metadata][account_id]"], "acc-1");
    // R-C1b-2. `always` STAYS: `if_required` collects a card only when the first invoice has an
    // amount due, and `subscription_data[trial_period_days]` makes that invoice zero for every
    // subscription, so it would take a card from nobody and falsify the terms' "card taken first".
    assertEquals(f["payment_method_collection"], "always");
    // §8 of the legal note: express informed consent to the auto-renewal terms, its own checkbox,
    // which the consent row presumes.
    assertEquals(f["consent_collection[terms_of_service]"], "required");
    // D5: 14 days, on a first subscription only.
    if (trial) assertEquals(f["subscription_data[trial_period_days]"], "14");
    else assert(!("subscription_data[trial_period_days]" in f));
    // Checkpoint A: Managed Payments, per session.
    assertEquals(f["managed_payments[enabled]"], "true");
    // Not refused under Managed Payments (T0r Q4), so they stay.
    assertEquals(f["allow_promotion_codes"], "true");
    assertEquals(f["billing_address_collection"], "required");
    assertEquals(f["success_url"], "https://knowlu.com/subscribed.html");
    assertEquals(f["cancel_url"], "https://knowlu.com/index.html");
  }
  // The keys v0.4.0's form carried (`handler.ts:51-72` on main at 8c78e1f8). The ones that left are
  // exactly checkpoint A's list, the two Stripe documents refusing under Managed Payments. The one that
  // came is Managed Payments itself: `custom_text`, which Stripe also refuses beside it (T1c,
  // 2026-11-27), never joins.
  const before = [
    "mode",
    "line_items[0][price]",
    "line_items[0][quantity]",
    "customer",
    "client_reference_id",
    "subscription_data[metadata][account_id]",
    "subscription_data[trial_period_days]",
    "payment_method_collection",
    "allow_promotion_codes",
    "automatic_tax[enabled]",
    "customer_update[address]",
    "billing_address_collection",
    "consent_collection[terms_of_service]",
    "success_url",
    "cancel_url",
  ];
  const now = Object.keys(form(true));
  assertEquals(before.filter((k) => !now.includes(k)).sort(), [
    "automatic_tax[enabled]",
    "customer_update[address]",
  ]);
  assertEquals(now.filter((k) => !before.includes(k)).sort(), ["managed_payments[enabled]"]);
  // And no API version rides on the form: the pin in `_shared/stripe.ts` serves the session call.
  assert(!now.some((k) => /version/i.test(k)));
});

// T1c (found 2026-11-27 on staging): beside `managed_payments[enabled]`, Stripe refuses the session with
// "You cannot use custom_text with Managed Payments." in every slot (`submit`,
// `terms_of_service_acceptance`, `after_submit`), so no key of the form may start with `custom_text`.
Deno.test("the_form_sets_no_custom_text_under_managed_payments", async () => {
  const noCustomText = (form: Record<string, string>, what: string) => {
    assertEquals(form["managed_payments[enabled]"], "true", what);
    assertEquals(Object.keys(form).filter((k) => k.startsWith("custom_text")), [], what);
  };
  // The form itself, with and without the trial.
  for (const trial of [true, false]) {
    noCustomText(
      checkoutForm({
        price: "price_test_yearly",
        customer: "cus_1",
        accountId: "acc-1",
        successUrl: "https://knowlu.com/subscribed.html",
        cancelUrl: "https://knowlu.com/index.html",
        trial,
      }),
      `the form, trial ${trial}`,
    );
  }
  // And the session the handler sends, for both plans, on a first subscription and after one.
  for (const plan of ["yearly", "four_weekly"]) {
    for (
      const row of [null, { stripe_subscription_id: "sub_old", status: "canceled" }]
    ) {
      const { deps, seen } = fakes({ subscriptionState: () => Promise.resolve(row) });
      assertEquals((await run(buy(plan), deps)).status, 200, plan);
      assertEquals(seen.forms.length, 1);
      assertEquals("subscription_data[trial_period_days]" in seen.forms[0], row === null, plan);
      noCustomText(seen.forms[0], `${plan}, trial ${row === null}`);
    }
  }
});

Deno.test("an account with no Stripe customer gets one, once, and it is saved", async () => {
  let saved: [string, string] | null = null;
  const { deps, seen } = fakes({
    getAccount: () =>
      Promise.resolve({
        email: "a@example.invalid",
        stripe_customer_id: null,
        age_attested_at: "2026-09-17T12:00:00Z",
      }),
    saveCustomerId: (a, c) => {
      saved = [a, c];
      return Promise.resolve();
    },
  });
  const res = await run(buy("four_weekly"), deps);
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { url: "https://checkout.stripe.com/c/cs_1" });
  assertEquals(seen.calls.filter((c) => c.startsWith("/v1/")), ["/v1/customers", "/v1/checkout/sessions"]);
  assertEquals(saved, ["acc-1", "cus_new"]);
});

Deno.test("the auto-renew consent is logged with its version and the price, before the redirect", async () => {
  const { deps, seen } = fakes();
  await run(buy("yearly", { "x-forwarded-for": "203.0.113.7, 10.0.0.1" }), deps);
  assertEquals(seen.consents, [{
    account_id: "acc-1",
    subject_email: "a@example.invalid",
    kind: "auto_renew",
    version: TERMS_VERSION,
    price_cents: 8437,
    ip: "203.0.113.7",
  }]);
  assert(seen.calls.indexOf("recordConsent") < seen.calls.indexOf("/v1/checkout/sessions"));
});

Deno.test("R-C1-59 (M4): a malformed x-forwarded-for never 502s a checkout — the consent row just carries no ip", async () => {
  const { deps, seen } = fakes();
  // Not an IP at all — the shape a hostile or misconfigured proxy could hand the function.
  const res = await run(buy("four_weekly", { "x-forwarded-for": "definitely not an ip" }), deps);
  assertEquals(res.status, 200);
  assertEquals(seen.consents.length, 1);
  assertEquals(seen.consents[0].ip, null);
});

Deno.test("an account that never attested to being 18 cannot reach Stripe", async () => {
  let touched = false;
  const { deps } = fakes({
    // Migration 20260917000100 lets an OAuth sign-up land here with a null attestation; this is
    // the server-side tooth that replaced the trigger's `raise`.
    getAccount: () =>
      Promise.resolve({ email: "a@example.invalid", stripe_customer_id: null, age_attested_at: null }),
    recordConsent: () => {
      touched = true;
      return Promise.resolve();
    },
    stripe: () => {
      touched = true;
      return Promise.resolve({});
    },
  });
  const res = await run(buy("four_weekly"), deps);
  assertEquals(res.status, 403);
  assert(!touched, "no customer, no consent row, no session");
});

Deno.test("an account that did attest goes through exactly as before", async () => {
  const { deps } = fakes();
  const res = await run(buy("four_weekly"), deps);
  assertEquals(res.status, 200);
});

// Friends testers (docs/specs/2026-10-18-friends-testers-design.md §3.4, Q3, §9 "Functions"; L8 S4):
// while `effective_entitlements` answers `plan = 'tester'` for the caller, checkout is refused with a
// 409, so no path, old build or stale page can start a charge. The fake lookup answers from the case
// table's own expectation for the view, so an expired (P8) or ended (P9) grant is decided where the
// view decides it, and those callers reach Checkout exactly as today.
const viewSaysTester = (id: string): boolean => {
  const c = CASES.find((x) => x.id === id);
  if (!c) throw new Error(`no case ${id} in testers_cases.ts`);
  return c.expect[0]?.plan === "tester";
};

function testerDeps(caseId: string, seen: string[]): Deps {
  const catalog = catalogOf();
  return {
    verify: () => Promise.resolve({ id: "acc-1", email: "a@example.invalid" }),
    getAccount: () =>
      Promise.resolve({
        email: "a@example.invalid",
        stripe_customer_id: null,
        age_attested_at: "2026-09-17T12:00:00Z",
      }),
    isTester: (id) => {
      seen.push(`isTester:${id}`);
      return Promise.resolve(viewSaysTester(caseId));
    },
    subscriptionState: () => Promise.resolve(null),
    getPrice: (id) => Promise.resolve(goodPrice(catalog, id)),
    listOpenCheckouts: () => (seen.push("listOpenCheckouts"), Promise.resolve(NOTHING_OPEN)),
    expireCheckout: () => (seen.push("expireCheckout"), Promise.resolve({})),
    saveCustomerId: () => (seen.push("saveCustomerId"), Promise.resolve()),
    recordConsent: () => (seen.push("recordConsent"), Promise.resolve()),
    stripe: (path) => {
      seen.push(path);
      return Promise.resolve(
        path === "/v1/customers"
          ? { id: "cus_new" }
          : { id: "cs_t", url: "https://checkout.stripe.com/c/cs_t" },
      );
    },
    catalog,
    successUrl: "https://knowlu.com/subscribed.html",
    cancelUrl: "https://knowlu.com/index.html",
  };
}

const checkoutReq = () => buy("four_weekly");

Deno.test("a caller the view shows as tester is 409, and no customer, consent row or session is made", async () => {
  for (const id of ["P3", "P3x", "P5", "P6"]) {
    assert(viewSaysTester(id), `${id} must be a tester row in the case table`);
    const seen: string[] = [];
    const res = await handle(checkoutReq(), testerDeps(id, seen)).catch((e) => e as Response);
    assertEquals(res.status, 409, id);
    assertEquals(await res.json(), { error: "this account has free tester access" }, id);
    assertEquals(
      seen,
      ["isTester:acc-1"],
      `${id}: the lookup is scoped to the caller and nothing follows it`,
    );
  }
});

Deno.test("an expired (P8) or ended (P9) grant, and an unpaid or paid account, reach Checkout as today", async () => {
  for (const id of ["P8", "P9", "P0", "P1"]) {
    assert(!viewSaysTester(id), `${id} must not be a tester row in the case table`);
    const seen: string[] = [];
    const res = await handle(checkoutReq(), testerDeps(id, seen));
    assertEquals(res.status, 200, id);
    assertEquals(await res.json(), { url: "https://checkout.stripe.com/c/cs_t" }, id);
    assertEquals(
      seen,
      ["isTester:acc-1", "/v1/customers", "saveCustomerId", "recordConsent", "/v1/checkout/sessions"],
      `${id}: the lookup comes first, then today's path unchanged`,
    );
  }
});

// `index.ts` is never loaded by `deno test` (it reads `Deno.env` and calls `Deno.serve`), so a fake in
// this file alone would leave production never reading the view (§3.4). The precedent is
// `account/handler_test.ts`'s `sources` guard, which reads its `index.ts` as text.
Deno.test("index.ts wires isTester to effective_entitlements, by the caller's account id, selecting plan", async () => {
  const text = await Deno.readTextFile(new URL("./index.ts", import.meta.url));
  const m = text.match(/isTester: async \(id\) => \{[\s\S]*?\n {6}\},/);
  assert(m, "could not find isTester's body in index.ts");
  const body = m[0];
  assert(/restSelect<[^>]*>\(\s*rest,\s*"effective_entitlements",/.test(body), "isTester must read the view");
  assert(body.includes("`account_id=eq.${encodeURIComponent(id)}&select=plan&limit=1`"), body);
  assert(body.includes('=== "tester"'), "isTester must answer true only for plan tester");
  assert(!/"entitlements"|comp_grants/.test(body), "isTester reads the view, never the table or the grants");
});

Deno.test("index.ts resolves the catalog from Deno.env.get and holds no cents map", async () => {
  const text = await Deno.readTextFile(new URL("./index.ts", import.meta.url));
  assert(/from "\.\.\/_shared\/price_catalog\.ts"/.test(text), "index.ts imports the catalog");
  assert(
    /resolveCatalog\(\s*CATALOG,\s*\(name\)\s*=>\s*Deno\.env\.get\(name\)\s*\)/.test(text),
    "the catalog is resolved from Deno.env.get",
  );
  assert(/\bcatalog[,:]/.test(text), "and handed to the handler");
  assert(!/STRIPE_PRICE_/.test(text), "no price variable is named here");
  const code = text.replace(/\/\/.*$/gm, "").replace(/fail\(\d{3},/g, "fail(");
  assert(!/\b\d{3,}\b/.test(code), "no cents, outside comments and HTTP statuses");
  assert(!/Record<Plan,\s*number>|priceCentsFor|priceFor/.test(text), "the cents map is gone");
  // `subscriptionState` reads the entitlements table (not the view), scoped to the caller.
  const m = text.match(/subscriptionState: async \(id\) => \{[\s\S]*?\n {6}\},/);
  assert(m, "could not find subscriptionState's body in index.ts");
  assert(/"entitlements",/.test(m[0]), m[0]);
  assert(
    m[0].includes("`account_id=eq.${encodeURIComponent(id)}&select=stripe_subscription_id,status&limit=1`"),
    m[0],
  );
  // `getPrice` reads through the pinned GET helper.
  assert(/getPrice: \(id\) =>\s*stripeGetFrom\(/.test(text), "getPrice goes through stripeGetFrom");
  assert(text.includes("`/v1/prices/${encodeURIComponent(id)}`"), "one price, path-encoded");
});

// T1b (the v0.4.1 cloud review, finding 6): before a new Checkout Session is made, every page this
// customer still has open is expired, so two pages opened together can never both be paid. Any failure
// on the way is a 502 in the payment provider's words, with no consent row and no session; the student
// presses again, and by then the webhook has recorded a completed page and D5's check answers.

const LIST_REFUSED = "the payment provider did not list this account's open checkout pages";
const EXPIRE_REFUSED = "the payment provider did not close an earlier checkout page";
const LIST_UNENDING = "the payment provider's list of open checkout pages did not end";

/** One open Checkout Session, as Stripe lists it: only the fields the handler reads. */
const openSession = (id: string, customer = "cus_1", status = "open") => ({
  id,
  object: "checkout.session",
  customer,
  status,
});

/** One page of Stripe's list of `cus_1`'s open sessions. */
const openList = (ids: string[], hasMore = false): Record<string, unknown> => ({
  object: "list",
  data: ids.map((id) => openSession(id)),
  has_more: hasMore,
});

/** The purchase's calls up to the price check, then `step`. */
const upToThePrice = (plan: string, step: string[]) => [
  "getAccount",
  "isTester",
  "subscriptionState",
  `getPrice:${PRICE_ID[plan]}`,
  ...step,
];

/** The whole purchase, with `step` between the price check and the consent row. */
const withExpiry = (plan: string, step: string[]) => [
  ...upToThePrice(plan, step),
  "recordConsent",
  "/v1/checkout/sessions",
];

/** `console.error`, captured for the length of one body. */
async function capturingLogs<T>(body: (logged: string[]) => Promise<T>): Promise<T> {
  const logged: string[] = [];
  const original = console.error;
  console.error = (...a: unknown[]) => void logged.push(a.map(String).join(" "));
  try {
    return await body(logged);
  } finally {
    console.error = original;
  }
}

/** Neither a refusal nor a log line names a session, the customer, the account or the address. */
function namesNobody(text: string, what: string) {
  for (const id of ["cs_open", "cus_", "acc-1", "example.invalid"]) {
    assert(!text.includes(id), `${what} names ${id}: ${text}`);
  }
}

Deno.test("open_checkout_pages_are_expired_before_a_new_one", async () => {
  // One open page, and two. The fake answers the same list on every call, as a list that has not
  // caught up with the expires would: a short list is the end, so it is asked for exactly once.
  for (
    const [plan, open] of [
      ["four_weekly", ["cs_open_a"]],
      ["yearly", ["cs_open_a"]],
      ["four_weekly", ["cs_open_a", "cs_open_b"]],
      ["yearly", ["cs_open_a", "cs_open_b"]],
    ] as const
  ) {
    const { deps, seen } = fakes({
      listOpenCheckouts: (
        customer,
      ) => (seen.calls.push(`list:${customer}`), Promise.resolve(openList([...open]))),
    });
    const res = await run(buy(plan), deps);
    assertEquals(res.status, 200, `${plan}, ${open.length} open`);
    assertEquals(await res.json(), { url: "https://checkout.stripe.com/c/cs_1" });
    assertEquals(
      seen.calls,
      withExpiry(plan, ["list:cus_1", ...open.map((id) => `expire:${id}`)]),
      `${plan}, ${open.length} open: one list call, each page expired, then the consent row and the session`,
    );
    // The new page is today's: the same form and the same consent row.
    assertEquals(seen.forms, [checkoutForm({
      price: PRICE_ID[plan],
      customer: "cus_1",
      accountId: "acc-1",
      successUrl: "https://knowlu.com/subscribed.html",
      cancelUrl: "https://knowlu.com/index.html",
      trial: true,
    })]);
    assertEquals(seen.consents.length, 1);
    assertEquals(seen.consents[0].price_cents, OFFERED[plan]);
  }
});

Deno.test("a_new_customer_has_no_pages_to_expire", async () => {
  const { deps, seen } = fakes({
    getAccount: () => {
      seen.calls.push("getAccount");
      return Promise.resolve({
        email: "a@example.invalid",
        stripe_customer_id: null,
        age_attested_at: "2026-09-17T12:00:00Z",
      });
    },
    listOpenCheckouts: (customer) => (
      seen.calls.push(`list:${customer}`), Promise.resolve(openList(["cs_open_a"]))
    ),
  });
  const res = await run(buy("four_weekly"), deps);
  assertEquals(res.status, 200);
  assertEquals(
    seen.calls,
    withExpiry("four_weekly", ["/v1/customers", "saveCustomerId"]),
    "a customer made in this request has no pages: no list call, nothing expired",
  );
  assertEquals(seen.forms[0]["customer"], "cus_new");
});

Deno.test("a_page_that_cannot_be_expired_stops_the_checkout", async () => {
  const cases: {
    name: string;
    list: () => Promise<Record<string, unknown> | null>;
    expire?: (id: string) => Promise<unknown>;
    error: string;
    step: string[];
  }[] = [
    {
      // What `stripePostFrom` throws when Stripe refuses, as it does when the page was completed a
      // moment ago and is no longer open.
      name: "an expire Stripe refuses",
      list: () => Promise.resolve(openList(["cs_open_a", "cs_open_b"])),
      expire: () =>
        Promise.reject(Response.json({ error: "the payment provider refused the request" }, { status: 502 })),
      error: EXPIRE_REFUSED,
      step: ["list:cus_1", "expire:cs_open_a"],
    },
    {
      // A network failure: Deno's message carries the URL, so the handler must not log it.
      name: "an expire that never reached Stripe",
      list: () => Promise.resolve(openList(["cs_open_a", "cs_open_b"])),
      expire: (id) =>
        Promise.reject(new Error(`error sending request for url (/v1/checkout/sessions/${id}/expire)`)),
      error: EXPIRE_REFUSED,
      step: ["list:cus_1", "expire:cs_open_a"],
    },
    {
      name: "a list Stripe refuses (`stripeGetFrom` answers null on any non-2xx)",
      list: () => Promise.resolve(null),
      error: LIST_REFUSED,
      step: ["list:cus_1"],
    },
    {
      name: "a list call that never reached Stripe",
      list: () =>
        Promise.reject(new Error("error sending request for url (/v1/checkout/sessions?customer=cus_1)")),
      error: LIST_REFUSED,
      step: ["list:cus_1"],
    },
    {
      name: "a list with no data",
      list: () => Promise.resolve({ object: "list", has_more: false }),
      error: LIST_REFUSED,
      step: ["list:cus_1"],
    },
    {
      name: "a listed session with no id",
      list: () =>
        Promise.resolve({ object: "list", data: [{ object: "checkout.session" }], has_more: false }),
      error: LIST_REFUSED,
      step: ["list:cus_1"],
    },
    {
      // Review M3: an id, but no customer at all.
      name: "a listed session with an id and no customer",
      list: () =>
        Promise.resolve({
          object: "list",
          data: [{ id: "cs_open_a", object: "checkout.session", status: "open" }],
          has_more: false,
        }),
      error: LIST_REFUSED,
      step: ["list:cus_1"],
    },
    // Review M5: an id goes into a URL path, so it must look like a Checkout Session's before it is
    // used. `.` and `..` survive `encodeURIComponent` and the URL parser would resolve them as path
    // segments. Each sits beside a good session, and nothing on the list is expired.
    ...([["..", "a dot-dot id"], [".", "a dot id"], ["", "an empty id"], [42, "a number for an id"], [
      "pi_open_a",
      "an id that is not a Checkout Session's",
    ], ["cs_open/a", "an id with a slash"]] as const).map(([id, name]) => ({
      name,
      list: () =>
        Promise.resolve({
          object: "list",
          data: [openSession("cs_open_a"), {
            id,
            object: "checkout.session",
            customer: "cus_1",
            status: "open",
          }],
          has_more: false,
        }),
      error: LIST_REFUSED,
      step: ["list:cus_1"],
    })),
    {
      // A reply that ignored the customer filter must never expire a stranger's page: nothing on the
      // page is expired, not even the first session, which is this customer's.
      name: "a listed session of another customer",
      list: () =>
        Promise.resolve({
          object: "list",
          data: [openSession("cs_open_a"), openSession("cs_open_x", "cus_other")],
          has_more: false,
        }),
      error: LIST_REFUSED,
      step: ["list:cus_1"],
    },
    {
      name: "a listed session that is not open",
      list: () =>
        Promise.resolve({
          object: "list",
          data: [openSession("cs_open_a", "cus_1", "complete")],
          has_more: false,
        }),
      error: LIST_REFUSED,
      step: ["list:cus_1"],
    },
  ];
  for (const c of cases) {
    await capturingLogs(async (logged) => {
      const { deps, seen } = fakes({
        listOpenCheckouts: (customer) => (seen.calls.push(`list:${customer}`), c.list()),
        ...(c.expire
          ? { expireCheckout: (id: string) => (seen.calls.push(`expire:${id}`), c.expire!(id)) }
          : {}),
      });
      const res = await run(buy("yearly"), deps);
      assertEquals(res.status, 502, c.name);
      const text = await res.text();
      assertEquals(JSON.parse(text), { error: c.error }, c.name);
      assertEquals(
        seen.calls,
        upToThePrice("yearly", c.step),
        `${c.name}: nothing after the failure is called`,
      );
      assertEquals(seen.consents, [], `${c.name}: no consent row`);
      assertEquals(seen.forms, [], `${c.name}: no session`);
      assert(logged.length > 0, `${c.name}: the failure is logged`);
      namesNobody(text, `${c.name}: the refusal`);
      namesNobody(logged.join("\n"), `${c.name}: the log`);
    });
  }
});

Deno.test("a_short_list_ends_it_and_a_full_one_is_listed_again", async () => {
  /** `count` invented session ids, from `cs_open_<from>`. */
  const ids = (from: number, count: number) => Array.from({ length: count }, (_, i) => `cs_open_${from + i}`);
  /** A list call as recorded: the customer, and anything else it was ever handed (a cursor, say). */
  const listed = (customer: string, rest: unknown[]) =>
    rest.length === 0 ? `list:${customer}` : `list:${customer}:${rest.join(",")}`;
  /** A purchase against a list whose `n`th call (from 0) answers `reply(n)`. */
  const against = (reply: (n: number) => Record<string, unknown>) =>
    capturingLogs(async (logged) => {
      let n = 0;
      const { deps, seen } = fakes({
        listOpenCheckouts: (customer, ...rest) => (
          seen.calls.push(listed(customer, rest)), Promise.resolve(reply(n++))
        ),
      });
      const res = await run(buy("four_weekly"), deps);
      const text = await res.text();
      assert(!seen.calls.some((x) => x.startsWith("list:cus_1:")), "no list call carries a cursor");
      namesNobody(text, "the answer");
      namesNobody(logged.join("\n"), "the log");
      return { status: res.status, text, seen };
    });
  /** What a list call past the expected last one would find: a page no test expects to see expired. */
  const UNEXPECTED = openList(["cs_open_unexpected"]);
  const lists = (calls: string[]) => calls.filter((x) => x.startsWith("list:")).length;
  const expires = (calls: string[]) => calls.filter((x) => x.startsWith("expire:"));

  // 100, then 1: a full list is listed again from the start, and the short one ends it. `has_more`
  // steers nothing (the first list says more follow).
  {
    const { status, seen } = await against((n) =>
      [openList(ids(1, 100), true), openList(ids(101, 1))][n] ?? UNEXPECTED
    );
    assertEquals(status, 200);
    assertEquals(
      seen.calls,
      withExpiry("four_weekly", [
        "list:cus_1",
        ...ids(1, 100).map((id) => `expire:${id}`),
        "list:cus_1",
        "expire:cs_open_101",
      ]),
      "two list calls, 101 expires, no third list",
    );
    assertEquals(seen.forms.length, 1);
  }
  // A lagging list: the second round still shows sessions this request expired in the first. Those
  // are skipped, the new one is expired, and a short list ends it. (A `limit=100` list holds at most
  // 100, so the lag shown is the last 50 of round one's 100 beside the one new session.)
  {
    const { status, seen } = await against((n) =>
      [openList(ids(1, 100)), openList([...ids(51, 50), "cs_open_101"])][n] ?? UNEXPECTED
    );
    assertEquals(status, 200);
    assertEquals(
      seen.calls,
      withExpiry("four_weekly", [
        "list:cus_1",
        ...ids(1, 100).map((id) => `expire:${id}`),
        "list:cus_1",
        "expire:cs_open_101",
      ]),
      "the 100 are not expired twice, the new one is, and it ends there",
    );
  }
  // The same lag, with the list already calling those 50 `expired`: still this request's own, still
  // skipped. A session this request did not expire must be `open` (`a listed session that is not open`).
  {
    const lagging = {
      object: "list",
      data: [...ids(51, 50).map((id) => openSession(id, "cus_1", "expired")), openSession("cs_open_101")],
      has_more: false,
    };
    const { status, seen } = await against((n) => [openList(ids(1, 100)), lagging][n] ?? UNEXPECTED);
    assertEquals(status, 200);
    assertEquals(expires(seen.calls), [...ids(1, 100), "cs_open_101"].map((id) => `expire:${id}`));
    assertEquals(lists(seen.calls), 2);
  }
  // A full round with nothing new to expire: the list is not catching up, and what lies past it cannot
  // be seen, so it is the refusal, with nothing written.
  {
    const { status, text, seen } = await against((n) => n < 2 ? openList(ids(1, 100)) : UNEXPECTED);
    assertEquals(status, 502);
    assertEquals(JSON.parse(text), { error: LIST_UNENDING });
    assertEquals(lists(seen.calls), 2);
    assertEquals(expires(seen.calls), ids(1, 100).map((id) => `expire:${id}`), "each expired once");
    assertEquals(seen.consents, []);
    assertEquals(seen.forms, []);
  }
  // The bound's far edge (review M2): four full lists of new sessions, then a short one, is the most
  // the rounds clear, and it is a 200 with every one of the 401 expired.
  {
    const { status, seen } = await against((n) =>
      n < 4 ? openList(ids(n * 100 + 1, 100)) : n === 4 ? openList(["cs_open_401"]) : UNEXPECTED
    );
    assertEquals(status, 200);
    assertEquals(lists(seen.calls), 5);
    assertEquals(expires(seen.calls), ids(1, 401).map((id) => `expire:${id}`));
    assertEquals(seen.forms.length, 1);
  }
  // The bound: a list that is full of new sessions every round ends in the refusal after
  // OPEN_ROUND_BOUND rounds, not in a loop, with nothing written. The fake gives out after twice the
  // bound (review M2), so code that lost its bound fails here at once rather than hanging the suite.
  {
    assertEquals(OPEN_ROUND_BOUND, 5);
    const { status, text, seen } = await against((n) => {
      if (n >= 2 * OPEN_ROUND_BOUND) throw new Error("the list was asked for past twice the bound");
      return openList(ids(n * 100 + 1, 100));
    });
    assertEquals(status, 502);
    assertEquals(JSON.parse(text), { error: LIST_UNENDING });
    assertEquals(lists(seen.calls), OPEN_ROUND_BOUND);
    assertEquals(expires(seen.calls).length, OPEN_ROUND_BOUND * 100);
    assertEquals(seen.consents, []);
    assertEquals(seen.forms, []);
  }
});

// Review M3: an existing customer with nothing open costs exactly one list call, and nothing else.
Deno.test("nothing_open_costs_one_list_call", async () => {
  const { deps, seen } = fakes({
    listOpenCheckouts: (customer) => (seen.calls.push(`list:${customer}`), Promise.resolve(NOTHING_OPEN)),
  });
  const res = await run(buy("yearly"), deps);
  assertEquals(res.status, 200);
  assertEquals(seen.calls, withExpiry("yearly", ["list:cus_1"]));
});

// Review M3: one list naming the same session twice expires it once; a second expire of the same
// session would be refused, and a harmless duplicate would become a 502.
Deno.test("a_session_listed_twice_is_expired_once", async () => {
  const { deps, seen } = fakes({
    listOpenCheckouts: (customer) => (
      seen.calls.push(`list:${customer}`), Promise.resolve(openList(["cs_open_a", "cs_open_b", "cs_open_a"]))
    ),
  });
  const res = await run(buy("four_weekly"), deps);
  assertEquals(res.status, 200);
  assertEquals(seen.calls, withExpiry("four_weekly", ["list:cus_1", "expire:cs_open_a", "expire:cs_open_b"]));
});

// `stripePostFrom` logs the path and Stripe's body of a refused call, and an expire's path carries the
// session id; `_shared/stripe.ts` does not change in this lane (note D7), so the expire's `fetch` turns
// a refusal into a throw before that helper reads it. Driven with a fetch that serves itself.
Deno.test("an_expire_refusal_logs_no_session_id", async () => {
  const asked: string[] = [];
  const stripeFetch = (status: number): typeof fetch => (input, init) => {
    asked.push(`${init?.method} ${String(input)}`);
    const body = status === 200
      ? { id: "cs_open_a", object: "checkout.session", status: "expired" }
      : { error: { message: "This Checkout Session cs_open_a is not open; its customer is cus_1." } };
    return Promise.resolve(Response.json(body, { status }));
  };
  const path = "/v1/checkout/sessions/cs_open_a/expire";
  await capturingLogs(async (logged) => {
    const post = stripePostFrom("sk_test_invented", unloggedRefusals(stripeFetch(400), "an expire"));
    let threw = false;
    try {
      await post(path, {});
    } catch {
      threw = true;
    }
    assert(threw, "a refused expire throws");
    assert(logged.some((l) => l.includes("400")), `the status is logged: ${logged}`);
    namesNobody(logged.join("\n"), "the log");
  });
  const post = stripePostFrom("sk_test_invented", unloggedRefusals(stripeFetch(200), "an expire"));
  assertEquals(await post(path, {}), { id: "cs_open_a", object: "checkout.session", status: "expired" });
  assertEquals(asked, [`POST https://api.stripe.com${path}`, `POST https://api.stripe.com${path}`]);
});

Deno.test("index.ts lists and expires the open pages through the pinned helpers, ids encoded", async () => {
  const text = await Deno.readTextFile(new URL("./index.ts", import.meta.url));
  assert(
    /listOpenCheckouts: \(customer\) =>\s*stripeGetFrom\(stripeKey, globalThis\.fetch\)\(openCheckoutsPath\(customer\)\)/
      .test(text),
    "the list goes through stripeGetFrom, on the handler's path, with no cursor",
  );
  assert(!text.includes("starting_after"), "no cursor is wired");
  assert(
    /expireCheckout: \(id\) =>\s*stripePostFrom\(stripeKey, unloggedRefusals\(globalThis\.fetch, "an expire"\)\)\(/
      .test(text),
    "the expire goes through stripePostFrom, its refusals unlogged",
  );
  assert(
    /`\/v1\/checkout\/sessions\/\$\{encodeURIComponent\(id\)\}\/expire`,\s*\{\},?\s*\)/.test(text),
    "one session, encoded, an empty form",
  );
  // The list's path, on values: this customer's open sessions from the start, a full page, no cursor,
  // the id encoded so it cannot add a parameter of its own.
  assertEquals(OPEN_PAGE_SIZE, 100, "Stripe's largest page");
  assertEquals(openCheckoutsPath("cus_1"), "/v1/checkout/sessions?customer=cus_1&status=open&limit=100");
  assertEquals(
    openCheckoutsPath("cus_1&status=complete&starting_after=cs_open_b"),
    "/v1/checkout/sessions?customer=cus_1%26status%3Dcomplete%26starting_after%3Dcs_open_b" +
      "&status=open&limit=100",
  );
});
