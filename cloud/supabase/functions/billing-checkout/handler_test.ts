import { assert, assertEquals } from "@std/assert";
import { checkoutForm, type Deps, handle } from "./handler.ts";
import { CASES } from "../../migrations/testers_cases.ts";

Deno.test("the Checkout form carries the trial, the card, the tax and the terms checkbox", () => {
  const form = checkoutForm({
    price: "price_monthly",
    customer: "cus_1",
    accountId: "acc-1",
    successUrl: "https://knowlu.com/subscribed.html",
    cancelUrl: "https://knowlu.com/index.html",
  });
  assertEquals(form["mode"], "subscription");
  assertEquals(form["line_items[0][price]"], "price_monthly");
  assertEquals(form["line_items[0][quantity]"], "1");
  assertEquals(form["customer"], "cus_1");
  assertEquals(form["client_reference_id"], "acc-1");
  assertEquals(form["subscription_data[metadata][account_id]"], "acc-1");
  // R2: seven days, and the card up front — a wall before the first session costs more than a week
  // of inference, and a trial with no card is a wall in a different place.
  assertEquals(form["subscription_data[trial_period_days]"], "7");
  // R-C1b-2. `always` STAYS: `if_required` collects a card only when the first invoice has an
  // amount due, and `subscription_data[trial_period_days]` makes that invoice zero for every
  // subscription — so it would take a card from nobody and falsify `site/terms.html`'s bolded
  // "your card taken at sign-up", the version each `auto_renew` consent row is stamped with.
  assertEquals(form["payment_method_collection"], "always");
  // Spec §8: the promotion-code field on Stripe's own page. The code is Quinn's to create in the
  // dashboard (P3); nothing in this repo names a code, a coupon id or a percentage.
  assertEquals(form["allow_promotion_codes"], "true");
  // §9, sales tax: Stripe Tax decides Kentucky's 6%, and an address is what lets it.
  assertEquals(form["automatic_tax[enabled]"], "true");
  assertEquals(form["customer_update[address]"], "auto");
  assertEquals(form["billing_address_collection"], "required");
  // §8 of the legal note: express informed consent to the auto-renewal terms, its own checkbox.
  assertEquals(form["consent_collection[terms_of_service]"], "required");
  assertEquals(form["success_url"], "https://knowlu.com/subscribed.html");
  assertEquals(form["cancel_url"], "https://knowlu.com/index.html");
});

Deno.test("an account with no Stripe customer gets one, once, and it is saved", async () => {
  const calls: string[] = [];
  let saved: [string, string] | null = null;
  const res = await handle(
    new Request("http://127.0.0.1:1/", {
      method: "POST",
      headers: { authorization: "Bearer good" },
      body: JSON.stringify({ plan: "monthly", terms_version: "2026-09-10" }),
    }),
    {
      verify: () => Promise.resolve({ id: "acc-1", email: "a@example.invalid" }),
      getAccount: () =>
        Promise.resolve({
          email: "a@example.invalid",
          stripe_customer_id: null,
          age_attested_at: "2026-09-17T12:00:00Z",
        }),
      isTester: () => Promise.resolve(false),
      saveCustomerId: (a, c) => {
        saved = [a, c];
        return Promise.resolve();
      },
      recordConsent: () => Promise.resolve(),
      stripe: (path, form) => {
        calls.push(path);
        return Promise.resolve(
          path === "/v1/customers"
            ? { id: "cus_new" }
            : { id: "cs_1", url: "https://checkout.stripe.com/c/cs_1", ...form },
        );
      },
      priceFor: () => "price_monthly",
      priceCentsFor: () => 999,
      successUrl: "https://knowlu.com/subscribed.html",
      cancelUrl: "https://knowlu.com/index.html",
    },
  );
  assertEquals(res.status, 200);
  assertEquals(await res.json(), { url: "https://checkout.stripe.com/c/cs_1" });
  assertEquals(calls, ["/v1/customers", "/v1/checkout/sessions"]);
  assertEquals(saved, ["acc-1", "cus_new"]);
});

Deno.test("the auto-renew consent is logged with its version and the price, before the redirect", async () => {
  const consents: unknown[] = [];
  await handle(
    new Request("http://127.0.0.1:1/", {
      method: "POST",
      headers: { authorization: "Bearer good", "x-forwarded-for": "203.0.113.7, 10.0.0.1" },
      body: JSON.stringify({ plan: "academic_year", terms_version: "2026-09-10" }),
    }),
    {
      verify: () => Promise.resolve({ id: "acc-1", email: "a@example.invalid" }),
      getAccount: () =>
        Promise.resolve({
          email: "a@example.invalid",
          stripe_customer_id: "cus_1",
          age_attested_at: "2026-09-17T12:00:00Z",
        }),
      isTester: () => Promise.resolve(false),
      saveCustomerId: () => Promise.resolve(),
      recordConsent: (c) => {
        consents.push(c);
        return Promise.resolve();
      },
      stripe: () => Promise.resolve({ id: "cs_2", url: "https://checkout.stripe.com/c/cs_2" }),
      priceFor: (p) => (p === "academic_year" ? "price_year" : "price_monthly"),
      priceCentsFor: (p) => (p === "academic_year" ? 6999 : 999),
      successUrl: "https://knowlu.com/subscribed.html",
      cancelUrl: "https://knowlu.com/index.html",
    },
  );
  assertEquals(consents.length, 1);
  assertEquals(consents[0], {
    account_id: "acc-1",
    subject_email: "a@example.invalid",
    kind: "auto_renew",
    version: "2026-09-10",
    price_cents: 6999,
    ip: "203.0.113.7",
  });
});

