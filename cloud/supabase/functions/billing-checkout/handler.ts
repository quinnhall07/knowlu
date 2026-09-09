/**
 * `POST /billing-checkout` → `{url}`, a hosted Stripe Checkout session the app opens in the SYSTEM
 * browser (spec §4.2 step 2). Hosted, not embedded, for one reason worth stating: a card form inside
 * our webview would make us the thing handling card data, and we have no reason to be.
 *
 * The consent row is written **before** the redirect, not after the webhook, because ROSCA and
 * California's ARL want a record of the consent that was shown — and a customer who abandons the
 * page still saw it. A duplicate row from a second attempt is harmless; a missing one is not.
 */
import { requireUser, VerifyToken } from "../_shared/auth.ts";
import { fail, json, methodNotAllowed, readJson } from "../_shared/http.ts";
import { StripePost } from "../_shared/stripe.ts";

export type Plan = "monthly" | "academic_year";

const PLANS: readonly Plan[] = ["monthly", "academic_year"];

export interface ConsentRecord {
  account_id: string;
  subject_email: string;
  kind: "auto_renew";
  version: string;
  price_cents: number;
  ip: string | null;
}

export interface Deps {
  verify: VerifyToken;
  getAccount: (accountId: string) => Promise<{ email: string; stripe_customer_id: string | null } | null>;
  saveCustomerId: (accountId: string, customerId: string) => Promise<void>;
  recordConsent: (c: ConsentRecord) => Promise<void>;
  stripe: StripePost;
  priceFor: (plan: Plan) => string;
  priceCentsFor: (plan: Plan) => number;
  successUrl: string;
  cancelUrl: string;
}

export function checkoutForm(a: {
  price: string;
  customer: string;
  accountId: string;
  successUrl: string;
  cancelUrl: string;
}): Record<string, string> {
  return {
    "mode": "subscription",
    "line_items[0][price]": a.price,
    "line_items[0][quantity]": "1",
    "customer": a.customer,
    "client_reference_id": a.accountId,
    "subscription_data[metadata][account_id]": a.accountId,
    // R2: a 7-day trial with the card taken up front.
    "subscription_data[trial_period_days]": "7",
    "payment_method_collection": "always",
    // Stripe Tax needs an address to decide Kentucky's 6% (legal note §8).
    "automatic_tax[enabled]": "true",
    "customer_update[address]": "auto",
    "billing_address_collection": "required",
    // The auto-renewal terms get their own checkbox, not a line buried in the ToS.
    "consent_collection[terms_of_service]": "required",
    "success_url": a.successUrl,
    "cancel_url": a.cancelUrl,
  };
}

/** The caller's address for the consent log. The first hop is the client; the rest are proxies. */
function clientIp(req: Request): string | null {
  const fwd = req.headers.get("x-forwarded-for");
  return fwd ? fwd.split(",")[0].trim() : null;
}

export async function handle(req: Request, deps: Deps): Promise<Response> {
  if (req.method !== "POST") return methodNotAllowed(["POST"]);
  const user = await requireUser(req, deps.verify);
  const body = await readJson<{ plan?: string; terms_version?: string }>(req);
  const plan = body.plan as Plan;
  if (!PLANS.includes(plan)) throw fail(400, `unknown plan; use ${PLANS.join(" or ")}`);
  if (!body.terms_version) throw fail(400, "terms_version is required");

  const account = await deps.getAccount(user.id);
  if (!account) throw fail(404, "no such account");

  let customer = account.stripe_customer_id;
  if (!customer) {
    const created = await deps.stripe("/v1/customers", {
      email: account.email,
      "metadata[account_id]": user.id,
    });
    if (typeof created.id !== "string") throw fail(502, "the payment provider returned no customer id");
    customer = created.id;
    await deps.saveCustomerId(user.id, customer);
  }

  await deps.recordConsent({
    account_id: user.id,
    subject_email: account.email,
    kind: "auto_renew",
    version: body.terms_version,
    price_cents: deps.priceCentsFor(plan),
    ip: clientIp(req),
  });

  const session = await deps.stripe(
    "/v1/checkout/sessions",
    checkoutForm({
      price: deps.priceFor(plan),
      customer,
      accountId: user.id,
      successUrl: deps.successUrl,
      cancelUrl: deps.cancelUrl,
    }),
  );
  const url = session.url;
  if (typeof url !== "string") throw fail(502, "the payment provider returned no checkout link");
  return json(200, { url });
}
