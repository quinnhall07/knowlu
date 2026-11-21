/**
 * The two things a subscription needs that neither Stripe nor a user does on their own:
 *
 * 1. **The move** (v0.4.1 note D14). When a price changes and its amendment rules `move` for the old
 *    price, a subscriber on it is emailed 7 to 30 days before a renewal, then marked, then moved to
 *    the key's selling price with no proration, so the period already paid never changes. Tell first,
 *    act second: a failed notice changes nothing.
 * 2. **The annual reminder** California's ARL requires: what it is, what it costs, how often, and how
 *    to cancel. The amount is the subscriber's own, read from their Stripe subscription and stated
 *    in the price catalog's words (D10); a price the catalog lacks is never mailed a guess.
 *
 * The summer pause is gone (D10): no subscriber is ever sent a `pause_collection` call.
 *
 * Driven by `pg_cron` once a day, authenticated by a shared job token — not by a user's JWT, since
 * there is no user, and not by the service-role key, which has no business in a database's config.
 *
 * **One subscriber's failure never stops the run**: each is wrapped in its own `try/catch`, logged
 * by account id only (never an email address or a token), and counted in `failed`.
 */
import { fail, json, methodNotAllowed } from "../_shared/http.ts";
import { equalHex, StripePost } from "../_shared/stripe.ts";
import {
  type Entry,
  lookupPrice,
  type PlanKey,
  priceText,
  type ResolvedCatalog,
  type ResolvedEntry,
  sellingFor,
} from "../_shared/price_catalog.ts";
import { chargeDate } from "../stripe-webhook/handler.ts";

// deno-lint-ignore no-explicit-any
type Json = any;

export interface Subscriber {
  account_id: string;
  email: string;
  subscription_id: string;
  plan: string | null;
  paused: boolean;
  started_at: string;
  last_reminded_at: string | null;
  current_period_end: string | null;
}

export interface Mail {
  to: string;
  subject: string;
  text: string;
}

export interface Deps {
  token: string;
  now: () => Date;
  listSubscribers: () => Promise<Subscriber[]>;
  /** The Subscription, through `stripeGetFrom` (pinned version); `null` when it cannot be read. */
  getSubscription: (id: string) => Promise<Json | null>;
  /** The move's two writes: the mark, then the price change. */
  stripe: StripePost;
  sendEmail: (m: Mail) => Promise<void>;
  recordReminder: (accountId: string, at: string) => Promise<void>;
  /** The price catalog, every variable optional: an unset one matches no id, never a 500 (D14). */
  catalog: ResolvedCatalog;
}

const DAY_MS = 24 * 60 * 60 * 1000;
const YEAR_MS = 365 * DAY_MS;

/** The subscription metadata key the move marks a noticed renewal with. Only this key is written:
 * Stripe merges metadata, so `metadata[account_id]`, which the webhook keys on, is never touched. */
const NOTICE_KEY = "knowlu_price_notice";

export function reminderDue(a: { lastSentAt: string | null; startedAt: string; now: Date }): boolean {
  const since = Date.parse(a.lastSentAt ?? a.startedAt);
  if (!Number.isFinite(since)) return false;
  return a.now.getTime() - since > YEAR_MS;
}

/** The plan's words, as Settings writes them (D9). Cadence only, never a price. */
const PLAN_WORDS: Record<PlanKey, string> = { yearly: "Yearly", four_weekly: "Every 4 weeks" };

/** D14's window, exact: `N × 24h ≤ periodEnd − now < (N + 6) × 24h`, on Stripe's own period end. */
export function inMoveWindow(periodEndMs: number, nowMs: number, noticeDays: number): boolean {
  const ahead = periodEndMs - nowMs;
  return ahead >= noticeDays * DAY_MS && ahead < (noticeDays + 6) * DAY_MS;
}

/** Whom the move reads: a view copy within one day either side of some `move` entry's window. The
 * view's copy only chooses; the decision and the date stated use the Stripe read. */
function mayBeDue(s: Subscriber, nowMs: number, catalog: ResolvedCatalog): boolean {
  const viewEnd = s.current_period_end === null ? NaN : Date.parse(s.current_period_end);
  if (!Number.isFinite(viewEnd)) return false;
  const ahead = viewEnd - nowMs;
  return catalog.kept.some((k) =>
    k.rule === "move" && ahead >= (k.noticeDays - 1) * DAY_MS && ahead < (k.noticeDays + 7) * DAY_MS
  );
}

/** At the pinned `2025-03-31.basil` the period end is on the item; the pre-basil field is the fallback. */
function periodEndSeconds(sub: Json): number | null {
  const item = sub?.items?.data?.[0];
  if (typeof item?.current_period_end === "number") return item.current_period_end;
  return typeof sub?.current_period_end === "number" ? sub.current_period_end : null;
}

function annualReminder(s: Subscriber, entry: Entry, catalog: ResolvedCatalog): Mail {
  return {
    to: s.email,
    subject: "Your Knowlu subscription",
    text: [
      "This is your yearly reminder that you have a Knowlu subscription.",
      "",
      `Knowlu, ${priceText(entry)}, ${catalog.words.tax}, renewing automatically until you cancel.`,
      "",
      "To cancel, open Knowlu, go to Settings, then Membership, and choose Manage subscription — or",
      "reply to this message and we will cancel it for you.",
    ].join("\n"),
  };
}

