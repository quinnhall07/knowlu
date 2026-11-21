/**
 * **The only writer of `entitlements`** (spec §5.1). Nothing else in this codebase may write that
 * table, and no client can: RLS grants `select` and nothing more, and only the service role — which
 * this function holds and no browser does — bypasses it.
 *
 * The raw body is read as text and verified *before* it is parsed. Parsing first would mean acting
 * on a shape an attacker chose.
 *
 * The plan comes from the price id, through the price catalog (v0.4.1 note D2, D13): never from the
 * interval, because after a price change two prices share one key and one cadence. It also sends the
 * one email Stripe does not: the reminder before a trial's first charge (D6), on
 * `customer.subscription.trial_will_end`, which writes no entitlement.
 */
import { EntitlementStatus } from "../_shared/entitlement.ts";
import { fail, json, methodNotAllowed } from "../_shared/http.ts";
import { verifyStripeSignature } from "../_shared/stripe.ts";
import {
  lookupPrice,
  type PlanKey,
  priceText,
  type ResolvedCatalog,
  unknownPriceText,
} from "../_shared/price_catalog.ts";

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
  /**
   * `true` when `accounts.id` still has a row. `DELETE /account` can leave Stripe still sending
   * events for a customer we no longer know: `customer.subscription.updated` fires immediately from
   * the `cancel_at_period_end` write, and `customer.subscription.deleted` follows at period end.
   * Without this check `writeEntitlement` would upsert into `entitlements` against a foreign key that
   * no longer resolves — a 502 Stripe retries for days and can eventually disable the endpoint.
   */
  accountExists: (accountId: string) => Promise<boolean>;
  writeEntitlement: (accountId: string, row: EntitlementWrite, updatedAt: string) => Promise<void>;
  /** `checkout.session.completed` carries a subscription id, not the subscription. */
  fetchSubscription: (id: string) => Promise<Json | null>;
  /** The price catalog, every variable optional: an unset one matches no id, never a 500 (D2). */
  catalog: ResolvedCatalog;
  /** `accounts.email` for this account: the reminder goes only to the account's own address. */
  accountEmail: (accountId: string) => Promise<string | null>;
  sendEmail: (m: Mail) => Promise<void>;
}

export interface Mail {
  to: string;
  subject: string;
  text: string;
}

const WEEKDAYS = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const MONTHS = [
  "January",
  "February",
  "March",
  "April",
  "May",
  "June",
  "July",
  "August",
  "September",
  "October",
  "November",
  "December",
];

/**
 * The date of a charge, written in UTC-10 (Hawaii), the westernmost of the fifty states: never later
 * than the date the student sees anywhere in them, so a student who cancels by it is never late.
 * No time zone is assumed beyond that bound.
 */
export function chargeDate(seconds: number): string {
  const d = new Date((seconds - 10 * 60 * 60) * 1000);
  return `${WEEKDAYS[d.getUTCDay()]}, ${MONTHS[d.getUTCMonth()]} ${d.getUTCDate()}, ${d.getUTCFullYear()}`;
}

/** The plan's words, as Settings writes them (D9). Cadence only, never a price. */
const PLAN_WORDS: Record<PlanKey, string> = { yearly: "Yearly", four_weekly: "Every 4 weeks" };

/**
 * D6: the plan words and the amount from the catalog entry the price id matches; for an id the
 * catalog lacks, no plan words and the event's own price, through the same formatter. A price that
 * cannot be stated truthfully (not US dollars, say) is not guessed.
 */
