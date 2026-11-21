/**
 * `GET /billing-checkout` → the offer: the catalog's selling plans, each with its cents and the lines
 * the app shows, and the terms version (v0.4.1 note D9). It reads no account.
 *
 * `POST /billing-checkout` → `{url}`, a hosted Stripe Checkout session the app opens in the SYSTEM
 * browser (spec §4.2 step 2). Hosted, not embedded, for one reason worth stating: a card form inside
 * our webview would make us the thing handling card data, and we have no reason to be.
 *
 * The consent row is written **before** the redirect, not after the webhook, because ROSCA and
 * California's ARL want a record of the consent that was shown — and a customer who abandons the
 * page still saw it. A duplicate row from a second attempt is harmless; a missing one is not.
 *
 * Every amount, price variable and terms version comes from the price catalog
 * (`_shared/price_catalog.ts`, note D13), handed in resolved through `Deps`.
 */
import { requireUser, VerifyToken } from "../_shared/auth.ts";
import { fail, json, methodNotAllowed, readJson } from "../_shared/http.ts";
import { StripePost } from "../_shared/stripe.ts";
import {
  cadenceWords,
  offer,
  type PlanKey,
  type ResolvedCatalog,
  type ResolvedEntry,
  sellingFor,
  type Words,
} from "../_shared/price_catalog.ts";

/** The plans sold (D1): keys that outlive their prices. */
export type Plan = PlanKey;
export const PLANS: readonly Plan[] = ["yearly", "four_weekly"];

/** The names a build older than v0.4.1 sends (D3): refused with a sentence a student can act on. */
export const RETIRED_PLANS: readonly string[] = ["monthly", "academic_year"];

/** D5 and ruling (7): 14 days free, on an account's first subscription only. */
export const TRIAL_DAYS = 14;

/** D5: a subscription in one of these states is still live, so a second one is refused. */
const LIVE_STATUSES: readonly string[] = ["active", "trialing", "past_due", "unpaid"];

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
  getAccount: (
    accountId: string,
  ) => Promise<{ email: string; stripe_customer_id: string | null; age_attested_at: string | null } | null>;
  /** Friends-testers spec §3.4, Q3: whether `effective_entitlements` shows `plan = 'tester'` for this
   * account. The view alone decides expiry and ending, so an expired or ended grant answers false. */
  isTester: (accountId: string) => Promise<boolean>;
  /** D5: the account's `entitlements` row, the table and not the view; `null` when there is none. */
  subscriptionState: (
    accountId: string,
  ) => Promise<{ stripe_subscription_id: string | null; status: string | null } | null>;
  /** D4: the Stripe Price, read through `stripeGetFrom`; `null` when Stripe would not hand it back. */
  getPrice: (priceId: string) => Promise<Record<string, unknown> | null>;
  saveCustomerId: (accountId: string, customerId: string) => Promise<void>;
  recordConsent: (c: ConsentRecord) => Promise<void>;
  stripe: StripePost;
  /** The price catalog, resolved from this project's environment. */
  catalog: ResolvedCatalog;
  successUrl: string;
  cancelUrl: string;
}

/**
 * D8, P9: the sentence Stripe's page shows by the button. The cadence words are the key's, which never
 * change; the tax phrase and the money-back line are the catalog's words, so they cannot drift from
 * the app's lines.
 */
export function renewalTerms(plan: Plan, trial: boolean, words: Words): string {
  const free = trial ? ` after ${TRIAL_DAYS} days free` : "";
  const head = `Billed ${cadenceWords(plan)}${free}, ${words.tax}, until you cancel.`;
  return plan === "yearly"
    ? `${head} ${words.moneyBack} (knowlu.com/terms).`
    : `${head} Cancel any time in Knowlu: Settings, Membership, Manage subscription.`;
}

export function checkoutForm(a: {
  price: string;
  customer: string;
  accountId: string;
  successUrl: string;
  cancelUrl: string;
  trial: boolean;
  customText: string;
}): Record<string, string> {
  const form: Record<string, string> = {
    "mode": "subscription",
    "line_items[0][price]": a.price,
    "line_items[0][quantity]": "1",
    "customer": a.customer,
    "client_reference_id": a.accountId,
    "subscription_data[metadata][account_id]": a.accountId,
    "payment_method_collection": "always",
    // §8, R-C1b-2: Stripe's own promotion-code field, beside a card that is still taken up front.
    // The code is typed on Stripe's page; nothing in this repo names a code, a coupon or a percentage.
    "allow_promotion_codes": "true",
    // Not refused under Managed Payments (T0r Q4), so it stays; Link collects the tax itself.
    "billing_address_collection": "required",
    // The auto-renewal terms get their own checkbox, not a line buried in the ToS.
    "consent_collection[terms_of_service]": "required",
    // D8: the renewal terms, on Stripe's page, before the card is entered.
    "custom_text[submit][message]": a.customText,
    // D7, checkpoint A: Managed Payments, with Link as seller of record. Stripe refuses
    // `automatic_tax[enabled]` and `customer_update[address]` beside it, so those two left the form.
    "managed_payments[enabled]": "true",
    "success_url": a.successUrl,
    "cancel_url": a.cancelUrl,
  };
  // D5: 14 days free, card taken first, on a first subscription only.
  if (a.trial) form["subscription_data[trial_period_days]"] = String(TRIAL_DAYS);
  return form;
}

/** IPv4, or IPv6 (bare, or a `[…]` literal a proxy may wrap it in) — loose on purpose, since all
 * this decides is whether Postgres's `inet` cast can take the value, not whether it is well-formed
 * by RFC. */
const IP_SHAPED = /^(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}|\[?[0-9a-fA-F:]+\]?)$/;

