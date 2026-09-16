// The OAuth code -> token exchange, and the one place this codebase reads an identity out of
// Google's answer.
//
// It lives in its own file, out of `index.ts`, for one reason: `index.ts` is the wiring Deno
// serves and nothing in it can be driven by a test, so the parsing that decides whether a consent
// becomes a stored grant used to be untestable. C2's final review found exactly the bug that hides
// there — `body.id_token.split(".")` on a field Google had not been asked for — so the parsing is
// here, exported, and driven by a fake `fetch` in `exchange_test.ts`.
//
// **Why `id_token` can be absent.** Google returns an `id_token` only when the consent asked for
// an identity scope (`openid`). C2 asked for exactly one API scope (`calendar.readonly` or
// `gmail.readonly`), so the field was never sent and the dereference above was a TypeError on
// every real consent. `google-connect/handler.ts` now asks for `openid email` alongside the API
// scope, so the normal path has an `id_token` again — and this file still works without one,
// because a student may untick a scope on the consent screen and an incremental consent returns
// only what is new.
//
// Nothing here logs. A token exchange's error body can echo the authorization code and, on some
// failures, the client secret's prefix, so no response body reaches a message this file throws.

/** The token endpoint. Not configurable: it is Google's, and a test injects `fetchImpl` instead. */
export const TOKEN_ENDPOINT = "https://oauth2.googleapis.com/token";
/** OpenID Connect's userinfo endpoint — the fallback identity when there is no `id_token`. */
export const USERINFO_ENDPOINT = "https://www.googleapis.com/oauth2/v3/userinfo";

/** A failure of the exchange itself, named so `callbackHandler`'s class-only log line says
 * something. Every message here is our own words: never a response body, never the code. */
export class ExchangeError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ExchangeError";
  }
}

export interface ExchangeDeps {
  clientId: string;
  clientSecret: string;
  redirectUri: string;
  /** Injected by the test; production passes nothing and gets the real `fetch`. */
  fetchImpl?: typeof fetch;
}

export interface ExchangedGrant {
  refresh_token: string;
  access_token: string;
  sub: string;
  email?: string;
  /** What Google ACTUALLY granted, from its own `scope` field, verbatim. */
  scopes: string[];
}

/**
 * The `sub` and `email` claims out of an ID token's payload, or `null` if it is not a readable
 * JWT. **Not verified cryptographically**: the token came from a TLS connection to Google's own
 * endpoint in response to our own code, which is the same trust the access token beside it rests
 * on. `null` rather than a throw, so an unreadable token falls through to userinfo instead of
 * failing a consent that is otherwise perfectly good.
 */
export function claimsFromIdToken(idToken: unknown): { sub: string; email?: string } | null {
  if (typeof idToken !== "string") return null;
  const parts = idToken.split(".");
  if (parts.length < 2 || parts[1] === "") return null;
  try {
    const json = atob(parts[1].replace(/-/g, "+").replace(/_/g, "/"));
    const claims = JSON.parse(json) as { sub?: unknown; email?: unknown };
    if (typeof claims.sub !== "string" || claims.sub === "") return null;
    return { sub: claims.sub, email: typeof claims.email === "string" ? claims.email : undefined };
  } catch {
    return null;
  }
}

/** The same two values from the userinfo endpoint, with the access token — the path a consent that
 * carried no identity scope (or whose `openid` was unticked) takes. `null` on any failure. */
async function claimsFromUserinfo(
  accessToken: string,
  fetchImpl: typeof fetch,
): Promise<{ sub: string; email?: string } | null> {
  try {
    const response = await fetchImpl(USERINFO_ENDPOINT, {
      headers: { Authorization: `Bearer ${accessToken}` },
    });
    if (!response.ok) return null;
    const body = await response.json() as { sub?: unknown; email?: unknown };
    if (typeof body.sub !== "string" || body.sub === "") return null;
    return { sub: body.sub, email: typeof body.email === "string" ? body.email : undefined };
  } catch {
    return null;
  }
}

/**
 * One authorization code for one grant. Throws [`ExchangeError`] — never a TypeError — when the
 * exchange fails, when it carries no refresh token, or when neither an `id_token` nor userinfo can
 * name who consented.
 */
export async function exchangeCode(code: string, deps: ExchangeDeps): Promise<ExchangedGrant> {
  const fetchImpl = deps.fetchImpl ?? fetch;
  const response = await fetchImpl(TOKEN_ENDPOINT, {
    method: "POST",
    headers: { "Content-Type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams({
      code,
      client_id: deps.clientId,
      client_secret: deps.clientSecret,
      redirect_uri: deps.redirectUri,
      grant_type: "authorization_code",
    }),
  });
  if (!response.ok) throw new ExchangeError(`token exchange ${response.status}`);
  const body = await response.json().catch(() => ({})) as {
    refresh_token?: unknown;
    access_token?: unknown;
    id_token?: unknown;
    scope?: unknown;
  };
  if (typeof body.refresh_token !== "string" || body.refresh_token === "") {
    throw new ExchangeError("no refresh_token in the exchange");
  }
  const accessToken = typeof body.access_token === "string" ? body.access_token : "";
  // The id_token first — it costs no round trip — and userinfo only when there is none to read.
  const identity = claimsFromIdToken(body.id_token) ??
    (accessToken === "" ? null : await claimsFromUserinfo(accessToken, fetchImpl));
  if (identity === null) {
    throw new ExchangeError("the exchange named no Google account (no id_token and no userinfo)");
  }
  // `scope` is space-separated and is what was ACTUALLY granted — a student can untick one on the
  // consent screen, and on an incremental ask Google returns only the new one. Recorded verbatim,
  // `openid`/`email` included: `google_accounts.scopes` is Google's answer, not our request.
  return {
    refresh_token: body.refresh_token,
    access_token: accessToken,
    sub: identity.sub,
    email: identity.email,
    scopes: (typeof body.scope === "string" ? body.scope : "").split(" ").filter((s) => s !== ""),
  };
}
