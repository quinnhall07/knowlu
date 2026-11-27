/**
 * T1d: the Stripe calls `DELETE /account` makes before anything is deleted: one read of the stored
 * subscription's state; whenever the account has a Stripe customer, one list of that customer's
 * subscriptions (the stored record can lag Stripe, fix rounds 1 and 2); and the update that sets each
 * subscription that can still charge to cancel at period end.
 *
 * All go through `_shared/stripe.ts`'s helpers, so the key, the pinned `Stripe-Version` and the form
 * encoding live in one place; that file does not change here (the lane's note D7). What this file adds
 * is the `fetch` they run over, which decides every non-2xx before the helper sees it:
 *
 * - **The reads' one pass-through:** the answer that means "Stripe does not know this object" is the
 *   only refusal handed back to `stripeGetFrom`, and the call records that it arrived; "unknown" is
 *   answered only then, never for a 2xx whose body is `null`. For the subscription read that is a 404
 *   `resource_missing`; for the customer's list, a 400 or 404 `resource_missing` on `customer`
 *   (`CUSTOMER_MISSING`, as Stripe was measured to answer). An answer that says a similar object
 *   exists in the other mode is a key of the wrong mode, not a missing object, and is refused like the
 *   rest: reading it as "nothing to cancel" would delete an account whose subscription still charges.
 * - **Everything else is the delete's 502**, the same answer `stripePostFrom` gives a refusal: any
 *   other status, a network error, a 2xx that is not JSON or not the shape expected, and every refusal
 *   of an update. A delete never proceeds past a subscription whose state is unknown, so nobody is
 *   charged after their account is gone.
 *
 * Stripe's error body can echo an email and always names the id it refused. It is logged, cut and
 * redacted of every Stripe object id (the platform's account and request ids, client secrets too),
 * emails and keys, and never returned. A refused path is never logged: it carries the id
 * (`stripePostFrom`'s own log line would, so a refusal is thrown here, before that helper reads it).
 */
import { fail } from "../_shared/http.ts";
import { stripeGetFrom, type StripePost, stripePostFrom } from "../_shared/stripe.ts";

/** The 502 `stripePostFrom` gives a refusal, word for word: the app already says it to the student. */
const REFUSED = "the payment provider refused the request";
const MAX_LOGGED = 600;

/** The shapes of a Stripe Subscription and Customer id, checked before one goes into a path or a query. */
export const SUBSCRIPTION_ID = /^sub_[A-Za-z0-9]+$/;
export const CUSTOMER_ID = /^cus_[A-Za-z0-9]+$/;

