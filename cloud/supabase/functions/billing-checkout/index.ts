import { authGetUser, restFromEnv, restPatch, restSelect, restUpsert } from "../_shared/db.ts";
import { sha256Hex } from "../_shared/crypto.ts";
import { asResponse, fail } from "../_shared/http.ts";
import { stripePostFrom } from "../_shared/stripe.ts";
import { handle, Plan } from "./handler.ts";

function env(name: string): string {
  const v = Deno.env.get(name);
  if (!v) throw fail(500, "the function is not configured");
  return v;
}

Deno.serve(async (req) => {
  try {
    const rest = restFromEnv();
    const prices: Record<Plan, string> = {
      monthly: env("STRIPE_PRICE_MONTHLY"),
      academic_year: env("STRIPE_PRICE_YEAR"),
    };
    const cents: Record<Plan, number> = { monthly: 999, academic_year: 6999 };
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
      stripe: stripePostFrom(env("STRIPE_SECRET_KEY"), globalThis.fetch),
      priceFor: (p) => prices[p],
      priceCentsFor: (p) => cents[p],
      successUrl: "https://knowlu.com/subscribed.html",
      cancelUrl: "https://knowlu.com/index.html",
    });
  } catch (e) {
    return asResponse(e);
  }
});