Deno.test("R-C1-59 (M4): a malformed x-forwarded-for never 502s a checkout — the consent row just carries no ip", async () => {
  const consents: unknown[] = [];
  const res = await handle(
    new Request("http://127.0.0.1:1/", {
      method: "POST",
      // Not an IP at all — the shape a hostile or misconfigured proxy could hand the function.
      headers: { authorization: "Bearer good", "x-forwarded-for": "definitely not an ip" },
      body: JSON.stringify({ plan: "monthly", terms_version: "2026-09-10" }),
    }),
    {
      verify: () => Promise.resolve({ id: "acc-1", email: "a@example.invalid" }),
      getAccount: () =>
        Promise.resolve({
          email: "a@example.invalid",
          stripe_customer_id: "cus_1",
          age_attested_at: "2026-09-17T12:00:00Z",
        }),
      isTester: () => Promise.resolve(false),
      saveCustomerId: () => Promise.resolve(),
      recordConsent: (c) => {
        consents.push(c);
        return Promise.resolve();
      },
      stripe: () => Promise.resolve({ id: "cs_3", url: "https://checkout.stripe.com/c/cs_3" }),
      priceFor: () => "price_monthly",
      priceCentsFor: () => 999,
      successUrl: "https://knowlu.com/subscribed.html",
      cancelUrl: "https://knowlu.com/index.html",
    },
  );
  assertEquals(res.status, 200);
  assertEquals(consents.length, 1);
  assertEquals((consents[0] as { ip: string | null }).ip, null);
});

Deno.test("an unknown plan is 400 and never reaches Stripe", async () => {
  let touched = false;
  const res = await handle(
    new Request("http://127.0.0.1:1/", {
      method: "POST",
      headers: { authorization: "Bearer good" },
      body: JSON.stringify({ plan: "lifetime", terms_version: "2026-09-10" }),
    }),
    {
      verify: () => Promise.resolve({ id: "acc-1", email: "a@example.invalid" }),
      getAccount: () =>
        Promise.resolve({
          email: "a@example.invalid",
          stripe_customer_id: "cus_1",
          age_attested_at: "2026-09-17T12:00:00Z",
        }),
      isTester: () => Promise.resolve(false),
      saveCustomerId: () => Promise.resolve(),
      recordConsent: () => Promise.resolve(),
      stripe: () => {
        touched = true;
        return Promise.resolve({});
      },
      priceFor: () => "price_monthly",
      priceCentsFor: () => 999,
      successUrl: "https://knowlu.com/subscribed.html",
      cancelUrl: "https://knowlu.com/index.html",
    },
  ).catch((e) => e as Response);
  assertEquals(res.status, 400);
  assert(!touched);
});

Deno.test("an account that never attested to being 18 cannot reach Stripe", async () => {
  let touched = false;
  const res = await handle(
    new Request("http://127.0.0.1:1/", {
      method: "POST",
      headers: { authorization: "Bearer good" },
      body: JSON.stringify({ plan: "monthly", terms_version: "2026-09-10" }),
    }),
    {
      verify: () => Promise.resolve({ id: "acc-1", email: "a@example.invalid" }),
      // Migration 20260917000100 lets an OAuth sign-up land here with a null attestation; this is
      // the server-side tooth that replaced the trigger's `raise`.
      getAccount: () =>
        Promise.resolve({ email: "a@example.invalid", stripe_customer_id: null, age_attested_at: null }),
      isTester: () => Promise.resolve(false),
      saveCustomerId: () => Promise.resolve(),
      recordConsent: () => {
        touched = true;
        return Promise.resolve();
      },
      stripe: () => {
        touched = true;
        return Promise.resolve({});
      },
      priceFor: () => "price_monthly",
      priceCentsFor: () => 999,
      successUrl: "https://knowlu.com/subscribed.html",
      cancelUrl: "https://knowlu.com/index.html",
    },
  ).catch((e) => e as Response);
  assertEquals(res.status, 403);
  assert(!touched, "no customer, no consent row, no session");
});

Deno.test("an account that did attest goes through exactly as before", async () => {
  const res = await handle(
    new Request("http://127.0.0.1:1/", {
      method: "POST",
      headers: { authorization: "Bearer good" },
      body: JSON.stringify({ plan: "monthly", terms_version: "2026-09-10" }),
    }),
    {
      verify: () => Promise.resolve({ id: "acc-1", email: "a@example.invalid" }),
      getAccount: () =>
        Promise.resolve({
          email: "a@example.invalid",
          stripe_customer_id: "cus_1",
          age_attested_at: "2026-09-17T12:00:00Z",
        }),
      isTester: () => Promise.resolve(false),
      saveCustomerId: () => Promise.resolve(),
      recordConsent: () => Promise.resolve(),
      stripe: () => Promise.resolve({ id: "cs_4", url: "https://checkout.stripe.com/c/cs_4" }),
      priceFor: () => "price_monthly",
      priceCentsFor: () => 999,
      successUrl: "https://knowlu.com/subscribed.html",
      cancelUrl: "https://knowlu.com/index.html",
    },
  );
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
    priceFor: () => "price_monthly",
    priceCentsFor: () => 999,
    successUrl: "https://knowlu.com/subscribed.html",
    cancelUrl: "https://knowlu.com/index.html",
  };
}

const checkoutReq = () =>
  new Request("http://127.0.0.1:1/", {
    method: "POST",
    headers: { authorization: "Bearer good" },
    body: JSON.stringify({ plan: "monthly", terms_version: "2026-09-10" }),
  });

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
