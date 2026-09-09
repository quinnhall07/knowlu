import { restFromEnv, restSelect, restUpsert } from "../_shared/db.ts";
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
      seenEvent: async (eventId) => {
        const rows = await restSelect<{ event_id: string }>(
          rest,
          "webhook_events",
          `event_id=eq.${encodeURIComponent(eventId)}&select=event_id`,
        );
        return rows.length > 0;
      },
      recordEvent: (eventId, type, createdIso) =>
        restUpsert(rest, "webhook_events", [{
          event_id: eventId,
          event_type: type,
          created_at: createdIso,
        }], "event_id"),
      currentUpdatedAt: async (accountId) => {
        const rows = await restSelect<{ updated_at: string }>(
          rest,
          "entitlements",
          `account_id=eq.${encodeURIComponent(accountId)}&select=updated_at`,
        );
        // Normalised to the same `toISOString()` shape `createdIso` is: PostgREST hands back
        // `timestamptz` as `…+00:00` (or another offset, if the database's `TimeZone` is ever not
        // UTC), and `handler.ts`'s stale guard compares these as instants, never as raw strings.
        return rows.length > 0 ? new Date(rows[0].updated_at).toISOString() : null;
      },
      writeEntitlement: (accountId, row, updatedAt) =>
        restUpsert(rest, "entitlements", [{
          account_id: accountId,
          plan: row.plan,
          status: row.status,
          current_period_end: row.current_period_end,
          stripe_subscription_id: row.stripe_subscription_id,
          paused: row.paused,
          started_at: row.started_at,
          source: "stripe",
          updated_at: updatedAt,
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
