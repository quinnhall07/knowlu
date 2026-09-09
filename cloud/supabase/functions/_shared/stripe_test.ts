import { assert, assertEquals } from "@std/assert";
import {
  formEncode,
  hmacHex,
  parseStripeSignature,
  STRIPE_API_VERSION,
  stripeGetFrom,
  stripePostFrom,
  verifyStripeSignature,
} from "./stripe.ts";

Deno.test("formEncode drops nullish values and escapes the bracket keys Stripe uses", () => {
  assertEquals(
    formEncode({
      "line_items[0][price]": "price_1",
      "line_items[0][quantity]": 1,
      mode: "subscription",
      nope: undefined,
      alsono: null,
    }),
    "line_items%5B0%5D%5Bprice%5D=price_1&line_items%5B0%5D%5Bquantity%5D=1&mode=subscription",
  );
  assertEquals(formEncode({ "automatic_tax[enabled]": true }), "automatic_tax%5Benabled%5D=true");
});

Deno.test("parseStripeSignature reads the timestamp and every v1 signature", () => {
  assertEquals(parseStripeSignature("t=1700000000,v1=aaa,v1=bbb,v0=zzz"), {
    t: 1700000000,
    v1: ["aaa", "bbb"],
  });
  assertEquals(parseStripeSignature("v1=aaa"), null);
  assertEquals(parseStripeSignature("t=notanumber,v1=aaa"), null);
  assertEquals(parseStripeSignature(""), null);
});

Deno.test("a signature Stripe would have made verifies, and one byte off does not", async () => {
  // The secret here is a literal invented for this test and is not a credential of any account.
  const secret = "whsec_test_only_not_a_real_key";
  const payload = '{"id":"evt_1","type":"customer.subscription.updated"}';
  const t = 1_700_000_000;
  const sig = await hmacHex(secret, `${t}.${payload}`);
  assert(await verifyStripeSignature(payload, `t=${t},v1=${sig}`, secret, t + 10));
  assert(!await verifyStripeSignature(payload + " ", `t=${t},v1=${sig}`, secret, t + 10), "payload changed");
  assert(
    !await verifyStripeSignature(payload, `t=${t},v1=${sig.slice(0, -1)}0`, secret, t + 10),
    "signature changed",
  );
  assert(!await verifyStripeSignature(payload, `t=${t},v1=${sig}`, secret, t + 400), "outside the tolerance");
  assert(!await verifyStripeSignature(payload, `t=${t},v1=${sig}`, "another_secret", t + 10), "wrong secret");
});

Deno.test("every Stripe call carries the API version this code parses", async () => {
  // P2 sets the WEBHOOK ENDPOINT's version, which governs the event payloads Stripe pushes. This is
  // the other half: the calls we make ourselves. The GET matters most — its body is what
  // `entitlementFromSubscription` reads, and unversioned it comes back under the account's own
  // default, where `current_period_end` is on neither shape the handler looks at.
  const seen: Array<[string, Headers]> = [];
  const fake: typeof fetch = (input, init) => {
    seen.push([String(input), new Headers(init?.headers)]);
    return Promise.resolve(
      new Response(JSON.stringify({ id: "sub_1" }), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );
  };
  // Not a credential of any account: `sk_test_` plus words, never sent anywhere by this test.
  const key = "sk_test_not_a_real_key";
  await stripePostFrom(key, fake)("/v1/customers", { email: "a@example.invalid" });
  await stripeGetFrom(key, fake)("/v1/subscriptions/sub_1");
  assertEquals(seen.length, 2);
  for (const [url, headers] of seen) {
    assertEquals(headers.get("stripe-version"), STRIPE_API_VERSION, url);
  }
  assertEquals(seen[1][0], "https://api.stripe.com/v1/subscriptions/sub_1");
});
