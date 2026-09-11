import { assert, assertEquals } from "@std/assert";
import { checkoutForm, handle } from "./handler.ts";

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
  assertEquals(form["payment_method_collection"], "always");
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
      getAccount: () => Promise.resolve({ email: "a@example.invalid", stripe_customer_id: null }),
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
      getAccount: () => Promise.resolve({ email: "a@example.invalid", stripe_customer_id: "cus_1" }),
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
      getAccount: () => Promise.resolve({ email: "a@example.invalid", stripe_customer_id: "cus_1" }),
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
      getAccount: () => Promise.resolve({ email: "a@example.invalid", stripe_customer_id: "cus_1" }),
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