// Fix round 2: an id carrying `_secret_` (a client secret, `pi_…_secret_…`) is removed whole. It has
// no word boundary inside it, so the general rule below would leave it standing.
const SECRET_ID = /\b[a-z]+(?:_[A-Za-z0-9]+)*?_secret_[A-Za-z0-9]+/g;
const SUB_OR_CUSTOMER_ID = /\b(?:sub|cus)_[A-Za-z0-9_]+/g;
// Every other Stripe object id (`acct_`, `req_`, `in_`, `pm_`, `sub_sched_`, ...): lower-case prefix
// words, then a random part that holds a digit or a capital. Stripe's codes (`card_declined`,
// `resource_missing`) are lower-case words only, so they survive.
const STRIPE_ID = /\b[a-z]+(?:_[a-z]+)*_(?=[A-Za-z0-9]*[0-9A-Z])[A-Za-z0-9]+\b/g;
const API_KEY = /\b(?:sk|rk|pk)_(?:test|live)_[A-Za-z0-9_*]+/g;
const EMAIL = /[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/g;
const OTHER_MODE = /similar object exists in (?:live|test) mode/i;

/** Stripe's error text, safe to log: no Stripe object id, no email, no key, and short. */
export function redactStripeText(text: string): string {
  const out = text.replace(EMAIL, "<email>").replace(SECRET_ID, "<secret>").replace(API_KEY, "<key>")
    .replace(SUB_OR_CUSTOMER_ID, "<id>").replace(STRIPE_ID, "<id>");
  return out.length > MAX_LOGGED ? `${out.slice(0, MAX_LOGGED - 3)}...` : out;
}

function refused(): Response {
  return fail(502, REFUSED);
}

/** The parts of Stripe's `error` a missing-object rule reads. `param` is `null` when Stripe sent none
 * (absent or `null`); a `param` that is not a string is kept as `"?"`, which no rule accepts. */
interface StripeErrorParts {
  code: string | null;
  message: string | null;
  param: string | null;
}

/** Stripe's `error` parts, or nulls when the body is not Stripe's JSON. */
function stripeError(text: string): StripeErrorParts {
  try {
    const e = (JSON.parse(text) as { error?: { code?: unknown; message?: unknown; param?: unknown } } | null)
      ?.error;
    return {
      code: typeof e?.code === "string" ? e.code : null,
      message: typeof e?.message === "string" ? e.message : null,
      param: e?.param === undefined || e?.param === null ? null : typeof e.param === "string" ? e.param : "?",
    };
  } catch {
    return { code: null, message: null, param: null };
  }
}

/** Decides whether a refused answer means "Stripe does not know this object". */
type MissingRule = (status: number, e: StripeErrorParts) => boolean;

const notOtherMode = (e: StripeErrorParts) => !OTHER_MODE.test(e.message ?? "");

/** The single-subscription read: a 404 `resource_missing`, and only that (not the other mode's). */
const SUBSCRIPTION_MISSING: MissingRule = (status, e) =>
  status === 404 && e.code === "resource_missing" && notOtherMode(e);

/**
 * Fix round 2, measured on Stripe's test mode: a list filtered by a customer Stripe does not know
 * answers **400** `resource_missing`, `param: customer`, "No such customer: '<id>'". For the list only,
 * a 400 or a 404 `resource_missing` naming the customer means nothing to cancel: by `param`, or, when
 * Stripe sends no `param`, by a message that begins "No such customer". Not the other mode's.
 */
const CUSTOMER_MISSING: MissingRule = (status, e) =>
  (status === 400 || status === 404) && e.code === "resource_missing" && notOtherMode(e) &&
  (e.param === "customer" || (e.param === null && (e.message ?? "").startsWith("No such customer")));

/**
 * A `fetch` that answers a 2xx untouched and decides every other answer itself. With `missing`, an
 * answer its `rule` accepts calls its `mark` and comes back as a bodiless 404, its body never logged;
 * every other non-2xx, and a network error, is logged by `label`, its status and Stripe's redacted
 * body, and thrown as 502. An update passes no `missing`: every refusal of a write is the 502.
 */
function deleteFetch(
  fetchImpl: typeof fetch,
  label: string,
  missing: { rule: MissingRule; mark: () => void } | null,
): typeof fetch {
  return async (input, init) => {
    let res: Response;
    try {
      res = await fetchImpl(input, init);
    } catch (e) {
      // The error's name only: a network error's message names the URL, and the URL names the id.
      console.error(
        `account delete: ${label} did not reach the payment provider (${(e as Error)?.name ?? "error"})`,
      );
      throw refused();
    }
    if (res.ok) return res;
    const text = await res.text().catch(() => "");
    if (missing && missing.rule(res.status, stripeError(text))) {
      missing.mark();
      return new Response(null, { status: 404 });
    }
    console.error(`account delete: ${label} answered ${res.status} ${redactStripeText(text)}`);
    throw refused();
  };
}

/** Any throw that is not already a `Response` (a 2xx that is not JSON, say) becomes the 502. */
async function or502<T>(label: string, run: () => Promise<T>): Promise<T> {
  try {
    return await run();
  } catch (e) {
    if (e instanceof Response) throw e;
    console.error(`account delete: ${label} gave an unreadable answer (${(e as Error)?.name ?? "error"})`);
    throw refused();
  }
}

/**
 * One versioned GET through `stripeGetFrom`, over a `deleteFetch` built for this call alone, so that
 * "Stripe does not know it" is answered only when an answer `rule` accepts really arrived. A `null`
 * from the helper is otherwise a body like any other (a 2xx whose JSON is `null`), and the caller's
 * shape check refuses it (review finding 1: that `null` used to read as "missing" and was purged past).
 */
async function getOrMissing(
  secret: string,
  fetchImpl: typeof fetch,
  label: string,
  path: string,
  rule: MissingRule,
): Promise<{ body: unknown } | "missing"> {
  let missing = false;
  const get = stripeGetFrom(secret, deleteFetch(fetchImpl, label, { rule, mark: () => missing = true }));
  const body: unknown = await get(path);
  return missing ? "missing" : { body };
}

function isObject(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

const READ = "the subscription read";
const LIST = "the customer's subscription list";
const CANCEL = "the cancel";

/**
 * The stored subscription's Stripe `status`, read live; `null` only when Stripe answered 404
 * `resource_missing` (not the other mode's). Throws the delete's 502 for anything else: any body that
 * is not an object carrying a string `status`, `null` included. The caller has checked the id's
 * shape; it is encoded here regardless.
 */
export function subscriptionStatusFrom(
  secret: string,
  fetchImpl: typeof fetch,
): (subscriptionId: string) => Promise<string | null> {
  return (subscriptionId) =>
    or502(READ, async () => {
      const got = await getOrMissing(
        secret,
        fetchImpl,
        READ,
        `/v1/subscriptions/${encodeURIComponent(subscriptionId)}`,
        SUBSCRIPTION_MISSING,
      );
      if (got === "missing") return null;
      if (!isObject(got.body) || typeof got.body.status !== "string") {
        console.error(`account delete: ${READ} answered something that is not a subscription`);
        throw refused();
      }
      return got.body.status;
    });
}

/** One subscription of the customer's list, as the delete needs it. */
export interface ListedSubscription {
  id: string;
  status: string;
  /** Only an explicit `true` counts; an absent flag is read as not cancelling, so it is set. */
  cancel_at_period_end: boolean;
}

/** The most one list answers; a list that says `has_more` is refused, never paged past. */
const LIST_LIMIT = 100;

/**
 * T1d fix round 1 (review finding 2): the customer's subscriptions at Stripe, in Stripe's default list
 * (which leaves out the ended ones), `limit=100`; `null` only when Stripe answered that it does not
 * know the customer (`CUSTOMER_MISSING`: a 400 or 404 `resource_missing` on `customer`, not the other
 * mode's). The 502 for anything else: a list that

 * is not an object with a `data` array, a `has_more` that is not `false`, or an item that is not an
 * object with a subscription-shaped `id` and a string `status`. A list read in part could hide the one
 * subscription that still charges, so nothing short of the whole, exact shape is trusted. The caller
 * has checked the customer id's shape; it is encoded here regardless.
 */
export function customerSubscriptionsFrom(
  secret: string,
  fetchImpl: typeof fetch,
): (customerId: string) => Promise<ListedSubscription[] | null> {
  return (customerId) =>
    or502(LIST, async () => {
      const got = await getOrMissing(
        secret,
        fetchImpl,
        LIST,
        `/v1/subscriptions?customer=${encodeURIComponent(customerId)}&limit=${LIST_LIMIT}`,
        CUSTOMER_MISSING,
      );
      if (got === "missing") return null;
      const list = got.body;
      if (!isObject(list) || !Array.isArray(list.data) || list.has_more !== false) {
        console.error(`account delete: ${LIST} is not a whole list`);
        throw refused();
      }
      const out: ListedSubscription[] = [];
      for (const item of list.data) {
        if (
          !isObject(item) || typeof item.id !== "string" || !SUBSCRIPTION_ID.test(item.id) ||
          typeof item.status !== "string"
        ) {
          console.error(`account delete: ${LIST} holds an item that is not a subscription`);
          throw refused();
        }
        out.push({
          id: item.id,
          status: item.status,
          cancel_at_period_end: item.cancel_at_period_end === true,
        });
      }
      return out;
    });
}

/** `stripePostFrom` for the delete's updates: every refusal, 404 included, is the 502, and no log line
 * carries the path. */
export function cancelPostFrom(secret: string, fetchImpl: typeof fetch): StripePost {
  const post = stripePostFrom(secret, deleteFetch(fetchImpl, CANCEL, null));
  return (path, form) => or502(CANCEL, () => post(path, form));
}
