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
 * two emails Stripe does not: the confirmation when someone subscribes (D6b), on
 * `checkout.session.completed`, and the reminder before a trial's first charge (D6), on
 * `customer.subscription.trial_will_end`, which writes no entitlement.
 */
import { EntitlementStatus } from "../_shared/entitlement.ts";
import { fail, json, methodNotAllowed } from "../_shared/http.ts";
import { verifyStripeSignature } from "../_shared/stripe.ts";
import { readFailure } from "./subscription_read.ts";
import {
  lookupPrice,
  type PlanKey,
  priceText,
  type ResolvedCatalog,
  type ResolvedEntry,
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
  /**
   * `checkout.session.completed` and `invoice.payment_failed` carry a subscription id, not the
   * subscription. `null` means Stripe answered 404 (no such subscription), which the handler logs
   * by event type and ignores; a throw means the read failed (another status, the network, a body
   * whose `object` is not "subscription"), and the handler answers 502 so Stripe redelivers. A
   * `SubscriptionReadError` names the status for the log.
   */
  fetchSubscription: (id: string) => Promise<Json | null>;
  /** The price catalog, every variable optional: an unset one matches no id, never a 500 (D2). */
  catalog: ResolvedCatalog;
  /** `accounts.email` for this account: the confirmation (D6b) and the reminder (D6) go only to the
   * account's own address. */
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
 * cannot be stated truthfully (not US dollars, say) is not guessed. Both mails (D6, D6b) say it
 * this way, so they cannot drift apart. `entry` is the matched catalog entry, or `null`.
 */
function planWords(sub: Json, catalog: ResolvedCatalog): { what: string; entry: ResolvedEntry | null } {
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
  return { what, entry };
}

/**
 * The period end in seconds, or `null`. **The period end moved.** In Stripe API `2025-03-31.basil`
 * `current_period_end` left the Subscription for each item; `_shared/stripe.ts` pins that version,
 * and the item is read first so the field is found on either shape rather than becoming a silent
 * `null`.
 */
function periodEndSeconds(sub: Json): number | null {
  const item = sub?.items?.data?.[0] ?? null;
  if (typeof item?.current_period_end === "number") return item.current_period_end;
  return typeof sub?.current_period_end === "number" ? sub.current_period_end : null;
}

/** D6: the reminder before a trial's first charge. */
export function trialReminder(sub: Json, catalog: ResolvedCatalog, to: string): Mail {
  const { what } = planWords(sub, catalog);
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

/**
 * D6b: the confirmation the terms promise when someone subscribes: the plan, the price, how often it
 * renews and how to cancel. The plan and price are `planWords`, the tax phrase and the money-back
 * sentence the catalog's own words. The date is the first charge when trialing, else the period end
 * read as the entitlement reads it; with neither, no date line, never a guessed one. No id, no link.
 */
export function subscribeConfirmation(sub: Json, catalog: ResolvedCatalog, to: string): Mail {
  const { what, entry } = planWords(sub, catalog);
  const trial = sub?.status === "trialing" && typeof sub?.trial_end === "number";
  const started = trial ? "Your Knowlu free trial has started" : "Your Knowlu subscription has started";
  const end = periodEndSeconds(sub);
  const dateLine = trial
    ? `Your first charge is on ${chargeDate(sub.trial_end)}, when your free trial ends.`
    : (end === null ? null : `Your next charge is on ${chargeDate(end)}.`);
  const discounted = (Array.isArray(sub?.discounts) && sub.discounts.length > 0) ||
    (typeof sub?.discount === "object" && sub.discount !== null);
  const lines: (string | null)[] = [
    `${started}. Keep this message: it is your confirmation.`,
    "",
    `Your plan: ${what}, ${catalog.words.tax}.`,
    dateLine,
    discounted
      ? "A promotion code applies to this subscription, so you are charged less than the price above."
      : null,
    "It renews automatically until you cancel.",
    entry?.key === "yearly" ? `${catalog.words.moneyBack}.` : null,
    "",
    // The reminder's wording without "before then"; "Manage subscription" on one line, as there.
    "To cancel, open Knowlu and go to Settings, then Membership, then",
    "Manage subscription, or reply to this message and we will cancel it for you.",
  ];
  return { to, subject: started, text: lines.filter((l) => l !== null).join("\n") };
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
  // The item's period end first, the subscription's as the fallback (`periodEndSeconds`).
  const endSeconds = periodEndSeconds(sub);
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
const CHECKOUT_COMPLETED = "checkout.session.completed";

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
    !SUBSCRIPTION_EVENTS.has(type) && type !== CHECKOUT_COMPLETED &&
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
  let fetchId: unknown = null;
  if (SUBSCRIPTION_EVENTS.has(type)) {
    sub = event.data.object;
  } else if (type === CHECKOUT_COMPLETED) {
    fetchId = event?.data?.object?.subscription;
  } else if (type === "invoice.payment_failed") {
    // Under the pinned `2025-03-31.basil` API version, an Invoice's subscription moved to
    // `parent.subscription_details.subscription`; the pre-basil `subscription` field is the fallback.
    fetchId = event?.data?.object?.parent?.subscription_details?.subscription ??
      event?.data?.object?.subscription;
  }
  if (typeof fetchId === "string") {
    try {
      sub = await deps.fetchSubscription(fetchId);
    } catch (e) {
      // Not "no subscription": a read that failed. 502 and nothing recorded, so Stripe redelivers and
      // neither the write nor the confirmation is lost to a short outage. The type and the status
      // (or "network") only: no id, no address, no error text.
      console.error(`stripe-webhook: ${type}: the subscription could not be read (${readFailure(e)})`);
      return fail(502, "the subscription could not be read");
    }
  }
  if (typeof fetchId === "string" && !sub) {
    // Stripe keeps cancelled subscriptions readable, so a 404 for one an event names usually means a key
    // of the wrong mode or account: one line, so that shows. The type only: no id, no account id.
    console.error(`stripe-webhook: ${type}: Stripe has no such subscription`);
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
  const stale = Number.isFinite(seenMs) && Number.isFinite(createdMs) && seenMs >= createdMs;
  if (!stale) {
    await deps.writeEntitlement(accountId, entitlementFromSubscription(sub, deps.catalog), createdIso);
  }
  const body = stale ? { stale: eventId } : { ok: true };
  if (type === CHECKOUT_COMPLETED) {
    return await confirmSubscription(event?.data?.object, sub, eventId, accountId, createdIso, body, deps);
  }
  await deps.recordEvent(eventId, type, createdIso);
  return json(200, body);
}

/**
 * D6b: one confirmation per subscription checkout whose subscription is `trialing` or `active`, to
 * the account's own address. It does not wait behind the stale guard, which guards the entitlement
 * write only: `customer.subscription.created` often lands first with the same or a later timestamp.
 * A failed send answers 502 and records nothing, so Stripe retries; the retry's write is stale and
 * skipped, and the mail goes. After a send nothing answers 5xx, or the retry would mail twice.
 */
async function confirmSubscription(
  session: Json,
  sub: Json,
  eventId: string,
  accountId: string,
  createdIso: string,
  body: Record<string, unknown>,
  deps: Deps,
): Promise<Response> {
  const mode = session?.mode;
  const subscriptionCheckout = mode === "subscription" ||
    (mode == null && typeof session?.subscription === "string");
  const started = sub?.status === "trialing" || sub?.status === "active";
  const to = subscriptionCheckout && started ? await deps.accountEmail(accountId) : null;
  if (!to) {
    await deps.recordEvent(eventId, CHECKOUT_COMPLETED, createdIso);
    return json(200, { ...body, confirmed: false });
  }
  try {
    await deps.sendEmail(subscribeConfirmation(sub, deps.catalog, to));
  } catch {
    // The account id only: never the address, the error (which can echo it) or an id of Stripe's.
    console.error(`stripe-webhook: ${accountId}: the subscribe confirmation could not be sent`);
    return fail(502, "the subscribe confirmation could not be sent");
  }
  try {
    await deps.recordEvent(eventId, CHECKOUT_COMPLETED, createdIso);
  } catch {
    console.error(`stripe-webhook: ${accountId}: the confirmation was sent but not recorded`);
  }
  return json(200, { ...body, confirmed: true });
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
