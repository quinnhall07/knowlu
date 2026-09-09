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
  stripe_subscription_id: string | null;
  paused: boolean;
  started_at: string | null;
}

export interface Deps {
  secret: string;
  nowSeconds: () => number;
  seenEvent: (eventId: string) => Promise<boolean>;
  recordEvent: (eventId: string, type: string, createdIso: string) => Promise<void>;
  /** `entitlements.updated_at` for this account, or null. It carries the EVENT's `created`. */
  currentUpdatedAt: (accountId: string) => Promise<string | null>;
  writeEntitlement: (accountId: string, row: EntitlementWrite, updatedAt: string) => Promise<void>;
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
  const item = sub?.items?.data?.[0] ?? null;
  const interval = item?.price?.recurring?.interval ?? null;
  const plan = interval === "month" ? "monthly" : interval === "year" ? "academic_year" : null;
  // **The period end moved.** In Stripe API `2025-03-31.basil` `current_period_end` left the
  // Subscription for each item; `_shared/stripe.ts` pins that version, and the item is read first so
  // the field is found on either shape rather than becoming a silent `null` in every row.
  const endSeconds = typeof item?.current_period_end === "number"
    ? item.current_period_end
    : (typeof sub?.current_period_end === "number" ? sub.current_period_end : null);
  const end = endSeconds === null ? null : new Date(endSeconds * 1000).toISOString();
  const started = typeof sub?.start_date === "number" ? new Date(sub.start_date * 1000).toISOString() : null;
  return {
    plan,
    status: statusOf(String(sub?.status ?? "")),
    current_period_end: end,
    stripe_subscription_id: typeof sub?.id === "string" ? sub.id : null,
    // A `pause_collection` window leaves Stripe's status `active`, so this flag — not the status —
    // is what the summer job reads to know whether it has already acted.
    paused: !!sub?.pause_collection,
    started_at: started,
  };
}

export function accountIdFromEvent(event: Json): string | null {
  const o = event?.data?.object ?? {};
  // In order: a subscription's own metadata, an invoice's parent.subscription_details.metadata under
  // the pinned `2025-03-31.basil` API version (invoice metadata is the invoice's own and is empty),
  // the pre-basil subscription_details.metadata location, then Checkout's client_reference_id.
  return o?.metadata?.account_id ?? o?.parent?.subscription_details?.metadata?.account_id ??
    o?.subscription_details?.metadata?.account_id ?? o?.client_reference_id ?? null;
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

  const eventId = String(event?.id ?? "");
  if (!eventId) return fail(400, "the event has no id");
  if (await deps.seenEvent(eventId)) return json(200, { duplicate: eventId });
  const createdIso = new Date((Number(event?.created) || deps.nowSeconds()) * 1000).toISOString();

  const accountId = accountIdFromEvent(event);
  if (!accountId) return json(200, { ignored: "no account id on the event" });

  let sub: Json | null = null;
  if (SUBSCRIPTION_EVENTS.has(type)) {
    sub = event.data.object;
  } else if (type === "checkout.session.completed") {
    const id = event?.data?.object?.subscription;
    sub = typeof id === "string" ? await deps.fetchSubscription(id) : null;
  } else if (type === "invoice.payment_failed") {
    // Under the pinned `2025-03-31.basil` API version, an Invoice's subscription moved to
    // `parent.subscription_details.subscription`; the pre-basil `subscription` field is the fallback.
    const id = event?.data?.object?.parent?.subscription_details?.subscription ??
      event?.data?.object?.subscription;
    sub = typeof id === "string" ? await deps.fetchSubscription(id) : null;
  }
  if (!sub) return json(200, { ignored: `no subscription on ${type}` });

  // A late event must not resurrect a state a later one already replaced. Compared as INSTANTS, not
  // strings: `createdIso` is `toISOString()` (`…:00.000Z`) but PostgREST's own `timestamptz` text
  // (`…:00+00:00`, and not necessarily UTC if the database's `TimeZone` is ever anything else) sorts
  // differently byte for byte — `'+'` sorts before `'.'`, so a string compare never calls an event
  // stale even when its `created` exactly equals the row's `updated_at`. A parse failure on either
  // side is treated as "not stale" — a row we cannot date does not get to block a write.
  const seen = await deps.currentUpdatedAt(accountId);
  const seenMs = seen === null ? NaN : Date.parse(seen);
  const createdMs = Date.parse(createdIso);
  if (Number.isFinite(seenMs) && Number.isFinite(createdMs) && seenMs >= createdMs) {
    await deps.recordEvent(eventId, type, createdIso);
    return json(200, { stale: eventId });
  }
  await deps.writeEntitlement(accountId, entitlementFromSubscription(sub), createdIso);
  await deps.recordEvent(eventId, type, createdIso);
  return json(200, { ok: true });
}