function priceNotice(
  s: Subscriber,
  from: ResolvedEntry,
  to: ResolvedEntry,
  renewalSeconds: number,
  catalog: ResolvedCatalog,
): Mail {
  return {
    to: s.email,
    subject: "Your Knowlu price is changing",
    text: [
      `The price of your Knowlu plan (${PLAN_WORDS[from.key]}) is changing at your next renewal.`,
      "",
      `Now: ${priceText(from)}.`,
      `From your renewal on ${chargeDate(renewalSeconds)}: ${priceText(to)}, ${catalog.words.tax}.`,
      "",
      "The period you have already paid for does not change. It renews automatically until you",
      "cancel. To cancel before your renewal, open Knowlu and go to Settings, then Membership, then",
      "Manage subscription, or reply to this message and we will cancel it for you.",
    ].join("\n"),
  };
}

/**
 * D14, for one subscription already read: notice, mark, change, in that order. Returns the entry the
 * subscription pays from its next renewal on when it was moved, `null` when it was not due.
 */
async function moveIfDue(s: Subscriber, sub: Json, nowMs: number, deps: Deps): Promise<ResolvedEntry | null> {
  const item = sub?.items?.data?.[0];
  const priceId = item?.price?.id;
  const kept = deps.catalog.kept.find((k) => k.priceId !== null && k.priceId === priceId);
  if (!kept || kept.rule !== "move") return null;
  if (sub.cancel_at_period_end === true) return null;
  if (sub.status !== "active" && sub.status !== "trialing") return null;
  const end = periodEndSeconds(sub);
  if (end === null || !inMoveWindow(end * 1000, nowMs, kept.noticeDays)) return null;
  const selling = sellingFor(deps.catalog, kept.key);
  // The trial rule (Q15, as the note recommends): on a raise, a trial's first charge is the amount
  // its consent row records, so the move waits until the subscription is active. A cut moves it.
  // Asked before the selling price id is needed, so an unset one never fails a trial it would skip.
  if (selling && sub.status === "trialing" && selling.cents > kept.cents) return null;
  if (!selling || selling.priceId === null) throw new Error(`no selling price for ${kept.key}`);
  if (typeof item?.id !== "string") throw new Error("the subscription has no item id");

  const path = `/v1/subscriptions/${encodeURIComponent(s.subscription_id)}`;
  // The mark names this renewal and the price it moves to, so a later change of either is noticed
  // again rather than acted on silently.
  const mark = `${end}:${selling.priceId}`;
  if (sub?.metadata?.[NOTICE_KEY] !== mark) {
    await deps.sendEmail(priceNotice(s, kept, selling, end, deps.catalog));
    await deps.stripe(path, { [`metadata[${NOTICE_KEY}]`]: mark });
  }
  await deps.stripe(path, {
    "items[0][id]": item.id,
    "items[0][price]": selling.priceId,
    "proration_behavior": "none",
  });
  return selling;
}

export async function handle(req: Request, deps: Deps): Promise<Response> {
  if (req.method !== "POST") return methodNotAllowed(["POST"]);
  // R-C1-59 (M9): the same length-then-XOR compare `_shared/stripe.ts` uses for a webhook signature.
  if (!equalHex(req.headers.get("x-knowlu-job-token") ?? "", deps.token)) {
    return fail(401, "not a job caller");
  }

  const now = deps.now();
  const nowMs = now.getTime();
  let reminded = 0, moved = 0, failed = 0;

  for (const s of await deps.listSubscribers()) {
    try {
      const due = reminderDue({ lastSentAt: s.last_reminded_at, startedAt: s.started_at, now });
      // Only a subscriber something may be due to costs a Stripe read.
      if (!due && !mayBeDue(s, nowMs, deps.catalog)) continue;
      const sub = await deps.getSubscription(s.subscription_id);
      if (!sub) {
        console.error(`billing-jobs: ${s.account_id}: the subscription could not be read`);
        failed++;
        continue;
      }

      // The move first (r5: m6), so a reminder sent the same day states what the renewal charges.
      const priceId = sub?.items?.data?.[0]?.price?.id;
      let entry: ResolvedEntry | null = typeof priceId === "string"
        ? lookupPrice(deps.catalog, priceId)
        : null;
      const movedTo = await moveIfDue(s, sub, nowMs, deps);
      if (movedTo) {
        moved++;
        entry = movedTo;
      }

      if (due) {
        // A subscriber whose price we do not recognize is never mailed a guessed price.
        if (!entry) {
          console.error(`billing-jobs: ${s.account_id}: the subscription's price is not in the catalog`);
          failed++;
          continue;
        }
        await deps.sendEmail(annualReminder(s, entry, deps.catalog));
        await deps.recordReminder(s.account_id, now.toISOString());
        reminded++;
      }
    } catch (e) {
      // Never an email address or a token: only the account id, and either the thrown Response's
      // status or the error's own string.
      console.error(`billing-jobs: ${s.account_id}: ${e instanceof Response ? e.status : String(e)}`);
      failed++;
    }
  }

  return json(200, { reminded, moved, failed });
}
