import { restFromEnv, restSelectAll, restUpsert } from "../_shared/db.ts";
import { asResponse, fail } from "../_shared/http.ts";
import { CATALOG, resolveCatalog } from "../_shared/price_catalog.ts";
import { stripeGetFrom, stripePostFrom } from "../_shared/stripe.ts";
import { handle, Subscriber } from "./handler.ts";

function env(name: string): string {
  const v = Deno.env.get(name);
  if (!v) throw fail(500, "the function is not configured");
  return v;
}

// The price catalog (note D13), resolved once per cold start through `Deno.env.get`, never through
// `env` above: every variable is optional here (note D14), so one unset kept variable leaves that
// entry matching no id and is logged once, rather than answering 500 for the whole run.
const catalog = resolveCatalog(CATALOG, (name) => Deno.env.get(name));
if (catalog.missing.length > 0) {
  console.error(`billing-jobs: price variables not set: ${catalog.missing.join(", ")}`);
}

Deno.serve(async (req) => {
  try {
    const rest = restFromEnv();
    const stripeKey = env("STRIPE_SECRET_KEY");
    return await handle(req, {
      token: env("BILLING_JOBS_TOKEN"),
      now: () => new Date(),
      listSubscribers: async () => {
        // One view read, not a join written here: `billing_subscribers` is the migration's view.
        // Paged, not a single `select=*`: `config.toml`'s `max_rows = 1000` caps any one request,
        // and `order=account_id` is what makes the paging stable.
        return await restSelectAll<Subscriber>(rest, "billing_subscribers", "select=*&order=account_id");
      },
      // D10, D14: the subscriber's own subscription, through the pinned GET helper, path-encoded.
      getSubscription: (id) =>
        stripeGetFrom(stripeKey, globalThis.fetch)(`/v1/subscriptions/${encodeURIComponent(id)}`),
      stripe: stripePostFrom(stripeKey, globalThis.fetch),
      catalog,
      sendEmail: async (m) => {
        const res = await fetch(env("EMAIL_API_URL"), {
          method: "POST",
          headers: { authorization: `Bearer ${env("EMAIL_API_KEY")}`, "content-type": "application/json" },
          body: JSON.stringify({ from: env("EMAIL_FROM"), to: m.to, subject: m.subject, text: m.text }),
        });
        if (!res.ok) {
          console.error(`email: ${res.status}`);
          throw fail(502, "the reminder could not be sent");
        }
      },
      recordReminder: (accountId, at) =>
        restUpsert(
          rest,
          "billing_reminders",
          [{ account_id: accountId, last_reminded_at: at }],
          "account_id",
        ),
    });
  } catch (e) {
    return asResponse(e);
  }
});