export function trialReminder(sub: Json, catalog: ResolvedCatalog, to: string): Mail {
  const price = sub?.items?.data?.[0]?.price ?? {};
  const entry = typeof price?.id === "string" ? lookupPrice(catalog, price.id) : null;
  const known = entry ? `Knowlu, ${PLAN_WORDS[entry.key]}, ${priceText(entry)}` : null;
  const stated = unknownPriceText({
    cents: price?.unit_amount,
    currency: String(price?.currency ?? ""),
    interval: String(price?.recurring?.interval ?? ""),
    count: price?.recurring?.interval_count ?? 1,
  });
  const what = known ?? (stated ? `Knowlu, ${stated}` : "Knowlu, at the price shown in Manage subscription");
  return {
    to,
    subject: "Your Knowlu free trial is ending",
    text: [
      "Your Knowlu free trial is ending.",
      "",
      `Your first charge is on ${chargeDate(sub.trial_end)}:`,
      `${what}, ${catalog.words.tax}.`,
      "It renews automatically until you cancel.",
      "",
      // "Manage subscription" on one line: it is the button's exact name.
      "To cancel before then, open Knowlu and go to Settings, then Membership, then",
      "Manage subscription, or reply to this message and we will cancel it for you.",
    ].join("\n"),
  };
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

export function entitlementFromSubscription(sub: Json, catalog: ResolvedCatalog): EntitlementWrite {
  const item = sub?.items?.data?.[0] ?? null;
  // D2: the price id, looked up among the catalog's selling and kept entries. Any other id is
  // `null`, which `billing-jobs` never mails a guessed price for.
  const priceId = item?.price?.id;
  const plan = typeof priceId === "string" ? lookupPrice(catalog, priceId)?.key ?? null : null;
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
    // A `pause_collection` window leaves Stripe's status `active`. The summer pause is gone (D10);
    // the column stays, because dropping it needs a migration and buys nothing.
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

const TRIAL_WILL_END = "customer.subscription.trial_will_end";

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
    type !== "invoice.payment_failed" && type !== TRIAL_WILL_END
  ) {
    return json(200, { ignored: type });
  }

  const eventId = String(event?.id ?? "");
  if (!eventId) return fail(400, "the event has no id");
  if (await deps.seenEvent(eventId)) return json(200, { duplicate: eventId });
  const createdIso = new Date((Number(event?.created) || deps.nowSeconds()) * 1000).toISOString();

  const accountId = accountIdFromEvent(event);
  if (!accountId) return json(200, { ignored: "no account id on the event" });
  // A deleted account: nothing left to write against, and nothing to retry — acknowledge and drop.
  if (!(await deps.accountExists(accountId))) return json(200, { ignored: "no such account" });

  if (type === TRIAL_WILL_END) {
    return await remindBeforeFirstCharge(event, eventId, accountId, createdIso, deps);
  }

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
  await deps.writeEntitlement(accountId, entitlementFromSubscription(sub, deps.catalog), createdIso);
  await deps.recordEvent(eventId, type, createdIso);
  return json(200, { ok: true });
}

/**
 * D6: one reminder before a trial's first charge. Sent only while the subscription is trialing, not
 * set to cancel, and its trial ends in the future; a retry that arrives after the trial ended sends
 * nothing and is recorded. A failed send answers 502 and records nothing, so Stripe retries. Writes
 * no entitlement.
 */
async function remindBeforeFirstCharge(
  event: Json,
  eventId: string,
  accountId: string,
  createdIso: string,
  deps: Deps,
): Promise<Response> {
  const sub = event?.data?.object ?? {};
  const willCharge = sub.status === "trialing" && sub.cancel_at_period_end !== true &&
    typeof sub.trial_end === "number" && sub.trial_end > deps.nowSeconds();
  if (!willCharge) {
    await deps.recordEvent(eventId, TRIAL_WILL_END, createdIso);
    return json(200, { ignored: "no charge is coming" });
  }
  const to = await deps.accountEmail(accountId);
  if (!to) {
    await deps.recordEvent(eventId, TRIAL_WILL_END, createdIso);
    return json(200, { ignored: "no address for the account" });
  }
  try {
    await deps.sendEmail(trialReminder(sub, deps.catalog, to));
  } catch {
    // The account id only: never the address, the error (which can echo it) or a token.
    console.error(`stripe-webhook: ${accountId}: the trial reminder could not be sent`);
    return fail(502, "the trial reminder could not be sent");
  }
  // Sent: from here nothing may answer 5xx, or Stripe retries, `seenEvent` is still false, and the
  // student gets a second reminder. A record that fails is logged (the account id only) and dropped.
  try {
    await deps.recordEvent(eventId, TRIAL_WILL_END, createdIso);
  } catch {
    console.error(`stripe-webhook: ${accountId}: the reminder was sent but not recorded`);
  }
  return json(200, { reminded: true });
}
