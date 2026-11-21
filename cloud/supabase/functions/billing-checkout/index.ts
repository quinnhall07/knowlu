import { authGetUser, restFromEnv, restPatch, restSelect, restUpsert } from "../_shared/db.ts";
import { sha256Hex } from "../_shared/crypto.ts";
import { asResponse, fail } from "../_shared/http.ts";
import { CATALOG, resolveCatalog } from "../_shared/price_catalog.ts";
import { stripeGetFrom, stripePostFrom } from "../_shared/stripe.ts";
import { handle } from "./handler.ts";

function env(name: string): string {
  const v = Deno.env.get(name);
  if (!v) throw fail(500, "the function is not configured");
  return v;
}

// The price catalog (note D13), resolved once per cold start from this project's own environment: the
// price ids differ per project and per Stripe mode, and their variable names are the catalog's to
// derive. An unset selling variable is not fatal here; the handler refuses that plan with a 500
// before anything is written (D4).
const catalog = resolveCatalog(CATALOG, (name) => Deno.env.get(name));
if (catalog.missing.length > 0) {
  console.error(`billing-checkout: price variables not set: ${catalog.missing.join(", ")}`);
}

Deno.serve(async (req) => {
  try {
    const rest = restFromEnv();
    const stripeKey = env("STRIPE_SECRET_KEY");
    return await handle(req, {
      verify: (token) => authGetUser(rest, token),
      getAccount: async (id) => {
        const rows = await restSelect<
          { email: string; stripe_customer_id: string | null; age_attested_at: string | null }
        >(
          rest,
          "accounts",
          `id=eq.${encodeURIComponent(id)}&select=email,stripe_customer_id,age_attested_at&limit=1`,
        );
        return rows[0] ?? null;
      },
      // Friends-testers spec §3.4: the view, not the grant table, so expiry and ending are decided in
      // one place. Pinned by `handler_test.ts`, which reads this file as text.
      isTester: async (id) => {
        const rows = await restSelect<{ plan: string | null }>(
          rest,
          "effective_entitlements",
          `account_id=eq.${encodeURIComponent(id)}&select=plan&limit=1`,
        );
        return rows[0]?.plan === "tester";
      },
      // D5: the table the webhook writes, not the view, because only a Stripe subscription counts.
      subscriptionState: async (id) => {
        const rows = await restSelect<{ stripe_subscription_id: string | null; status: string | null }>(
          rest,
          "entitlements",
          `account_id=eq.${encodeURIComponent(id)}&select=stripe_subscription_id,status&limit=1`,
        );
        return rows[0] ?? null;
      },
      // D4: through the pinned GET helper. The id is the catalog's, from this project's environment.
      getPrice: (id) => stripeGetFrom(stripeKey, globalThis.fetch)(`/v1/prices/${encodeURIComponent(id)}`),
      saveCustomerId: (id, cid) =>
        restPatch(rest, "accounts", `id=eq.${encodeURIComponent(id)}`, { stripe_customer_id: cid }),
      recordConsent: async (c) => {
        const hash = await sha256Hex(c.subject_email);
        await restUpsert(rest, "consents", [{
          account_id: c.account_id,
          subject_hash: hash,
          kind: c.kind,
          version: c.version,
          price_cents: c.price_cents,
          ip: c.ip,
        }]);
      },
      stripe: stripePostFrom(stripeKey, globalThis.fetch),
      catalog,
      successUrl: "https://knowlu.com/subscribed.html",
      cancelUrl: "https://knowlu.com/index.html",
    });
  } catch (e) {
    return asResponse(e);
  }
});