/** The caller's address for the consent log. The first hop is the client; the rest are proxies.
 * R-C1-59 (M4): a malformed header must not 502 a checkout — Postgres's `inet` column would reject
 * it, and a consent row with a null IP is still a consent row. */
function clientIp(req: Request): string | null {
  const fwd = req.headers.get("x-forwarded-for");
  if (!fwd) return null;
  const first = fwd.split(",")[0].trim();
  return IP_SHAPED.test(first) ? first : null;
}

const NOT_CONFIGURED = "the function is not configured";

/**
 * D4: the Stripe Price must be exactly what the catalog says this plan sells, tied to the plan (not
 * to a list of allowed pairs), so a price at the right amount but the wrong cadence never passes.
 * Returns the names of the fields that fail, `[]` when it passes.
 */
export function priceProblems(price: Record<string, unknown>, entry: ResolvedEntry): string[] {
  const recurring = (price.recurring ?? {}) as Record<string, unknown>;
  const out: string[] = [];
  if (price.active !== true) out.push("active");
  if (price.currency !== "usd") out.push("currency");
  if (price.unit_amount !== entry.cents) out.push("unit_amount");
  if (recurring.interval !== entry.interval) out.push("interval");
  if (recurring.interval_count !== entry.count) out.push("interval_count");
  if (price.tax_behavior !== "exclusive") out.push("tax_behavior");
  return out;
}

export async function handle(req: Request, deps: Deps): Promise<Response> {
  if (req.method !== "GET" && req.method !== "POST") return methodNotAllowed(["GET", "POST"]);
  const user = await requireUser(req, deps.verify);
  if (req.method === "GET") return json(200, offer(deps.catalog));

  const body = await readJson<{ plan?: unknown; amount_cents?: unknown; terms_version?: unknown }>(req);

  // D3, the three checks that read only the request, before any read or write.
  if (typeof body.plan === "string" && RETIRED_PLANS.includes(body.plan)) {
    throw fail(409, "This version of Knowlu can't subscribe any more. Update Knowlu, then try again.");
  }
  const plan = body.plan as Plan;
  if (!PLANS.includes(plan)) throw fail(400, `unknown plan; use ${PLANS.join(" or ")}`);
  if (body.amount_cents === undefined || body.amount_cents === null) {
    throw fail(400, "amount_cents is required");
  }
  if (body.terms_version === undefined || body.terms_version === null) {
    throw fail(400, "terms_version is required");
  }
  const entry = sellingFor(deps.catalog, plan);
  if (!entry) {
    console.error(`billing-checkout: the catalog sells no ${plan}`);
    throw fail(500, NOT_CONFIGURED);
  }
  if (body.amount_cents !== entry.cents || body.terms_version !== deps.catalog.termsVersion) {
    throw fail(409, "Knowlu's prices or terms have changed. Look them over again, then subscribe.");
  }

  const account = await deps.getAccount(user.id);
  if (!account) throw fail(404, "no such account");
  // Spec §5.1: migration 20260917000100 stopped the auth trigger raising on an OAuth sign-up, so
  // the 18+ gate lives here now — before the Stripe customer, before the consent row, before the
  // session. `POST /account/consent` is what fills it, and the app calls that on every sign-in.
  if (!account.age_attested_at) throw fail(403, "the 18+ attestation is missing — sign in again");
  // Friends-testers spec §3.4, Q3: while the view answers `plan = 'tester'` for this account, no path,
  // old build or stale page can start a charge. Asked before the Stripe customer, the consent row and
  // the session; an expired or ended grant is the view's to decide, and proceeds as today.
  if (await deps.isTester(user.id)) throw fail(409, "this account has free tester access");

  // D5: one live subscription per account, and one trial. A student whose card failed is sent to
  // Manage subscription, not to a second subscription while the first is still dunning.
  const state = await deps.subscriptionState(user.id);
  const hadSubscription = !!state?.stripe_subscription_id;
  if (hadSubscription && LIVE_STATUSES.includes(String(state?.status))) {
    throw fail(409, "This account already has a subscription. Manage it in Settings, then Membership.");
  }
  const trial = !hadSubscription;

  // D4: the price this plan sells, read from Stripe and checked, before anything is written.
  if (entry.priceId === null) {
    console.error(`billing-checkout: ${entry.variable} is not set`);
    throw fail(500, NOT_CONFIGURED);
  }
  const price = await deps.getPrice(entry.priceId);
  if (!price) {
    console.error(`billing-checkout: the ${plan} price could not be read`);
    throw fail(500, NOT_CONFIGURED);
  }
  const problems = priceProblems(price, entry);
  if (problems.length > 0) {
    // By field name only: never the price object, which is Stripe's and could change shape.
    console.error(`billing-checkout: the ${plan} price fails ${problems.join(", ")}`);
    throw fail(500, NOT_CONFIGURED);
  }

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

  // The catalog's cents and terms version, which equal what the app showed (D3).
  await deps.recordConsent({
    account_id: user.id,
    subject_email: account.email,
    kind: "auto_renew",
    version: deps.catalog.termsVersion,
    price_cents: entry.cents,
    ip: clientIp(req),
  });

  const session = await deps.stripe(
    "/v1/checkout/sessions",
    checkoutForm({
      price: entry.priceId,
      customer,
      accountId: user.id,
      successUrl: deps.successUrl,
      cancelUrl: deps.cancelUrl,
      trial,
      customText: renewalTerms(plan, trial, deps.catalog.words),
    }),
  );
  const url = session.url;
  if (typeof url !== "string") throw fail(502, "the payment provider returned no checkout link");
  return json(200, { url });
}
