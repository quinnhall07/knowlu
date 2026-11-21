import { assert, assertEquals } from "@std/assert";
import { checkoutForm, type ConsentRecord, type Deps, handle, renewalTerms } from "./handler.ts";
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
      customText: "the renewal terms",
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
    // D8, P9: Stripe's page states the renewal terms.
    assertEquals(f["custom_text[submit][message]"], "the renewal terms");
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
  // exactly checkpoint A's list, the two Stripe documents refusing under Managed Payments.
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
  assertEquals(now.filter((k) => !before.includes(k)).sort(), [
    "custom_text[submit][message]",
    "managed_payments[enabled]",
  ]);
  // And no API version rides on the form: the pin in `_shared/stripe.ts` serves the session call.
  assert(!now.some((k) => /version/i.test(k)));
});

Deno.test("stripes_page_states_the_renewal_terms", async () => {
  assertEquals(
    renewalTerms("four_weekly", true, WORDS),
    "Billed every 4 weeks after 14 days free, plus tax where it applies, until you cancel. " +
      "Cancel any time in Knowlu: Settings, Membership, Manage subscription.",
  );
  assertEquals(
    renewalTerms("four_weekly", false, WORDS),
    "Billed every 4 weeks, plus tax where it applies, until you cancel. " +
      "Cancel any time in Knowlu: Settings, Membership, Manage subscription.",
  );
  assertEquals(
    renewalTerms("yearly", true, WORDS),
    "Billed once a year after 14 days free, plus tax where it applies, until you cancel. " +
      "30-day money-back guarantee on each yearly charge (knowlu.com/terms).",
  );
  assertEquals(
    renewalTerms("yearly", false, WORDS),
    "Billed once a year, plus tax where it applies, until you cancel. " +
      "30-day money-back guarantee on each yearly charge (knowlu.com/terms).",
  );
  // (r5: m2) The tax phrase and the money-back line are the catalog's words, not `handler.ts`'s.
  const words = { tax: "plus invented tax words", moneyBack: "an invented money-back promise" };
  assert(renewalTerms("four_weekly", true, words).includes("plus invented tax words, until you cancel."));
  assertEquals(
    renewalTerms("yearly", false, words),
    "Billed once a year, plus invented tax words, until you cancel. an invented money-back promise (knowlu.com/terms).",
  );
  // And the session carries it, with the trial as the account's state decides.
  const invented = catalogOf({ ...TABLE, words });
  for (
    const [plan, row] of [["yearly", null], ["four_weekly", {
      stripe_subscription_id: "sub_old",
      status: "canceled",
    }]] as const
  ) {
    const { deps, seen } = fakes({ catalog: invented, subscriptionState: () => Promise.resolve(row) });
    assertEquals((await run(buy(plan), deps)).status, 200);
    assertEquals(seen.forms[0]["custom_text[submit][message]"], renewalTerms(plan, row === null, words));
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
