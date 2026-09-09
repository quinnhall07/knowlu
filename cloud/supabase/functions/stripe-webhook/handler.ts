/**
 * **The only writer of `entitlements`** (spec §5.1). Nothing else in this codebase may write that
 * table, and no client can: RLS grants `select` and nothing more, and only the service role — which
 * this function holds and no browser does — bypasses it.
 *
 * The raw body is read as text and verified *before* it is parsed. Parsing first would mean acting
 * on a shape an attacker chose.
 */
import { EntitlementStatus } from "../_shared/entitlement.ts";
import { fail, json, methodNotAllowed } from "../_shared/http.ts";
import { verifyStripeSignature } from "../_shared/stripe.ts";

// deno-lint-ignore no-explicit-any
type Json = any;

export interface EntitlementWrite {
  plan: string | null;
  status: EntitlementStatus;
  current_period_end: string | null;
}

export interface Deps {
  secret: string;
  nowSeconds: () => number;
  writeEntitlement: (accountId: string, row: EntitlementWrite) => Promise<void>;
  /** `checkout.session.completed` carries a subscription id, not the subscription. */
  fetchSubscription: (id: string) => Promise<Json | null>;
}

/** Stripe's status vocabulary is longer than ours; this is the whole of the mapping. */
function statusOf(stripeStatus: string): EntitlementStatus {
  switch (stripeStatus) {
    case "active":
      return "active";
    case "trialing":
      return "trialing";
    case "past_due":
    case "unpaid":
    case "incomplete":
      return "past_due";
    case "canceled":
    case "incomplete_expired":
      return "canceled";
    default:
      return "none";
  }
}

export function entitlementFromSubscription(sub: Json): EntitlementWrite {
  const interval = sub?.items?.data?.[0]?.price?.recurring?.interval ?? null;
  const plan = interval === "month" ? "monthly" : interval === "year" ? "academic_year" : null;
  // Under the pinned STRIPE_API_VERSION (2025-03-31.basil), `current_period_end` left the
  // Subscription for `items.data[].current_period_end`. Read the top-level field first — an
  // account whose events still carry it should keep working — and fall back to the item.
  const end = typeof sub?.current_period_end === "number"
    ? new Date(sub.current_period_end * 1000).toISOString()
    : typeof sub?.items?.data?.[0]?.current_period_end === "number"
    ? new Date(sub.items.data[0].current_period_end * 1000).toISOString()
    : null;
  // A `pause_collection` window (R3, June to August) leaves Stripe's own status `active`: the
  // student keeps the product and is not charged. Nothing here needs to know about the pause.
  return { plan, status: statusOf(String(sub?.status ?? "")), current_period_end: end };
}

export function accountIdFromEvent(event: Json): string | null {
  const o = event?.data?.object ?? {};
  return o?.metadata?.account_id ?? o?.client_reference_id ?? null;
}

const SUBSCRIPTION_EVENTS = new Set([
  "customer.subscription.created",
  "customer.subscription.updated",
  "customer.subscription.deleted",
]);

export async function handle(req: Request, deps: Deps): Promise<Response> {
  if (req.method !== "POST") return methodNotAllowed(["POST"]);
  const raw = await req.text();
  const header = req.headers.get("stripe-signature") ?? "";
  if (!await verifyStripeSignature(raw, header, deps.secret, deps.nowSeconds())) {
    return fail(400, "signature does not verify");
  }
  let event: Json;
  try {
    event = JSON.parse(raw);
  } catch {
    return fail(400, "body is not JSON");
  }

  const type = String(event?.type ?? "");
  // Every other event is acknowledged and dropped. A 4xx here makes Stripe retry for days, which
  // turns one unhandled type into a permanent alarm about nothing.
  if (
    !SUBSCRIPTION_EVENTS.has(type) && type !== "checkout.session.completed" &&
    type !== "invoice.payment_failed"
  ) {
    return json(200, { ignored: type });
  }

  const accountId = accountIdFromEvent(event);
  if (!accountId) return json(200, { ignored: "no account id on the event" });

  let sub: Json | null = null;
  if (SUBSCRIPTION_EVENTS.has(type)) {
    sub = event.data.object;
  } else if (type === "checkout.session.completed") {
    const id = event?.data?.object?.subscription;
    sub = typeof id === "string" ? await deps.fetchSubscription(id) : null;
  } else if (type === "invoice.payment_failed") {
    const id = event?.data?.object?.subscription;
    sub = typeof id === "string" ? await deps.fetchSubscription(id) : null;
  }
  if (!sub) return json(200, { ignored: `no subscription on ${type}` });

  await deps.writeEntitlement(accountId, entitlementFromSubscription(sub));
  return json(200, { ok: true });
}
