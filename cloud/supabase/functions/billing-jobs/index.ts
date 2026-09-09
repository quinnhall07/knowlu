import { restFromEnv, restSelect, restUpsert } from "../_shared/db.ts";
import { asResponse, fail } from "../_shared/http.ts";
import { stripePostFrom } from "../_shared/stripe.ts";
import { handle, Subscriber } from "./handler.ts";

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
      token: env("BILLING_JOBS_TOKEN"),
      now: () => new Date(),
      listSubscribers: async () => {
        // One view read, not a join written here: `billing_subscribers` is the migration's view.
        return await restSelect<Subscriber>(rest, "billing_subscribers", "select=*");
      },
      stripe: stripePostFrom(stripeKey, globalThis.fetch),
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
