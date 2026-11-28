import { restFromEnv, restSelect, restUpsert } from "../_shared/db.ts";
import { asResponse, fail } from "../_shared/http.ts";
import { CATALOG, resolveCatalog } from "../_shared/price_catalog.ts";
import { handle } from "./handler.ts";
import { subscriptionReader } from "./subscription_read.ts";

function env(name: string): string {
  const v = Deno.env.get(name);
  if (!v) throw fail(500, "the function is not configured");
  return v;
}

// The price catalog (note D13), resolved once per cold start through `Deno.env.get`, never through
// `env` above: every variable is optional here (note D2), so one unset value leaves its entry matching
// no id and is logged once, rather than answering 500 to every event Stripe sends.
const catalog = resolveCatalog(CATALOG, (name) => Deno.env.get(name));
if (catalog.missing.length > 0) {
  console.error(`stripe-webhook: price variables not set: ${catalog.missing.join(", ")}`);
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
      accountExists: async (accountId) => {
        const rows = await restSelect<{ id: string }>(
          rest,
          "accounts",
          `id=eq.${encodeURIComponent(accountId)}&select=id&limit=1`,
        );
        return rows.length > 0;
      },
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
      // **Through `stripeGetFrom`, not a bare `fetch`** (inside `subscriptionReader`): it pins
      // `Stripe-Version` to the version `entitlementFromSubscription` is written against, which is the
      // same version P2 sets on the webhook endpoint. The id is path-encoded — it is Stripe's, but it
      // arrives over the wire. Strict: `null` only for Stripe's 404, a throw for any other failure.
      fetchSubscription: subscriptionReader(stripeKey, globalThis.fetch),
      catalog,
      // D6, D6b: the account's own address, by the account id the event carries; nothing else.
      accountEmail: async (accountId) => {
        const rows = await restSelect<{ email: string }>(
          rest,
          "accounts",
          `id=eq.${encodeURIComponent(accountId)}&select=email&limit=1`,
        );
        return rows[0]?.email ?? null;
      },
      // The shape `billing-jobs/index.ts` sends with. A failure throws, and the handler answers 502.
      sendEmail: async (m) => {
        const res = await fetch(env("EMAIL_API_URL"), {
          method: "POST",
          headers: { authorization: `Bearer ${env("EMAIL_API_KEY")}`, "content-type": "application/json" },
          body: JSON.stringify({ from: env("EMAIL_FROM"), to: m.to, subject: m.subject, text: m.text }),
        });
        if (!res.ok) {
          console.error(`email: ${res.status}`);
          throw fail(502, "the email could not be sent");
        }
      },
    });
  } catch (e) {
    return asResponse(e);
  }
});
