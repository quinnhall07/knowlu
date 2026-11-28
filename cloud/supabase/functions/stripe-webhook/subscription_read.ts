/**
 * The webhook's one read of a Subscription, strict (v0.4.1 note D6b). `_shared/stripe.ts`'s
 * `stripeGetFrom` answers `null` for every non-2xx; here only Stripe's 404 means "no such
 * subscription". Any other answer, a network error, or a 2xx whose body is not a JSON object with
 * `object: "subscription"` throws a `SubscriptionReadError`, so the handler answers 502 and Stripe redelivers the event: a short outage
 * at the moment a student subscribes must not lose the confirmation or the entitlement write.
 *
 * It still goes through `stripeGetFrom`, so the key and the pinned `Stripe-Version` live in one place;
 * what this file adds is the `fetch` that helper runs over, which decides every non-2xx before the
 * helper sees it (as `account/stripe_delete.ts` does). The error carries a status or "network" only:
 * Stripe's error body can echo an address, and a network error's message names the URL, which names
 * the id.
 */
import { stripeGetFrom } from "../_shared/stripe.ts";

// deno-lint-ignore no-explicit-any
type Json = any;

export class SubscriptionReadError extends Error {
  constructor(readonly what: number | "network") {
    super(`the subscription could not be read (${what})`);
    this.name = "SubscriptionReadError";
  }
}

/** What a failed read is logged as: its HTTP status, or "network". Never a message of any throw. */
export function readFailure(e: unknown): string {
  return e instanceof SubscriptionReadError ? String(e.what) : "network";
}

/** `null` when Stripe answers 404; the Subscription object on a 2xx; a throw for anything else. */
export function subscriptionReader(
  secret: string,
  fetchImpl: typeof fetch,
): (id: string) => Promise<Json | null> {
  return async (id) => {
    // A holder, not a `let`: the status is set inside the `fetch` below.
    const answered: { status: number | null } = { status: null };
    const strict: typeof fetch = async (input, init) => {
      let res: Response;
      try {
        res = await fetchImpl(input, init);
      } catch {
        throw new SubscriptionReadError("network");
      }
      answered.status = res.status;
      if (res.ok) return res;
      // The body is never read: it can echo an address. A 404 reaches the helper bodiless, as `null`.
      await res.body?.cancel();
      if (res.status === 404) return new Response(null, { status: 404 });
      throw new SubscriptionReadError(res.status);
    };
    let body: unknown;
    try {
      body = await stripeGetFrom(secret, strict)(`/v1/subscriptions/${encodeURIComponent(id)}`);
    } catch (e) {
      if (e instanceof SubscriptionReadError) throw e;
      // A 2xx whose body is not JSON: `res.json()` threw.
      throw new SubscriptionReadError(answered.status ?? "network");
    }
    if (answered.status === 404) return null;
    // A 2xx whose JSON is not an object (`null`, an array, a string), or is an object of another kind
    // (`{}`, an invoice), is not a Subscription: Stripe marks every object it answers with `object`.
    if (
      typeof body !== "object" || body === null || Array.isArray(body) ||
      (body as { object?: unknown }).object !== "subscription"
    ) {
      throw new SubscriptionReadError(answered.status ?? "network");
    }
    return body;
  };
}
