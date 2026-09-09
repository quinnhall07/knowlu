import { authGetUser, restFromEnv, restSelect } from "../_shared/db.ts";
import { asResponse, fail } from "../_shared/http.ts";
import { stripePostFrom } from "../_shared/stripe.ts";
import { handle } from "./handler.ts";

Deno.serve(async (req) => {
  try {
    const rest = restFromEnv();
    const key = Deno.env.get("STRIPE_SECRET_KEY");
    if (!key) throw fail(500, "the function is not configured");
    return await handle(req, {
      verify: (token) => authGetUser(rest, token),
      getCustomerId: async (id) => {
        const rows = await restSelect<{ stripe_customer_id: string | null }>(
          rest,
          "accounts",
          `id=eq.${encodeURIComponent(id)}&select=stripe_customer_id&limit=1`,
        );
        return rows[0]?.stripe_customer_id ?? null;
      },
      stripe: stripePostFrom(key, globalThis.fetch),
      returnUrl: "https://knowlu.com/index.html",
    });
  } catch (e) {
    return asResponse(e);
  }
});
