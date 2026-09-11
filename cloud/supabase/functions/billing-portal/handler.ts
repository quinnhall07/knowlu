/**
 * `POST /billing-portal` → `{url}`. Stripe's own Customer Portal, with cancellation on and no
 * survey: California's ARL wants cancelling to take no more steps than signing up did, and the app's
 * *Cancel subscription* button is one click to here and one click there.
 */
import { requireUser, VerifyToken } from "../_shared/auth.ts";
import { fail, json, methodNotAllowed } from "../_shared/http.ts";
import { StripePost } from "../_shared/stripe.ts";

export interface Deps {
  verify: VerifyToken;
  getCustomerId: (accountId: string) => Promise<string | null>;
  stripe: StripePost;
  returnUrl: string;
}

export async function handle(req: Request, deps: Deps): Promise<Response> {
  if (req.method !== "POST") return methodNotAllowed(["POST"]);
  const user = await requireUser(req, deps.verify);
  const customer = await deps.getCustomerId(user.id);
  if (!customer) throw fail(409, "this account has never subscribed");
  const session = await deps.stripe("/v1/billing_portal/sessions", {
    customer,
    return_url: deps.returnUrl,
  });
  const url = session.url;
  if (typeof url !== "string") throw fail(502, "the payment provider returned no portal link");
  return json(200, { url });
}
