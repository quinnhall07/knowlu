/**
 * Stripe, with no SDK: form encoding, HMAC signature verification, and one `POST` helper. Every
 * pure part is exported so the tests can drive it without a key and without a request.
 */
import { fail } from "./http.ts";
import { toHex } from "./crypto.ts";

export function formEncode(params: Record<string, string | number | boolean | undefined | null>): string {
  const p = new URLSearchParams();
  for (const [k, v] of Object.entries(params)) {
    if (v === undefined || v === null) continue;
    p.append(k, String(v));
  }
  return p.toString();
}

export async function hmacHex(secret: string, message: string): Promise<string> {
  const key = await crypto.subtle.importKey(
    "raw",
    new TextEncoder().encode(secret),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"],
  );
  const sig = await crypto.subtle.sign("HMAC", key, new TextEncoder().encode(message));
  return toHex(sig);
}

export function parseStripeSignature(header: string): { t: number; v1: string[] } | null {
  let t: number | null = null;
  const v1: string[] = [];
  for (const part of header.split(",")) {
    const [k, v] = part.split("=", 2);
    if (k === "t") {
      const n = Number(v);
      if (!Number.isFinite(n)) return null;
      t = n;
    } else if (k === "v1" && v) {
      v1.push(v);
    }
  }
  return t !== null && v1.length > 0 ? { t, v1 } : null;
}

/** Constant-time-ish compare (length first, then XOR every character): neither string here is a
 * secret, but the habit is, and `billing-jobs/handler.ts`'s job-token check (R-C1-59 M9) reuses it
 * for exactly that reason — the same shape works for any same-length string, hex or not. */
export function equalHex(a: string, b: string): boolean {
  if (a.length !== b.length) return false;
  let diff = 0;
  for (let i = 0; i < a.length; i++) diff |= a.charCodeAt(i) ^ b.charCodeAt(i);
  return diff === 0;
}

/**
 * Stripe's scheme: HMAC-SHA256 over `<t>.<raw body>`, compared against every `v1` in the header,
 * inside a five-minute window. The window is what stops a captured webhook being replayed forever.
 */
export async function verifyStripeSignature(
  payload: string,
  header: string,
  secret: string,
  nowSeconds: number,
  toleranceSeconds = 300,
): Promise<boolean> {
  const parsed = parseStripeSignature(header);
  if (!parsed) return false;
  if (Math.abs(nowSeconds - parsed.t) > toleranceSeconds) return false;
  const expected = await hmacHex(secret, `${parsed.t}.${payload}`);
  return parsed.v1.some((got) => equalHex(got, expected));
}

export type StripePost = (path: string, form: Record<string, string>) => Promise<Record<string, unknown>>;

/**
 * The API version this code is written against, pinned. Stripe moves fields between objects at a
 * version boundary — `current_period_end` left the Subscription for `items.data[].current_period_end`
 * in `2025-03-31.basil` — and an account created today defaults well past that. Pinning means the
 * shape the handlers parse is the shape that arrives, and changing it is a deliberate edit with a
 * test beside it rather than a silent `null` in every entitlement row.
 */
export const STRIPE_API_VERSION = "2025-03-31.basil";

export function stripePostFrom(secret: string, fetchImpl: typeof fetch): StripePost {
  return async (path, form) => {
    const res = await fetchImpl(`https://api.stripe.com${path}`, {
      method: "POST",
      headers: {
        authorization: `Bearer ${secret}`,
        "content-type": "application/x-www-form-urlencoded",
        "stripe-version": STRIPE_API_VERSION,
      },
      body: formEncode(form),
    });
    const body = await res.json() as Record<string, unknown>;
    if (!res.ok) {
      // Stripe's error body can echo a customer email; it is logged, never returned.
      console.error(`stripe ${path}: ${res.status} ${JSON.stringify(body)}`);
      throw fail(502, "the payment provider refused the request");
    }
    return body;
  };
}

export type StripeGet = (path: string) => Promise<Record<string, unknown> | null>;

/**
 * The one `GET` this stream makes: the Subscription the webhook re-reads when an event carries an id
 * and nothing else. **Versioned exactly like the `POST`s** — this body is what
 * `entitlementFromSubscription` parses, and an unversioned read arrives under the account's own
 * default (well past `2025-03-31.basil`), where `current_period_end` is on neither the Subscription
 * nor the item the handler looks at, so every entitlement row would carry `null`.
 *
 * `null` on any non-2xx: a subscription we cannot read is an event to ignore, not a 500 — Stripe
 * retries a 500 and would replay it for days.
 */
export function stripeGetFrom(secret: string, fetchImpl: typeof fetch): StripeGet {
  return async (path) => {
    const res = await fetchImpl(`https://api.stripe.com${path}`, {
      headers: {
        authorization: `Bearer ${secret}`,
        "stripe-version": STRIPE_API_VERSION,
      },
    });
    return res.ok ? await res.json() as Record<string, unknown> : null;
  };
}
