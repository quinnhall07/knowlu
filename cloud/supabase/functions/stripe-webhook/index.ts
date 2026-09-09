import { restFromEnv, restUpsert } from "../_shared/db.ts";
import { asResponse, fail } from "../_shared/http.ts";
import { stripeGetFrom } from "../_shared/stripe.ts";
import { handle } from "./handler.ts";

function env(name: string): string {
  const v = Deno.env.get(name);
  if (!v) throw fail(500, "the function is not configured");
  return v;
}

Deno.serve(async (req) => {
  try {
    const rest = restFromEnv();
    const stripeKey = env("STRIPE_SECRET_KEY");
    return await handle(req, {
      secret: env("STRIPE_WEBHOOK_SECRET"),
      nowSeconds: () => Math.floor(Date.now() / 1000),
      writeEntitlement: (accountId, row) =>
        restUpsert(rest, "entitlements", [{
          account_id: accountId,
          plan: row.plan,
          status: row.status,
          current_period_end: row.current_period_end,
          source: "stripe",
          updated_at: new Date().toISOString(),
        }], "account_id"),
      // **Through `stripeGetFrom`, not a bare `fetch`**: it pins `Stripe-Version` to the version
      // `entitlementFromSubscription` is written against, which is the same version P2 sets on the
      // webhook endpoint. The id is path-encoded — it is Stripe's, but it arrives over the wire.
      fetchSubscription: (id) =>
        stripeGetFrom(stripeKey, globalThis.fetch)(`/v1/subscriptions/${encodeURIComponent(id)}`),
    });
  } catch (e) {
    return asResponse(e);
  }
});
